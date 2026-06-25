# HOTPATH REPORT — compiler.ajb Compile-Time Analysis

## Summary

The compile-time bottleneck is **llc (LLVM IR → assembly)** at 48% of total time.
Temp-file I/O is **not** dominant — the Ajeeb `writeAppend`/`writeFile`/`readFile`
builtin counters are 0 during compilation because the LLVM codegen path uses
Rust's native `writeln!`, not the Ajeeb runtime.

## Phase Breakdown (compiling compiler.ajb)

| Phase | Time | % of Total |
|-------|------|-----------|
| Rust parse + LLVM IR gen | ~0.76s | 29% |
| **llc (IR → assembly)** | **~1.28s** | **48%** |
| as + ld (assembly → binary) | ~0.62s | 23% |
| **Total** | **~2.65s** | 100% |

## Builtin Call Counts (interpreter mode)

When the Ajeeb interpreter runs a program with I/O:

| Function | Calls | Bytes |
|----------|-------|-------|
| writeAppend | 3,000 | 8,890 |
| writeFile | 1 | 6 |
| readFile | 0 | 0 |
| getOutbuf | 0 | 0 |

**During compiler.ajb compilation (LLVM path): ALL COUNTERS = 0.**

The LLVM codegen generates native code. The `writeAppend`/`writeFile` calls
in compiler.ajb become native function calls in the compiled binary — they
only fire at runtime, not during compilation.

## Root Cause

The Rust compiler's compile pipeline for compiler.ajb:

1. **Parse** → HIR → THIR → MIR (Rust, ~0.76s)
2. **MIR → LLVM IR** (Rust, included in above)
3. **llc -O2** → assembly (~1.28s) ← bottleneck
4. **gcc** → linked binary (~0.62s)

The llc step is slow because the generated LLVM IR is 8,335 lines / 242 KB
for a ~115-line Ajeeb source file. The IR is verbose because:
- Every function gets its own LLVM basic blocks
- String literals are stored as global constants
- The `@ajeeb_buf` and `@ajeeb_outbuf` globals are 262KB and 64KB
- No LLVM optimization passes beyond `-O2` in llc

## Recommendation

The bottleneck is **llc compilation time**, not Ajeeb-level I/O. Options:

1. **Reduce IR verbosity** — merge trivial basic blocks, inline small functions
2. **Use `opt` before `llc`** — run LLVM optimization passes to reduce IR size
3. **Parallelize** — split compilation into independent units
4. **Switch to GCC JIT** — avoid the llc round-trip entirely

Memory-mode replacement for writeAppend/writeFile is **not justified** by
these measurements — the counters are 0 during compilation.
