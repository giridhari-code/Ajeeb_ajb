# M1.4 Report — Integration Tests & Production Hardening

## Summary

**Status:** COMPLETE — All 11 test suites pass (72/72 tests).
**Self-hosted:** Cargo tests pass (24/24).
**Binary:** 86KB, 2,324 lines Ajeeb source.

## Test Matrix

| # | Suite | Tests | Pass | Fail | Description |
|---|-------|-------|------|------|-------------|
| 1 | Regression | 10 | 10 | 0 | strcmp_ajeeb semantics, csvContains, buffer aliasing, visited-set |
| 2 | SemVer | 16 | 16 | 0 | parseSemver, compareVersions, parseConstraint, matchesConstraint |
| 3 | Dependency Tree | 6 | 6 | 0 | Linear chain, tree display, shared deps, cycle detection, conflict |
| 4 | CSV Helpers | 9 | 9 | 0 | csvLen, csvGet, csvContains |
| 5 | Config Parser | 11 | 11 | 0 | Full config parse, package/deps/compiler fields, parseSpec |
| 6 | E2E: new | 4 | 4 | 0 | Create project, verify files, verify config content |
| 7 | E2E: init | 3 | 3 | 0 | Init in existing dir, verify files |
| 8 | E2E: add/remove | 6 | 6 | 0 | Add dep, verify in file, add second, remove, verify removal, error on missing |
| 9 | E2E: build | 2 | 2 | 0 | Build project, verify binary created |
| 10 | E2E: run | 1 | 1 | 0 | Build + execute project |
| 11 | E2E: tree | 4 | 4 | 0 | Tree display, resolveAll, cycle in tree, conflict detection |
| **Total** | | **72** | **72** | **0** | |

## Regression Tests — Specific Bugs Targeted

| ID | Bug | Test | Result |
|----|-----|------|--------|
| R1 | `strcmp_ajeeb` returns 0 for equal | R1a-c: 3 equality/inequality checks | ✓ |
| R2 | `csvContains` used wrong comparator | R2a-d: 4 find/reject/edge tests | ✓ |
| R3 | Buffer aliasing corrupts string vars | R3a: Store string, call parseDasConfig, verify string intact | ✓ |
| R4 | Visited-set false conflicts | R4: resolveAll with single dep, no false conflicts | ✓ |

## Self-Hosted Build Verification

```
cargo test: 24/24 passed
```

| Crate | Tests | Status |
|-------|-------|--------|
| ajeeb_compiler | 4 | ✓ |
| ajeeb_fmt | 4 | ✓ |
| ajeeb_lsp | 0 | N/A |
| parth | 16 | ✓ |
| stage2 | 0 | N/A |
| stage3 | 0 | N/A |
| Total | 24 | ✓ |

## Known Limitations

1. **Conflict detection is exact-vs-exact only.** Caret/tilde/gte range overlap is not checked. Two deps requiring `bar@^1.0.0` and `bar@^2.0.0` with no exact conflict will silently resolve to whichever is encountered first in BFS order.

2. **No automatic conflict resolution.** The resolver detects conflicts and reports them but does not attempt resolution (e.g., choosing the highest compatible version).

3. **No lockfile generation from resolveAll.** `generate-lockfile` exists but only lists direct deps, not transitive resolved versions.

4. **`remove` finds dep by string prefix matching.** A dep named `foo-bar` could be incorrectly matched by `remove foo` if `foo` happens to be a prefix. The current implementation checks `prefix == name && nextChar is space or =`, which is correct for well-formed configs but fragile.

5. **No `--json` or machine-readable output.** All output is human-readable Hinglish text.

6. **E2E tests use hardcoded `/tmp/` paths.** Not portable across users but acceptable for CI-like testing.

## Stage B Readiness

| Milestone | Status | Evidence |
|-----------|--------|----------|
| M0: Bootstrap (init, new, build, run) | ✓ COMPLETE | E2E tests 6,7,9,10 pass |
| M1.1: CLI wiring (add, remove) | ✓ COMPLETE | E2E test 8 passes |
| M1.2: SemVer + constraint matching | ✓ COMPLETE | Suite 2: 16/16 |
| M1.3: BFS resolution + tree display | ✓ COMPLETE | Suites 3,11 pass |
| M1.4: Integration tests + hardening | ✓ COMPLETE | All 72 tests pass |
| M2: Extended features | NOT STARTED | — |
| M3: Polish | NOT STARTED | — |

**Stage B M0–M1 is COMPLETE.** The Parth rewrite in Ajeeb has working:
- Project scaffolding (init, new)
- Build pipeline (compile → gcc → binary)
- Run pipeline (build + execute)
- Dependency management (add, remove)
- SemVer parsing with 5 constraint types
- BFS transitive dependency resolution
- Dependency tree display with cycle detection
- Version conflict detection (exact-vs-exact)
- Comprehensive test suite (72 tests, 11 suites)
