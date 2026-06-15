# Ajeeb Compiler — Agent Guide

## Bootstrap Self-hosting Check
```bash
bash tests/bootstrap_check.sh
```
This runs the 4-step pipeline:
1. Rust interpreter compiles `compiler/compiler.ajb` → `build/output.c`
2. GCC compiles output.c + runtime → `build/ajeeb_native`
3. `build/ajeeb_native` compiles `compiler/compiler.ajb` → `build/output2.c`
4. `diff` and `sha256sum` verify output.c ≡ output2.c

## Cargo Tests
```bash
cargo test
```

## Ajeeb Interpreter Tests
Run individual .ajb files:
```bash
cargo run -p ajeeb-compiler --bin ajeeb_compiler tests/<test_file>.ajb
```
Key test files: test_simple, test_small, test_strings, test_math, test_for, test_if, test_while, test_array, cross_simple, compiler_test

## After Any Change
1. Run `cargo test`
2. Run `bash tests/bootstrap_check.sh`
3. Run a few key .ajb interpreter tests (e.g. `cross_simple.ajb`, `test_strings.ajb`)
