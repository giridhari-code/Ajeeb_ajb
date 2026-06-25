# M1.3 Fix Report — BFS Resolution + Tree Display

## Two Bugs Fixed

### Bug 1: Buffer Corruption in `resolveAll()`
**Root cause:** `parseDasConfig()` calls `getStateBuf()` which overwrites buffer slots 0-200+. Previous `resolveAll` stored queue data at slots 900-999 and 950+ using character-by-character `bw(buf, si+k, charCode(str, k))`. Each `parseDasConfig` call for a transitive dependency destroyed this data.

**Fix:** Replaced buffer-slot storage with comma-separated Ajeeb string variables:
- `allNames` / `allVers` — queue contents
- `visitedNames` / `visitedConstraints` — visited set

Added helper functions: `csvGet(csv, idx)`, `csvLen(csv)`, `csvContains(csv, item)`.

**Before:** Test 5 output had garbage names (`b▒▒▒▒`, `b▒▒▒▒`).
**After:** Clean output (`bar @ 1.0.0`, `bar @ 2.0.0`).

### Bug 2: `strcmp_ajeeb` Return Value Convention
**Root cause:** AGENTS.md claimed `strcmp_ajeeb returns 1 for equal, 0 for not equal (opposite of C convention)`. But the C implementation returns raw `strcmp()` — **0 for equal, non-zero for not equal**.

**Affected code:**
- `csvContains()` line 159: `== 1` → should be `== 0`
- `resolveAll()` line 207: `== 1` → should be `== 0`

**Fix:** Changed both to `== 0`.

**Before:** `csvContains("foo,baz", "foo")` returned 0 (not found). Visited set never matched. All deps processed as new. No conflicts detected.
**After:** `csvContains("foo,baz", "foo")` returns 1 (found). Conflicts properly detected.

### Bug 3: Indirect Cycle Detection (A→B→C→A)
**Root cause:** `printTree()` only checked `strcmp_ajeeb(cn, name) == 0` (direct self-reference). A→B→C→A was not detected — tree recursed 10 levels deep then stopped.

**Fix:** Added `ancestors` parameter (comma-separated string) passed through recursive calls. Each node checks if its name is in the ancestor set before recursing. Labels cycle as `(cycle)`.

## Test Results (22/22 pass)

### test-semver (16/16)
All SemVer parsing, comparison, and constraint matching tests pass (unchanged from M1.2).

### test-tree (6/6)
| Test | Description | Result |
|------|-------------|--------|
| 1 | Linear chain (A→B→C) | ✓ |
| 2 | Tree display (A→B→C) | ✓ |
| 3 | Shared dependency (A→B→D, A→C→D) | ✓ |
| 4 | Indirect cycle (A→B→A) | ✓ |
| 5 | Version conflict (bar@1.0.0 vs bar@2.0.0) | ✓ |
| 6 | Conflict detection assertion | ✓ |

## Build Stats
- Source: 1,676 lines (parth_m1.ajb)
- Binary: 83KB
- All 22 tests pass
