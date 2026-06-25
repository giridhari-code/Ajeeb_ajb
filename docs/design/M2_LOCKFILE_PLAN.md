# M2.1 Lockfile Support — Implementation Plan

## Current State

### Ajeeb self-hosted (`parth/src/`)
- `resolver.ajb:687-734` — `generateLockfile()` exists but is minimal:
  - Only writes direct dependencies (not transitive)
  - Uses `[[package]]` TOML array-of-tables format
  - No checksums, no registry info, no dependency graph
  - Never read back during resolution or build
- `builder.ajb:52-141` — `buildProject()` calls `resolveAll()` which always re-resolves from scratch
- No lockfile-aware resolution: every `parth build` re-resolves everything

### Rust reference (`ajeebc/crates/parth/src/resolver.rs`)
- Full v2 lockfile with checksums, transitive deps, registry URLs
- Read during resolution to pin existing choices
- Written after successful resolution
- Topological sort from lockfile for compilation order

## Design: parth.lock v2 Format

```
# parth-lock-v2

[package-name]
version = "1.2.3"
checksum = "sha256hex..."
registry = ""
dependencies = "dep1@^1.0.0, dep2@>=2.0.0"
```

### Key properties
1. **Human-readable** — plain text, comment-friendly
2. **Deterministic** — packages sorted alphabetically
3. **Complete** — stores resolved transitive deps + checksums
4. **Compatible** — same format as Rust version

## Implementation Steps

### Step 1: Lockfile read/write helpers (`resolver.ajb`)

Add functions:
- `readLockfile(path): int` — parse parth.lock into CSV strings (lockNames, lockVersions, lockChecksums, lockDeps)
- `writeLockfile(path): int` — serialize lock state to parth.lock
- `getLockEntry(name): int` — lookup a package in lock data
- `lockfileEntryCount(): int` — number of entries

Storage: Use buffer slots (similar to parser.ajb) for lock entries:
- Slot 700+idx*5: nameOff, nameLen, verOff, verLen, depsOff
- Slot 750: lockEntryCount

### Step 2: Lockfile-aware resolution (`resolver.ajb`)

Modify `resolveAll()`:
1. Read existing parth.lock at start
2. For each dependency, check lockfile first
3. If lockfile has an exact version that matches the constraint, use it
4. If not in lockfile, resolve normally and record the choice
5. After resolution, write updated parth.lock

### Step 3: Lockfile in build pipeline (`builder.ajb`)

Modify `buildProject()`:
1. After `resolveAll()`, if lockfile was updated, print message
2. Lockfile serves as audit trail of what was resolved

### Step 4: CLI commands (`main.ajb`)

Add commands:
- `parth generate-lockfile` — already exists, enhance to use new format
- `parth install` — when lockfile exists, skip resolution and use pinned versions

### Step 5: Tests

Add test suite in `main.ajb`:
- Lockfile write/read roundtrip
- Lockfile pins exact versions on re-resolve
- Lockfile entries cleaned when deps change
- Lockfile survives `parth add`/`parth remove`

## Buffer Slot Layout (extended)

| Slots | Purpose |
|-------|---------|
| 0-14 | Config parser (existing) |
| 200+i*4 | Dep entries (existing) |
| 500-502 | SemVer parse (existing) |
| 600-603 | Constraint parse (existing) |
| 700+idx*5 | Lock entries: nameOff, nameLen, verOff, verLen, depsOff |
| 750 | lockEntryCount |
| 760+idx*4 | Lock checksum entries: chkOff, chkLen, regOff, regLen |
| 800 | lockChecksumCount |

## Files Modified

| File | Changes |
|------|---------|
| `parth/src/resolver.ajb` | Add readLockfile, writeLockfile, getLockEntry; modify resolveAll |
| `parth/src/main.ajb` | Add testLockfile suite, update generate-lockfile command |

## Verification

1. `cargo test` — Rust tests still pass
2. `bash tests/bootstrap_check.sh` — self-hosting still works
3. Manual test: create project → add deps → build → verify parth.lock exists → rebuild (should be faster)
4. Test suite: lockfile roundtrip, pinning, stale cleanup
