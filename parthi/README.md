# ParthI — Ajeeb Interpreter

Run `.ajb` files without compiling! ParthI ek MIR (Mid-Level IR) interpreter hai.

## Use

```bash
parthi file.ajb
parthi src/main.ajb
```

## How it works

1. Lex → Parse → HIR (High-Level IR)
2. HIR → MIR (Mid-Level IR)
3. MIR execute karta hai built-in interpreter se
4. No LLVM, no GCC needed!
