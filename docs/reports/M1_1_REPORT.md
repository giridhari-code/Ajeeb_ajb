# M1.1 Report — CLI Wiring for add/remove

## Status: COMPLETE

## Files Changed

| File | Action | LOC Added | Purpose |
|------|--------|-----------|---------|
| `parth/src/resolver.ajb` | Modified | +33 | `parseSpec()` function, fixed `addDep()` section-aware insertion |
| `parth/src/main.ajb` | Modified | +21 | CLI dispatch for `add` and `remove`, updated help string |
| `parth/parth_m1.ajb` | Rebuilt | 865 total | Single-file concatenation of all modules |

**Total new Ajeeb LOC:** 54

## What Was Implemented

### 1. `parseSpec(spec)` — resolver.ajb:131-143 (13 LOC)
Splits `"name@version"` on first `@` character. Defaults version to `"*"` if no `@` present.

### 2. `addDep()` fix — resolver.ajb:148-195
**Bug fixed:** Original `addDep` appended new dep line at END of file, after `[compiler]` section. Parser only reads deps in `[dependencies]` section, so re-added deps were invisible.

**Fix:** Scan for `[dependencies]` header, find next `[` section header, insert new line before it.

### 3. CLI dispatch — main.ajb:140-163
- `add` command: reads spec, calls `parseSpec`, splits name/version, calls `addDep`
- `remove` command: reads name, calls `removeDep`
- Usage validation for missing arguments

### 4. Help string — main.ajb:98-110
Added `add` and `remove` to usage line and command list.

## Test Results

| # | Test | Expected | Actual | Pass |
|---|------|----------|--------|------|
| 1 | `parth add foo` | `foo = "*"` in parth.das | ✓ | ✓ |
| 2 | `parth add bar@^1.2.3` | `bar = "^1.2.3"` in parth.das | ✓ | ✓ |
| 3 | `parth add foo` (duplicate) | "pehle se hai!" message | ✓ | ✓ |
| 4 | `parth remove foo` | Line removed from parth.das | ✓ | ✓ |
| 5 | `parth remove fake` | "nahi mili" error message | ✓ | ✓ |
| 6 | `parth new` | Creates project dir | ✓ | ✓ |
| 7 | `parth build` (no deps) | Compiles to native binary | ✓ | ✓ |
| 8 | `parth run` (no deps) | Builds and executes | ✓ | ✓ |
| 9 | `parth help` | Shows add/remove in help | ✓ | ✓ |
| 10 | cargo test | 16/16 pass | ✓ | ✓ |

## M0 Regression

All M0 commands verified working:
- `parth new` ✓
- `parth init` ✓
- `parth build` ✓
- `parth run` ✓
- `parth help` ✓

## Build Process

```
1. Concatenate parth/src/{parser,resolver,builder,runner,main}.ajb → parth/parth_m1.ajb
2. Self-hosted compiler: ajeebc parth/parth_m1.ajb → build/output.c
3. Fix C output (exec/mkdir decls, argc rename)
4. gcc: output.c + runtime → build/parth_m1
```

## Known Limitations (expected, not in M1.1 scope)

- Version string stored as-is, no semver parsing or validation
- No transitive dependency resolution
- No dependency tree display
- `addDep` inserts before next `[` character — works for standard parth.das format but could break on unusual formatting

## Remaining Work for M1.2

M1.2: Version Matching — implement:
- `parseSemver(s)` — parse `"1.2.3-alpha"` into major/minor/patch
- `cmpVersion(a, b)` — compare versions (-1/0/1)
- `parseConstraint(s)` — parse `"^1.0.0"` into type + version
- `matchConstraint(ver, constraint)` — check if version satisfies constraint
- `bestVersion(versions, constraint)` — find best matching version

Estimated: ~170 LOC, ~2h effort.
