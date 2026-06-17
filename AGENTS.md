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

## Key Bug Fix: While Loop Loop-back Edge (thir_to_mir.rs)
**Root cause:** When a while/for loop body ends with an `if` without `else`, `lower_if` calls `start_block()` for the merge block, which empties `current_stmts`. Then `is_terminated()` returns true, causing the loop-back `Goto(header_block)` to be skipped. The body never loops back to the header.

**Fix:** In `lower_while` and `lower_for`, always emit the loop-back `Goto` after the body (remove the `if !self.is_terminated()` guard). Safe because if the body already ended with return/break, the new block is unreachable but harmless.

## Key Bug Fix: If-Else Else Block Index (thir_to_mir.rs)
**Root cause:** `lower_if` computed the else block's SwitchInt default target as `then_block + 1`, but when the then branch contained loops, many blocks were created between then and else, making the else block unreachable. This caused `else { writeAppend(out, "Ident("); ... }` to be dropped from the generated IR.

**Fix:** Save the actual else block index from `start_block()` and use it directly instead of `then_block + 1`.

## LLVM Codegen Runtime Functions
Known to codegen: `getInt`, `setInt`, `getStateBuf`, `getOutbuf`, `charCode`, `len`, `strSet`, `writeFile`, `writeAppend`, `writeByte`, `itoa`, `println`, `readFile`, `strcmp_ajeeb`, `str_concat`, `substring`, `indexOf`, `contains`, `toUpperCase`, `toLowerCase`, `trim`, `startsWith`, `endsWith`, `replace`, `array_to_string`.
NOT known: `chr`, `rdPos`, `wrPos` (evaluator-only).
