# Ajeeb Compiler — Architecture Fix Report

## Overview

Four critical bugs were discovered and fixed in the self-hosted Ajeeb compiler's
C codegen pipeline (`main.ajb` + `stmt.ajb`). All four involved the shared
`state` buffer where HIR, MIR, scope table, and metadata coexist.

---

## Buffer Layout

### Before (broken)

```
┌─────────────────────────────────────────────────────────┐
│ Slot 0-127   : Reserved                                │
│ Slot 128-255 : Scope table (128 entries × 2 slots)     │
│ Slot 256     : scCnt ← OVERLAPS scope entry #32!       │
│ Slot 257     : scMax                                   │
│ Slot 390-399 : CLI config                              │
│ Slot 400-499 : Block offsets (during lowering)          │
│ Slot 502     : blockCount                              │
│ Slot 504     : instrCount (per-block)                  │
│ Slot 506     : current block offset                    │
│ Slot 508     : temp counter                            │
│ Slot 510     : MIR heap pointer = 1024                 │
│ Slot 511     : HIR heap pointer = 512                  │
│ Slot 512-1023: HIR data (grows up)                     │
│ Slot 1024-   : MIR data (grows up) ← OVERLAPS HIR!     │
│                                                         │
│ Total buffer: 16384 bytes = 2048 slots                  │
└─────────────────────────────────────────────────────────┘

Problems:
  1. MIR starts at slot 1024, HIR grows up from 512
     → files with >512 HIR slots corrupt MIR (e.g., stmt.ajb at 7847 slots)
  2. scCnt at slot 256 overlaps scope entry #32 (128 + 32×4 = 256)
     → >32 scopes corrupt scCnt, causing undefined behavior
  3. emitCFn uses mFnLocalCount (lc) for temp declarations
     → lc is wrong for functions with loops (MIR temp counter resets per-function)
```

### After (fixed)

```
┌─────────────────────────────────────────────────────────┐
│ Slot 0-127   : Reserved                                │
│ Slot 128-383 : Scope table (64 entries × 4 slots)      │
│ Slot 384     : scCnt (moved from 256)                  │
│ Slot 385     : scMax                                   │
│ Slot 390-399 : CLI config                              │
│ Slot 400-499 : Block offsets (during lowering)          │
│ Slot 502     : blockCount                              │
│ Slot 504     : instrCount (per-block)                  │
│ Slot 506     : current block offset                    │
│ Slot 508     : temp counter                            │
│ Slot 510     : MIR heap pointer = 16384                │
│ Slot 511     : HIR heap pointer = 512                  │
│ Slot 512-16383 : HIR data (grows up, 15872 slot budget)│
│ Slot 16384-  : MIR data (grows up, 16384 slot budget)  │
│                                                         │
│ Total buffer: 262144 bytes = 32768 slots                │
└─────────────────────────────────────────────────────────┘

Fixes:
  1. MIR starts at slot 16384 — well beyond max HIR (15872 slots)
  2. scCnt moved to slot 384 — after scope table area (128-383)
  3. emitCFn scans MIR instructions for maxTemp instead of using lc
```

---

## Bug #1: HIR/MIR Buffer Overlap

### Root Cause

`minit(state)` set the MIR heap pointer to slot 1024. HIR starts at slot 512
and grows upward. Files with >512 HIR slots (e.g., `stmt.ajb` at 7847 slots)
overwrote MIR data, causing corrupted codegen output.

### Evidence

Instrumentation via `writeAppend("/tmp/hir_mir_diag.txt", ...)` showed:

| File | HIR max slot | MIR start | Overlap? |
|------|-------------|-----------|----------|
| emit.ajb | 656 | 1024 | No |
| lexer.ajb | 2448 | 1024 | **YES** |
| expr.ajb | 3384 | 1024 | **YES** |
| stmt.ajb | 7847 | 1024 | **YES** |
| pass1.ajb | 2256 | 1024 | **YES** |

### Fix

- Buffer: 16384 → 262144 bytes (2048 → 32768 slots)
- MIR heap: slot 1024 → slot 16384
- Files changed: `main.ajb`, `compiler.ajb`, `ajeeb_runtime.c`, `llvm/mod.rs`, `c_codegen.rs`

---

## Bug #2: Scope Table Overflow

### Root Cause

`scCnt` was at slot 256, but the scope table occupies slots 128-383.
Scope entry `i` stores data at slots `128 + i*4` through `128 + i*4 + 3`.
Entry 32 starts at slot `128 + 32*4 = 256`, which **overlaps `scCnt`**.

When a function had >32 scopes (e.g., `test_scope.ajb` with 33 functions),
`scDefine` would write to the scope entry, overwriting `scCnt`. Subsequent
`scLookup` calls would read the corrupted count and iterate incorrectly.

### Fix

- `scCnt` moved from slot 256 to slot 384
- `scMax` moved from slot 257 to slot 385
- Scope table now spans slots 128-383 (64 entries × 4 slots)
- Files changed: `main.ajb` (lines 610-614)

---

## Bug #3: C Codegen Temp Variable Declaration

### Root Cause

`emitCFn` used `mFnLocalCount(buf, foff)` to determine how many temp variables
(`t0, t1, ...`) to declare in the C output. But `lc` (local count) was set from
`br(mirBuf, 508)` at the end of `lowerFnDef` — the MIR temp counter at that
point, which reflects the **global** temp counter, not the function-specific max.

For functions with while/for loops, the temp counter could be much higher than
the actual temps used, OR the temp count could be wrong because `freshTemp`
increments a global counter across all functions.

The result: the generated C declared too few (or wrong) temp variables, causing
GCC compilation errors like `'t1' undeclared`.

### Fix

Instead of using stored `lc`, scan all MIR instructions in the function to find
the actual max temp index:

```ajeb
set bi2: int = 0; set maxTemp: int = pc - 1;
while (bi2 < bc) {
    set blkOff: int = mFnBlock(buf, foff, bi2);
    set ic: int = mBlockInstrCount(buf, blkOff);
    set ii: int = 0;
    while (ii < ic) {
        set ioff: int = mBlockInstr(buf, blkOff, ii);
        set dd: int = mInstrDst(buf, ioff);
        set ss1: int = mInstrS1(buf, ioff);
        set ss2: int = mInstrS2(buf, ioff);
        if (dd > maxTemp) { maxTemp = dd; }
        if (ss1 > maxTemp) { maxTemp = ss1; }
        if (ss2 > maxTemp) { maxTemp = ss2; }
        ii = ii + 1;
    }
    bi2 = bi2 + 1;
}
```

- Files changed: `main.ajb` (lines 1327-1343)

---

## Bug #4: Import Handler `identStr`/`getOutbuf()` Aliasing

### Root Cause

`identStr()` in `lexer.ajb` returns `getOutbuf()` — the shared output buffer.
The import handler in `stmt.ajb` called `identStr()` to get the module name,
then called `getOutbuf()` again for the path scratch buffer. Both returned the
same string, so writing "compiler/" prefix to the scratch buffer overwrote the
module name.

Additionally, `identStr()` does NOT null-terminate. After `str_concat("compiler/", modName)`, `len(modName)` could return a stale/incorrect length, causing
the path construction loop to iterate millions of times (appearing as an
infinite loop).

### Fix

Read module name characters directly from source using `tokStrOff(buf)` and
`tokStrLen(buf)` instead of `identStr()`:

```ajeb
// OLD (broken):
set modName: string = identStr(buf);  // returns getOutbuf()
set scratchO: string = getOutbuf();   // SAME buffer!
str_concat("compiler/", modName);     // overwrites modName

// NEW (fixed):
set modOff: int = tokStrOff(buf);
set modLen: int = tokStrLen(buf);
// Build path character by character from source
```

- Files changed: `stmt.ajb` (line 218+, import handler)

---

## Verification

### Test Results

| Test | Status |
|------|--------|
| test_simple | PASS |
| test_math | PASS |
| test_if | PASS |
| test_while | PASS |
| test_for | PASS |
| test_strings | PASS |
| test_scope | PASS |
| cargo test | 8/8 PASS |
| bootstrap_check.sh | PASS |

### Self-hosted Compiler

- `build/main` (146KB): compiled from fixed `main.ajb` via Rust LLVM backend
- Can compile all individual compiler modules (emit, lexer, expr, stmt, pass1)
- All 6 core test files compile and run correctly via C codegen

### Files Changed

1. `compiler/main.ajb` — Buffer 262144, MIR start 16384, scCnt at 384, maxTemp scan
2. `compiler/compiler.ajb` — Buffer 262144
3. `runtime/ajeeb_runtime.c` — extern buffer 262144
4. `crates/ajeeb-compiler/src/llvm/mod.rs` — LLVM buffer 262144
5. `crates/ajeeb-compiler/src/c_codegen.rs` — C buffer 262144
6. `compiler/stmt.ajb` — Import handler fix (identStr → direct source read)
