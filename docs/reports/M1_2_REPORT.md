# M1.2 Report — SemVer Parsing + Constraint Matching

## Status: COMPLETE

## Files Changed

| File | Action | LOC Added | Purpose |
|------|--------|-----------|---------|
| `parth/src/resolver.ajb` | Modified | +131 | `parseSemver`, `compareVersions`, `parseConstraint`, `matchesConstraint` |
| `parth/src/main.ajb` | Modified | +125 | `testSemver()` — 16 test cases, `test-semver` CLI command |
| `parth/parth_m1.ajb` | Rebuilt | 1261 total | Single-file concatenation |

**Total new Ajeeb LOC:** 256

## Buffer Layout

| Slots | Purpose |
|-------|---------|
| 500-502 | `parseSemver` output (major, minor, patch) — scratch space |
| 510-522 | `compareVersions` working copies |
| 600-603 | `parseConstraint` output (type, major, minor, patch) |

## Functions Implemented

### `parseSemver(s: string): int`
Parses `"MAJOR.MINOR.PATCH"` into slots 500-502. Stops at `-` or `+` (ignores pre-release/build metadata). Returns 0 on success, 1 on error.

### `compareVersions(aOff, bOff): int`
Compares versions stored at buffer offsets. Returns -1 (a<b), 0 (equal), 1 (a>b). Compares major → minor → patch.

### `parseConstraint(s: string): int`
Parses constraint string into slots 600-603. Supports:
- `*` or empty → type 0 (Any)
- `1.2.3` → type 1 (Exact)
- `^1.2.3` → type 2 (Caret)
- `~1.2.3` → type 3 (Tilde)
- `>=1.2.3` → type 4 (Gte)

### `matchesConstraint(verOff, cOff): int`
Checks if version satisfies constraint. Returns 1 (match), 0 (no match).

**Caret semantics (`^`):**
- `^1.2.3` → major must match, version >= 1.2.3, version < 2.0.0
- `^0.2.3` → major AND minor must match (0.x minor lock)
- `^0.0.3` → major AND minor AND patch must match (0.0.x patch lock)

**Tilde semantics (`~`):**
- `~1.2.3` → major AND minor must match, version >= 1.2.3, version < 1.3.0

**Gte semantics (`>=`):**
- `>=1.2.3` → version >= 1.2.3

## Test Matrix (16/16 pass)

| # | Test | Expected | Actual |
|---|------|----------|--------|
| 1 | `parseSemver("1.2.3")` | 1.2.3 | ✓ |
| 2 | `parseSemver("0.1.0")` | 0.1.0 | ✓ |
| 3 | `compareVersions(1.2.3, 1.2.3)` | 0 | ✓ |
| 4 | `compareVersions(1.2.4, 1.2.3)` | 1 | ✓ |
| 5 | `compareVersions(1.2.3, 1.2.4)` | -1 | ✓ |
| 6 | `exact 1.2.3 matches 1.2.3` | true | ✓ |
| 7 | `exact 1.2.3 rejects 1.2.4` | false | ✓ |
| 8 | `^1.2.3 matches 1.5.0` | true | ✓ |
| 9 | `^1.2.3 rejects 2.0.0` | false | ✓ |
| 10 | `^0.2.3 rejects 0.3.0` | false | ✓ |
| 11 | `~1.2.3 matches 1.2.9` | true | ✓ |
| 12 | `~1.2.3 rejects 1.3.0` | false | ✓ |
| 13 | `>=1.2.3 matches 1.2.3` | true | ✓ |
| 14 | `>=1.2.3 matches 2.0.0` | true | ✓ |
| 15 | `>=1.2.3 rejects 1.2.2` | false | ✓ |
| 16 | `* matches 9.9.9` | true | ✓ |

## Regression Tests

| Test | Result |
|------|--------|
| `parth new` | ✓ |
| `parth add pkg@^1.0.0` | ✓ |
| `parth add pkg` (duplicate) | ✓ |
| `parth remove pkg` | ✓ |
| `parth remove fake` (not found) | ✓ |
| `parth build` (no deps) | ✓ |
| `parth run` | ✓ |
| `parth help` | ✓ |
| cargo test 16/16 | ✓ |

## Known Limitations

- Pre-release versions (`1.0.0-alpha`) not parsed (ignored, patch position used)
- No compound constraints (`>=1.0.0 <2.0.0` or `^1.0.0 || ^2.0.0`)
- No `<`, `<=`, `>` operators (only `>=` supported)
- No `!=` operator
- Scratch space at slots 500-522 may conflict if called recursively

## Readiness for M1.3

M1.2 provides the building blocks for M1.3:
- `parseSemver` + `compareVersions` → version comparison for resolution
- `parseConstraint` + `matchesConstraint` → constraint checking for transitive deps
- Buffer slots 600-603 available for constraint storage during resolution

M1.3 scope: transitive resolution (BFS), conflict detection, dependency tree display.
