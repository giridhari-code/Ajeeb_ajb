# PARTH M1 PLAN — Package Management Parity

## Scope

Implement core package-management parity: add, remove, dependency tree, version matching, manifest updates.

## Constraints

- No registry, signing, cache, workspace, lockfile, Stage C, compiler changes
- Single-file concatenation approach (same as M0)
- All new code in `parth/src/` modules, concatenated into `parth_m1.ajb`
- Must pass `cargo test` and `bash tests/bootstrap_check.sh`

---

## M1.1: add/remove CLI Wiring + Version Validation

**Goal:** Wire existing `addDep`/`removeDep` into CLI dispatch. Add `@`-parsing. Add version validation at add time.

**Files modified:**
- `parth/src/main.ajb` — add CLI cases for `add` and `remove`
- `parth/src/resolver.ajb` — improve `addDep` with `@`-parsing and basic validation

**Implementation order:**

| Step | What | LOC | Depends On |
|------|------|-----|-----------|
| 1.1a | `parseSpec(spec)` — split `"name@version"` into name+version, default `"*"` | ~15 | Nothing |
| 1.1b | Wire `add` in `main()` — call `parseSpec`, then `addDep` | ~8 | 1.1a |
| 1.1c | Wire `remove` in `main()` — call `removeDep` | ~5 | Nothing |
| 1.1d | Update help string to mention add/remove | ~3 | Nothing |

**Success criteria:**
- `parth add pkg@^1.0.0` creates correct line in parth.das
- `parth add pkg` defaults to `"*"`
- `parth add pkg` twice prints "pehle se hai!"
- `parth remove pkg` removes the line
- `parth help` shows add/remove

**Effort:** ~30 min

---

## M1.2: Version Matching

**Goal:** Implement semver parsing, comparison, and constraint matching.

**Files modified:**
- `parth/src/resolver.ajb` — add version matching functions (before resolveAll)

**Implementation order:**

| Step | What | LOC | Depends On |
|------|------|-----|-----------|
| 1.2a | `parseSemver(s)` — parse `"1.2.3-alpha+build"` into 3 slots (major,minor,patch) + pre-release string | ~40 | Nothing |
| 1.2b | `cmpVersion(a, b)` — compare two version slots, return -1/0/1. Pre-release < release. | ~25 | 1.2a |
| 1.2c | `parseConstraint(s)` — parse `"^1.0.0"` into type enum + version slots. Handle: `*`, `=`, `^`, `~`, `>=`, `<=`, `>`, `<` | ~50 | 1.2a |
| 1.2d | `matchConstraint(ver, constraint)` — check if version satisfies constraint. Caret: major lock (minor lock if 0.x). Tilde: major+minor lock. | ~40 | 1.2b, 1.2c |
| 1.2e | `bestVersion(versions, constraint)` — iterate sorted versions (newest first), return first match | ~15 | 1.2d |

**Data layout (buffer slots):**
- Version: 3 contiguous slots for major/minor/patch (integers)
- Constraint: 1 slot for type (0=Any,1=Exact,2=Caret,3=Tilde,4=Gte,5=Gt,6=Lte,7=Lt) + 3 slots for version
- Compound constraints (AND/OR): not in M1 scope — simplified to single constraints only

**Success criteria:**
- `parseSemver("1.2.3")` returns major=1,minor=2,patch=3
- `parseSemver("1.0.0-alpha")` returns pre-release="alpha"
- `cmpVersion("1.0.0", "2.0.0")` returns -1
- `cmpVersion("1.0.0-alpha", "1.0.0")` returns -1 (pre < release)
- `matchConstraint("1.5.0", "^1.2.3")` returns 1 (matches)
- `matchConstraint("2.0.0", "^1.2.3")` returns 0 (does not match)
- `matchConstraint("0.3.0", "^0.2.3")` returns 0 (0.x minor lock)
- `matchConstraint("1.2.5", "~1.2.3")` returns 1 (matches)
- `matchConstraint("1.3.0", "~1.2.3")` returns 0 (does not match)

**Effort:** ~2h

---

## M1.3: Transitive Resolution + Dependency Tree

**Goal:** Resolve transitive dependencies recursively. Detect conflicts. Display dependency tree.

**Files modified:**
- `parth/src/resolver.ajb` — replace linear `resolveAll` with BFS resolver, add tree display

**Implementation order:**

| Step | What | LOC | Depends On |
|------|------|-----|-----------|
| 1.3a | `readPkgDeps(name)` — read a package's parth.das, return its deps (name+constraint pairs) | ~20 | parser.ajb |
| 1.3b | `resolveAllBFS(deps)` — BFS queue resolver: for each dep, find best matching version, read transitive deps, enqueue if new, detect conflicts | ~80 | 1.2e, 1.3a |
| 1.3c | `detectConflict(name, existing_constraint, new_constraint)` — check if two constraints are satisfiable simultaneously | ~20 | 1.2c |
| 1.3d | `printTree(resolved_deps)` — recursive tree display with box-drawing chars and cycle detection | ~40 | 1.3b |
| 1.3e | Wire `tree` command in `main()` | ~5 | 1.3d |
| 1.3f | Wire `resolve` before `build` — call `resolveAllBFS` then update parth.das with resolved versions | ~15 | 1.3b |

**Data layout:**
- Resolved package: name (string), version (3 slots), constraint (4 slots), deps count
- BFS queue: reuse existing buffer slot pattern (or use array slots 1000+)
- Visited set: string comparison against resolved list

**Success criteria:**
- `parth resolve` resolves A→B→C transitively
- `parth tree` shows:
  ```
  my-project@0.1.0
  ├── pkg-a@^1.0.0
  │   └── pkg-b@~2.3.0
  └── pkg-c@>=1.5.0
  ```
- Conflict detection: if A requires `^1.0.0` and B requires `^2.0.0`, print error
- Cycle detection: if A→B→A, print error

**Effort:** ~3h

---

## M1.4: Integration Tests + Polish

**Goal:** Verify all M1 features end-to-end. Fix edge cases.

**Test cases:**

| # | Test | Expected |
|---|------|----------|
| 1 | `parth add pkg@^1.0.0` then `cat parth.das` | Line `pkg = "^1.0.0"` present |
| 2 | `parth add pkg` then `cat parth.das` | Line `pkg = "*"` present |
| 3 | `parth add pkg` twice | "pehle se hai!" message |
| 4 | `parth remove pkg` then `cat parth.das` | Line removed |
| 5 | `parth remove nonexistent` | Error message |
| 6 | Parse `"1.2.3"` → major=1,minor=2,patch=3 | Correct |
| 7 | Parse `"0.1.0-alpha.1"` → pre="alpha.1" | Correct |
| 8 | `^1.2.3` matches `1.5.0` | true |
| 9 | `^1.2.3` matches `2.0.0` | false |
| 10 | `^0.2.3` matches `0.3.0` | false |
| 11 | `~1.2.3` matches `1.2.5` | true |
| 12 | `~1.2.3` matches `1.3.0` | false |
| 13 | `>=1.0.0 <2.0.0` matches `1.5.0` | true |
| 14 | `parth tree` with transitive deps | Correct tree output |
| 15 | Circular dependency detection | Error message |
| 16 | Version conflict detection | Error message |
| 17 | `parth build` after `parth resolve` | Successful compilation |

**Effort:** ~1h

---

## Implementation Order (Exact)

```
M1.1  CLI wiring + @-parsing          (~30 min)
  ↓
M1.2  Version matching                 (~2h)
  ↓
M1.3  Transitive resolution + tree     (~3h)
  ↓
M1.4  Integration tests + polish       (~1h)
```

**Total estimated effort:** ~6.5h

---

## File Changes Summary

| File | M1.1 | M1.2 | M1.3 | M1.4 | Total |
|------|------|------|------|------|-------|
| `parth/src/main.ajb` | +16 | — | +5 | — | +21 |
| `parth/src/resolver.ajb` | +15 | +170 | +160 | — | +345 |
| `parth/parth_m1.ajb` | reconcat | reconcat | reconcat | reconcat | — |
| **Total Ajeeb LOC added** | +31 | +170 | +165 | 0 | **+366** |

**Final M1 size:** ~820 (M0) + 366 = ~1186 LOC

---

## Buffer Slot Layout (M1 additions)

| Slots | Purpose |
|-------|---------|
| 500-599 | Semver parse output (3 slots × N versions) |
| 600-699 | Constraint parse output (4 slots × N constraints) |
| 700-799 | Resolved packages (name_off, major, minor, patch × N) |
| 800-899 | BFS queue state |
| 900-999 | Tree display working space |
