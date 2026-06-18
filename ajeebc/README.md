# ajeebc — Ajeeb Compiler

Compile `.ajb` files to native binary!

## Build

```bash
cargo build --release -p ajeeb-compiler
cp target/release/ajeeb_compiler build/ajeebc
```

## Use

```bash
ajeebc file.ajb            # LLVM codegen → .ll
```

## Architecture

- Stage 0: Rust lexer/parser/codegen (`crates/ajeeb-compiler`)
- Stage 1-3: Self-hosting compiler in Ajeeb (`compiler/compiler.ajb`)
- Runtime: C runtime library (`runtime/ajeeb_runtime.c`)
