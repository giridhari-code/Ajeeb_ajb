# M2.5 Package Signing — Implementation Plan

## Audit Summary

### Current State (What Already Exists)
Both `ajeebc/crates/parth/` and `ajeebBootstrap/crates/parth/` have:

| Component | Status | Location |
|-----------|--------|----------|
| Ed25519 key generation | ✅ Done | `crypto.rs:7-32`, `keys.rs:1-64` |
| Package signing | ✅ Done | `crypto.rs:71-109`, `signing.rs:41-77` |
| Signature verification | ✅ Done | `crypto.rs:144-185`, `signing.rs:79-118` |
| Signature storage | ✅ Done | `~/.parth/signatures/<pkg>/<ver>.sig` |
| CLI: `parth sign` | ✅ Done | `commands/registry.rs:163-189` |
| CLI: `parth verify` | ✅ Done | `commands/registry.rs:191-201` |
| CLI: `parth keygen` | ✅ Done | `commands/registry.rs:241-249` |
| Security audit scan | ✅ Done | `security.rs:77-108` |

### Gaps Identified

| Gap | Severity | Description |
|-----|----------|-------------|
| Publish doesn't require signing | HIGH | `cmd_publish` calls `sign_package()` but only prints warning on failure — package publishes unsigned |
| Install doesn't verify signatures | HIGH | `cmd_install` / `download_package` never calls `verify_signature()` |
| Lockfile has no signature info | MEDIUM | `LockEntry` stores `checksum` but not signature hash or signer — can't verify lock integrity |
| Remote download doesn't verify SHA-256 | MEDIUM | `download_from_remote()` computes hash but never compares against expected from index |
| Resolver doesn't verify signatures | MEDIUM | `ensure_package()` checks checksums only, not signatures |
| No signature policy config | LOW | No way to configure "require signatures" vs "warn" vs "ignore" |
| Registry index has no signature field | LOW | Index stores `version = "checksum"` but no signature reference |

## Design Decisions

### D1: Signature Metadata Format

**Lockfile extension** — Add optional `signature` and `signer` fields to `LockEntry`:
```
[ajeeb-json]
version = "1.0.0"
checksum = "a1b2c3d4..."
signature = "deadbeef..."      # NEW: hex Ed25519 signature
signer = "cafebabe..."         # NEW: hex Ed25519 public key
```

**Why:** Keeps backward compatibility (old lockfiles without these fields still parse). The signature signs the checksum hash, so verifying = recompute checksum + verify Ed25519 signature against embedded public key.

**Registry index extension** — No format change needed. The signature file is stored separately at `~/.parth/signatures/<pkg>/<ver>.sig` and is already part of the publish flow.

### D2: Signing Policy

Three modes, configurable via `parth.das` `[security]` section or CLI flag:
- **`require`** — Fail install/publish if package is unsigned. Default for `parth publish`.
- **`warn`** — Print warning for unsigned packages but continue. Default for `parth install`.
- **`ignore`** — Skip signature checks entirely.

Backward compatibility: If no `[security]` section exists, default to `warn` for install, `require` for publish.

### D3: Verification Points

| Operation | Check | On Failure |
|-----------|-------|------------|
| `parth publish` | Sign package, include signature in metadata | Fail with error |
| `parth install` | Verify signature if present; warn if missing | Warn (or fail if `require` mode) |
| `parth build` (resolver) | Verify signatures for all locked packages | Warn per package |
| `parth audit` | Already checks signatures (keep as-is) | Report as security issue |
| `parth verify` | Explicit verification command (already exists) | Fail with error |

### D4: Backward Compatibility

- Old lockfiles (no `signature`/`signer` fields) parse fine — fields default to empty string
- Old packages without signatures are treated as "unsigned" — warned but not blocked in `warn` mode
- New lockfiles written with signature fields are readable by old Parth versions (they ignore unknown keys)

## Implementation Steps

### Step 1: Extend `LockEntry` type
- Add `signature: String` and `signer: String` fields to `LockEntry` in `types.rs`
- Update `read_lock()` in `resolver.rs` to parse optional `signature` and `signer` keys
- Update `write_lock()` to emit `signature` and `signer` when non-empty

### Step 2: Make publish signing mandatory
- In `cmd_publish`: if `sign_package()` fails, abort with error (not just warning)
- Store signature info in the lock entry when writing after resolve

### Step 3: Add signature verification to install flow
- After `download_package()` succeeds, call `verify_signature()` if signature file exists
- If no signature file exists, print warning (configurable via policy)
- If signature verification fails, abort with error

### Step 4: Add signature verification to resolver
- In `ensure_package()`, after checksum verification, also verify signature if `.sig` file exists
- Store signature + signer in the `LockEntry` during `make_lock_entry_with_registry()`

### Step 5: Add signature info to remote publish
- `publish_to_remote()` already includes signature in JSON metadata (line 218)
- Ensure the signature is included in the tarball or metadata

### Step 6: Add signing test suite
- Test: sign a package, verify it succeeds
- Test: tamper with package content, verify signature fails
- Test: sign with wrong key, verify fails
- Test: missing signature file produces correct error
- Test: lockfile roundtrip with signature fields
- Test: publish flow requires signing
- Test: install flow verifies signatures
- Test: backward compatibility with old lockfiles

### Step 7: Verify existing tests pass
- Run `cargo test` in both `ajeebc/crates/parth/` and `ajeebBootstrap/crates/parth/`
- Run `bash tests/bootstrap_check.sh`

## Files to Modify

| File | Change |
|------|--------|
| `ajeebc/crates/parth/src/types.rs` | Add `signature`, `signer` to `LockEntry` |
| `ajeebc/crates/parth/src/resolver.rs` | Parse/write `signature`/`signer` in lockfile; populate during resolve |
| `ajeebc/crates/parth/src/commands/registry.rs` | Make publish signing mandatory; add verification to install |
| `ajeebc/crates/parth/src/registry/mod.rs` | Add `ensure_package_with_signature()` helper |
| `ajeebc/crates/parth/src/registry/crypto.rs` | Add `verify_signature_file()` that returns structured result |
| `ajeebBootstrap/crates/parth/src/types.rs` | Mirror LockEntry changes |
| `ajeebBootstrap/crates/parth/src/resolver/mod.rs` | Mirror lockfile changes |
| `ajeebBootstrap/crates/parth/src/registry/tests.rs` | Add signing test suite |

## Verification Criteria

- [ ] All 16 cargo tests pass
- [ ] All 15 Parth suites pass  
- [ ] New signing tests pass (>= 6 new tests)
- [ ] `parth publish` fails if signing fails
- [ ] `parth install` warns on unsigned packages
- [ ] `parth verify` correctly detects tampered packages
- [ ] Old lockfiles without signature fields still parse
- [ ] Self-hosting bootstrap still works
