# MEMORY_CORRUPTION_REPORT.md

## Verdict: HIR/MIR OVERLAP CONFIRMED

## Buffer Layout (current)

```
__ajeeb_buf[16384] = 16384 bytes = 2048 slots (8 bytes/slot)

Slot Map:
  0-49:     Reserved (lexer state at 50-53, position at 60)
  128-383:  Scope table
  256-383:  Function table
  400-499:  Block scratch (parsing + MIR lowering reuse this)
  498:      Call nesting depth
  502:      Block ID counter
  504:      Current block instruction count
  506:      Current block MIR offset
  508:      Temp counter
  510:      MIR allocator head → initialized to 1024 by minit()
  511:      HIR allocator head → initialized to 512 by hinit()
  512-1023: HIR budget (512 slots)
  1024-2047: MIR budget (1024 slots)
```

## The Bug

HIR and MIR share `__ajeeb_buf` and grow toward each other:

- **HIR** starts at slot **512**, grows **upward** via `halloc()`
- **MIR** starts at slot **1024**, grows **upward** via `mallac()`
- **HIR budget**: 512 slots (512→1023)
- When HIR exceeds 512 slots, it writes into slot 1024+ which is the MIR region
- `minit()` then sets MIR head to 1024, overwriting HIR data stored there
- `lowerProgram()` reads corrupted HIR → generates wrong MIR → C codegen produces garbage

## Proof (instrumented output)

Added `writeFile("/tmp/hir_mir_diag.txt", ...)` calls at key points in `main.ajb`:
- After `hinit()`: records `HIR_HEAD_INIT`
- After `parseProgram()`: records `AFTER_PARSE_HIR_HEAD`
- After `minit()`: records `MIR_HEAD_INIT`
- After `lowerProgram()`: records `MIR_HEAD_AFTER_LOWER`

### Raw diagnostic output per file:

#### test_simple.ajb (8 lines, 142 bytes) — NO OVERLAP
```
HIR_HEAD_INIT=512
SRC_LEN=142
AFTER_PARSE_HIR_HEAD=570
MIR_HEAD_INIT=1024
MIR_HEAD_AFTER_LOWER=1104
```
- HIR used: 570 - 512 = **58 slots** (of 512 budget)
- MIR used: 1104 - 1024 = **80 slots** (of 1024 budget)
- HIR max (570) < MIR start (1024) → **SAFE**

#### emit.ajb (33 lines, 1425 bytes) — PARSE CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=1425
```
- Segfault during `parseProgram()` — no AFTER_PARSE written
- Separate issue (likely undefined function `chr()`)

#### pass1.ajb (211 lines, 8559 bytes) — PARSE CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=8559
```
- Segfault during `parseProgram()` — no AFTER_PARSE written
- Separate issue (likely undefined functions)

#### expr.ajb (308 lines, 12325 bytes) — OVERLAP + LOWER CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=12325
AFTER_PARSE_HIR_HEAD=7585
MIR_HEAD_INIT=1024
(no MIR_HEAD_AFTER_LOWER — segfault in lowerProgram)
```
- HIR used: 7585 - 512 = **7073 slots** (of 512 budget)
- HIR max (7585) >> MIR start (1024) → **OVERLAP: 6561 slots**
- `lowerProgram()` segfaults reading corrupted HIR

#### stmt.ajb (364 lines, 14408 bytes) — OVERLAP + LOWER CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=14408
AFTER_PARSE_HIR_HEAD=7847
MIR_HEAD_INIT=1024
(no MIR_HEAD_AFTER_LOWER — segfault in lowerProgram)
```
- HIR used: 7847 - 512 = **7335 slots** (of 512 budget)
- HIR max (7847) >> MIR start (1024) → **OVERLAP: 6823 slots**
- `lowerProgram()` segfaults reading corrupted HIR

#### lexer.ajb (227 lines, 9252 bytes) — OVERLAP + LOWER CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=9252
AFTER_PARSE_HIR_HEAD=6715
MIR_HEAD_INIT=1024
(no MIR_HEAD_AFTER_LOWER — segfault in lowerProgram)
```
- HIR used: 6715 - 512 = **6203 slots** (of 512 budget)
- HIR max (6715) >> MIR start (1024) → **OVERLAP: 5691 slots**
- `lowerProgram()` segfaults reading corrupted HIR

#### compiler.ajb (115 lines, 4793 bytes) — OVERLAP + LOWER CRASH
```
HIR_HEAD_INIT=512
SRC_LEN=4793
AFTER_PARSE_HIR_HEAD=1670
MIR_HEAD_INIT=1024
(no MIR_HEAD_AFTER_LOWER — segfault in lowerProgram)
```
- HIR used: 1670 - 512 = **1158 slots** (of 512 budget)
- HIR max (1670) >> MIR start (1024) → **OVERLAP: 646 slots**
- `lowerProgram()` segfaults reading corrupted HIR

## Summary Table

| File | Lines | HIR Slots Used | Budget | Overlap | Crash Phase |
|------|-------|---------------|--------|---------|-------------|
| test_simple.ajb | 8 | 58 | 512 | NO | OK |
| emit.ajb | 33 | N/A | 512 | N/A | parse |
| pass1.ajb | 211 | N/A | 512 | N/A | parse |
| expr.ajb | 308 | 7073 | 512 | 6561 | lower |
| stmt.ajb | 364 | 7335 | 512 | 6823 | lower |
| lexer.ajb | 227 | 6203 | 512 | 5691 | lower |
| compiler.ajb | 115 | 1158 | 512 | 646 | lower |

## Root Cause

**Every `.ajb` file with more than ~50 lines of function bodies will overflow the 512-slot HIR budget.** The HIR arena (slots 512-1023) is shared with the MIR arena (slots 1024+). When HIR grows past slot 1024, `minit()` and subsequent MIR writes overwrite HIR data, corrupting function names, parameter counts, and type information.

## Corrupted Output Evidence

The generated C file for `lexer.ajb` (`build/lexer.c` lines 48-51) shows:
```c
intptr_t // Lexer —(intptr_t);           // Should be: intptr_t matchKwd(intptr_t, intptr_t, intptr_t)
intptr_t (intptr_t, intptr_t, ..., intptr_t);  // Empty name, 2197 params (should be ~5)
intptr_t ();                              // Empty name, 0 params
intptr_t ();                              // Empty name, 0 params
```

The first 5 functions (isDigit, isAlpha, isAlphaNum, isSpace, skipWS) compile correctly because their HIR data occupies slots 512-~900, below the MIR start at 1024. The 6th function (matchKwd) and beyond have HIR data at slots 1024+, which gets overwritten.
