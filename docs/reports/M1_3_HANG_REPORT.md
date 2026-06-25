# M1.3 Hang Report

## Current State

- **Binary:** `/root/ajeeb_compiler/build/parth_m1` (145KB, compiled)
- **Process:** No process running — last run completed
- **Last command:** `test-tree`

## Test Results (last run)

| Test | Status | Detail |
|------|--------|--------|
| Test 1: Linear chain | PASS | `foo → bar → shared` resolves correctly |
| Test 2: Tree display | PASS | Tree renders with box-drawing chars |
| Test 3: Shared dependency | PASS | `foo→shared` and `baz→shared` both shown |
| Test 4: Cycle detection | PASS* | Depth limit (10) stops recursion, but cycle NOT properly detected |
| Test 5: Version conflict | **FAIL** | Conflict not detected; name string corrupted |

## Bug #1: Buffer Corruption in BFS Resolver (Test 5 FAIL)

**Root cause:** `getStateBuf()` returns a SINGLE shared buffer. `parseDasConfig()` overwrites slots 0-200+ every time it's called.

**BFS resolver stores queue data at:**
- Slots 900-949: queue metadata (nameLen, verLen)
- Slots 950+: queue string data (name chars, version chars)

**When `parseDasConfig(pkgPath)` is called for each transitive dep:**
1. It calls `getStateBuf()` → returns SAME buffer
2. It writes dep data starting at slot 0
3. It overwrites slots 950+ where queue strings are stored
4. Subsequent `bufToStr(buf, si, nameLen)` reads corrupted data

**Evidence:** Error message shows `bar???????????????????????????2.0.0?????shared??` — the `bar` name bytes are overwritten by `parseDasConfig` output for subsequent deps.

**Fix:** Store queue string data in Ajeeb string objects (not buffer slots), OR use separate buffer instances, OR re-enqueue names from original source strings before calling `parseDasConfig`.

## Bug #2: Cycle Detection Incomplete (Test 4)

**Current implementation:** `printTree` only checks if child name == current name (direct self-reference).

**Problem:** Does NOT detect indirect cycles (A→B→C→A). Only catches A→A.

**Evidence:** Test 4 output shows 10+ levels of `foo→bar→foo→bar→...` before depth limit stops it.

**Fix:** Track full ancestor path in `printTree` (pass visited set through recursion), or mark visited nodes globally.

## Bug #3: Conflict Detection Logic Flawed

**Current check:** Only flags conflict when BOTH constraints are exact (`type == 1`) AND different.

**Problem:** Should also detect:
- `^1.0.0` vs `^2.0.0` (caret range mismatch)
- `~1.2.3` vs `~1.3.0` (tilde range mismatch)
- Any two constraints with no overlapping version

**Fix:** Implement proper constraint intersection check (not just exact-vs-exact).

## Last Completed Test

Test 3 (shared dependency) was the last fully passing test.

## Suspected Blocker

**Buffer corruption (Bug #1)** is the primary blocker. The BFS resolver's queue data is destroyed by `parseDasConfig` calls, making the resolver produce garbage output.

## Exact Next Fix

**Priority 1:** Fix buffer corruption in `resolveAll()`:
- Store queue names/versions as Ajeeb string variables (not buffer slot character arrays)
- OR use a separate buffer for package parsing (call `parseDasConfig` on a different buf)
- OR rebuild the queue strings from the original `src` config before each `parseDasConfig` call

**Priority 2:** Fix cycle detection in `printTree()`:
- Pass a visited-name string set through recursion
- Check if child name is in the ancestor set before recursing

**Priority 3:** Improve conflict detection:
- Check if constraint ranges overlap, not just exact equality
