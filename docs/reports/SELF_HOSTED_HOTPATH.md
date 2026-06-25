# SELF_HOSTED HOTPATH REPORT — Self-Hosted Compiler Performance

## Summary

The self-hosted compiler (`build/compiler` compiling `compiler/compiler.ajb` → C)
has a **temp-file I/O bottleneck**. Every binary expression creates, writes, reads,
and deletes temporary files on disk. This accounts for ~6.1s of cumulative I/O time
out of a ~7.4s wall-clock compilation.

## Wall-Clock Measurement

| Metric | Value |
|--------|-------|
| Total compile time (wall) | ~7.4s |
| Source file | `compiler/compiler.ajb` (1279L → split 115L main + 6 modules) |

## C Runtime I/O Counters (cumulative across all calls)

| Function | Calls | Bytes Transferred | Cumulative Time |
|----------|-------|-------------------|-----------------|
| `writeAppend` | 5,308 | 25,151 B | ~1.54s |
| `writeFile` | 1,128 | 30 B | ~2.40s |
| `readFile` | 1,128 | 23,255 B | ~2.14s |
| `getOutbuf` | 713 | — | — |
| **Total I/O** | **8,277** | **48,436 B** | **~6.1s** |

**Note:** Times are cumulative (sum across all calls), not wall-clock. The I/O
calls overlap with each other and with CPU work.

## Root Cause: Temp-File Expression Protocol

The self-hosted compiler uses **temp files** as intermediate storage between
parse functions. The pattern appears in two places:

### 1. `parseAdd` (expr.ajb:227) — Binary `+`/`-` expressions

```
parseAdd(src, buf, out):
    tmp = "build/ap{ctr}.txt"       // unique filename via counter
    writeFile(tmp, "")               // clear temp file
    parseMul(src, buf, tmp)          // write left operand to tmp
    while (tokType == PLUS or MINUS):
        left = readFile(tmp)         // read left operand
        writeFile(tmp, "")           // clear tmp
        parseMul(src, buf, tmp)      // write right operand to tmp
        right = readFile(tmp)        // read right operand
        writeFile(tmp, "")           // clear tmp
        writeAppend(tmp, combined)   // write combined result
    emitStr(out, readFile(tmp))      // read final result
```

For a binary expression `a + b`:
- 1× `writeFile` (clear) + 1× `writeFile` (clear) + 1× `writeAppend` + 2× `readFile` = 5 I/O ops
- For `a + b + c`: 9 I/O ops (2 iterations)

**This is the dominant cost.** The 5,308 `writeAppend` + 1,128 `readFile` + 1,128
`writeFile` calls are mostly from this pattern.

### 2. `parsePrimary` (expr.ajb:128) — `println`/`print` argument concatenation

```
print(args...):
    ptmp = "build/_pp{ctr}.txt"
    writeFile(ptmp, "")
    parseExpr → ptmp                 // first arg
    for each additional arg:
        left = readFile(ptmp)
        writeFile(ptmp, "")
        parseExpr → ptmp             // next arg
        right = readFile(ptmp)
        writeFile(ptmp, "")
        writeAppend(ptmp, "str_concat(left, right)")
    emitStr(out, readFile(ptmp))
```

Same pattern. Each print call with N arguments does ~3N I/O operations.

## Why writeFile/readFile Are Slow

1. **`fflush()` after every write** — `ajeeb_runtime.c` calls `fflush(fp)` after
   every `writeAppend`/`writeFile` call. This forces a kernel syscall per I/O op.
2. **`fopen`/`fclose` per call** — Each `readFile` opens the file, reads it, and
   closes it. Each `writeFile`/`writeAppend` opens (or creates) the file.
3. **`unlink()` after each read** — `readFile` deletes the temp file after reading.
   This means each binary expression creates+destroys a file.
4. **Small files** — Most temp files are <100 bytes. The overhead of
   `open`/`write`/`read`/`close`/`unlink` dwarfs the actual data transfer.

## I/O Breakdown by Source

| Source Location | I/O Type | Calls (est.) | Notes |
|-----------------|----------|-------------|-------|
| `parseAdd` | `writeAppend` | ~5,000+ | Builds combined expression |
| `parseAdd` | `readFile` | ~1,128 | Reads operands back |
| `parseAdd` | `writeFile` | ~1,128 | Clears temp between operands |
| `parsePrimary` (print) | `writeAppend` | ~300+ | Print arg concatenation |
| `parsePrimary` (print) | `readFile` | ~100+ | Print arg reading |
| Import handler | `readFile` | 5 | Reads imported files |
| `emitStr` → `writeAppend` | `writeAppend` | ~4,000+ | Actual C output generation |

## Recommendation: Memory-Mode Replacement

**Replace the temp-file protocol with in-memory buffers.** The `out` parameter
already accumulates C code via `writeAppend(out, ...)`. The same pattern should
work for `parseAdd`:

```
// Instead of:
writeFile(tmp, ""); parseMul(src, buf, tmp); left = readFile(tmp);
// Use:
set leftBuf: string = getOutbuf(); parseMul(src, buf, leftBuf); left = leftBuf;
```

This eliminates all temp-file I/O for expression parsing. Estimated impact:
- Remove ~5,000 `writeAppend`, ~1,100 `readFile`, ~1,100 `writeFile` calls
- Eliminate ~2,256 file create/destroy cycles
- Reduce I/O time from ~6.1s to ~0.5s (only import reads + output generation remain)
- Wall-clock time: ~7.4s → ~1-2s estimated

## Blocking Issue: Phase 3 GCC Validation

The self-hosted binary can be rebuilt via `./build/ajeeb_compiler compiler/compiler.ajb --skip-run`
(Rust compiler → C → GCC). However, adding timer function forward declarations
to `compiler.ajb` triggers a variable scoping bug in the generated C (`v743
undeclared`), which prevents GCC compilation. This blocks:
1. Adding parseAdd timing hooks for profiling
2. Adding any new C runtime function calls to the Ajeeb source

## Status

| Task | Status |
|------|--------|
| C runtime counters installed | ✅ Complete |
| Counter measurements captured | ✅ Complete |
| Temp-file pattern identified | ✅ Complete |
| ParseAdd timing hooks | ❌ Blocked (codegen variable scoping bug) |
| Memory-mode implementation | ⬜ Pending |
| Phase 3 GCC Validation | ❌ Open (variable scoping in generated C) |
| SELF_HOSTED_HOTPATH.md | ✅ This file |
