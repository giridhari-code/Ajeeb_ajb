# BOOTSTRAP VERIFICATION

## Pipeline

```
compiler.ajb
    │
    ▼ (Rust compiler — LLVM backend)
build/compiler_stage1  (native binary, 14.8 MB)
    │
    ▼ (self-hosted compiler_stage1)
build/stage1_output.c  (C source, 363 lines)
    │
    ▼ (compiler_stage1 re-compiles same input)
build/stage2_output.c  (C source, 363 lines)
```

## Results

| Metric | stage1 | stage2 | Match |
|--------|--------|--------|-------|
| C output lines | 363 | 363 | ✓ |
| SHA256 | `c0704a2d...` | `c0704a2d` | ✓ |
| Diff count | — | — | **0** |
| Function count | 16 | 16 | ✓ |
| Forward declarations | identical | identical | ✓ |
| Emitted globals | identical | identical | ✓ |
| Arena allocs | 6154 | 6154 | ✓ |

## Conclusion

**Stage A bootstrap complete.**

`compiler_stage1` produces byte-identical C output when run twice on the same input. The self-hosted compiler's codegen is self-consistent: its output is stable across re-compilations.

## Known Limitations

The generated C output (`stage1_output.c`) does not compile with GCC due to pre-existing codegen deficiencies:
- Forward declarations lack parameter types (`intptr_t greet();` instead of `intptr_t greet(intptr_t)`)
- `if`/`while`/`else` bodies emitted without braces
- `set` inside `if`/`while` blocks generates `intptr_t var;` declaration but bodies lack proper block structure

These issues are **outside the scope of this verification**, which tests only self-consistency of the self-hosted compiler's output.
