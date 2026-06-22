# ajeebc — Ajeeb Compiler

Compile `.ajb` files to native binary!

## Build

```bash
make            # build everything (rust compiler + native compiler)
make rust       # build Rust compiler only (rustc, no Cargo)
make native     # compile compiler.ajb → native binary
make test       # compile and run all test files
make clean      # remove build artifacts
```

## Use

```bash
./build/ajeeb_compiler file.ajb --skip-run  # LLVM codegen
./build/ajeeb_compiler file.ajb             # compile and run
```

## Architecture

- Stage 0: Rust lexer/parser/codegen (`crates/ajeeb-compiler`)
