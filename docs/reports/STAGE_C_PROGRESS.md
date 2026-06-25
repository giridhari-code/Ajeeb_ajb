# STAGE_C_PROGRESS.md

**Date:** 2026-06-25
**Status:** IN PROGRESS — C1-C5 code complete, pending integration testing

---

## Milestones

| Milestone | Description | Status |
|-----------|-------------|--------|
| C1 | LLVM IR Core (module, preamble, functions, types, globals, entry, IR writer) | Done |
| C2 | Expressions (int/bool literals, arithmetic, comparisons, variables, assignments) | Done |
| C3 | Control Flow (if, else, while, for, return) | Done |
| C4 | Functions (declarations, parameters, calls, recursion, locals) | Done |
| C5 | Memory (alloca, load, store, globals, string constants) | Done |

## Files

| File | Lines | Description |
|------|-------|-------------|
| `compiler/emit_llvm.ajb` | ~700 | LLVM IR codegen module — new file |
| `compiler/main.ajb` | +40 | LLVM backend wiring (replaced stub) |

## What's Done

1. **LLVM IR codegen module** (`emit_llvm.ajb`): Walks MIR basic blocks and emits LLVM IR text. Covers all 14 MIR opcodes with proper LLVM IR patterns.

2. **Backend integration** (`main.ajb`): Replaced the "LLVM path coming soon" stub with actual `emitLLVMProg` call + `llc`/`as`/`cc` compilation pipeline.

3. **Verified LLVM IR output**: `test_llvm_codegen.ajb` generates valid `.ll` file that compiles with `llc` and links with the runtime. Binary runs correctly.

4. **C backend preserved**: No changes to C backend. `compiler.ajb` still compiles and works.

## What's NOT Done

1. **String literal emission (ext=13)**: The MIR opcode for string literals (`BINOP ext=13`) is stubbed. It stores `0` instead of a string pointer. This is the main blocker for running real programs through the LLVM backend.

2. **Self-hosting test**: `main.ajb` cannot be compiled by the C backend (too many temps). To test the LLVM codegen on real programs, we need the self-hosted compiler to already be compiled. This is a bootstrap chicken-and-egg problem.

3. **Full test suite comparison**: Haven't run all test files through both C and LLVM backends to verify identical behavior.

## Key Design Decisions

- **MIR reuse**: The LLVM codegen reads the same MIR data structures as the C codegen. No changes to HIR, THIR, or MIR.
- **Type representation**: All Ajeeb values are `i64` in LLVM IR (same as C backend's `intptr_t`).
- **String tracking**: Reuses the existing string temp bitmap (slots 480+) for `strcmp_ajeeb` vs `icmp eq` dispatch.
- **Function naming**: Uses `@funcname` (not mangled), matching the C backend's `funcname()` pattern.
- **Entry point**: `define i32 @main()` for entry function, `define i64 @func()` for others.

## Blocker: String Literal Emission

The MIR instruction `BINOP ext=13, s1=offset, s2=length` represents a string literal. The C backend emits:
```c
tN = (intptr_t)"escaped content";
```

For LLVM IR, this needs either:
- **Option A**: Emit `@.str.N = private constant [N x i8]` globals (already implemented in `emitLLVMStringGlobal`) and use `getelementptr` + `ptrtoint` at each use site.
- **Option B**: Use `@.str.N` globals but require a pre-scan pass to collect all string literals before function emission.

The `emitLLVMProg` function already has Option A's string global collection logic. The remaining work is wiring it into `emitLLVMBlockCode` for opcode ext=13.

## Next Session Priorities

1. Fix ext=13 string literal emission
2. Test with `test_simple.ajb` (Hello World)
3. Test with `test_math.ajb`, `test_if.ajb`, `test_while.ajb`
4. Compare C and LLVM backend outputs
5. Write C6_REPORT.md
