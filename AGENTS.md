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

## Key Bug Fix: Codegen Interpreter HashMap Key Collision (codegen.ajb)
**Root cause:** The interpreter's `setInt`/`getInt` use the **string content** as the HashMap key for integer buffers. When `buf` (output) and `ast` (AST storage) are the same string object, writing to `buf` via `strSet` changes the string content, which changes the lookup key. Subsequent `getInt` calls fail (return 0) because the key no longer matches.

**Fix:** Always use separate strings for `buf` (output buffer) and `ast` (AST storage). Use `getOutbuf()` for output (character buffer) and `getStateBuf()` for AST (integer buffer). Never pass the same string as both `buf` and `ast` to codegen functions.

## Key Bug Fix: LLVM Runtime strSet Missing Null-Termination
**Root cause:** The C runtime's `strSet` writes a character at a given position but does not null-terminate. After `getOutbuf()` sets `buf[0] = '\0'`, writing `buf[0] = 'i'` leaves old data at positions 1..N. `strlen(buf)` scans past position 0 and finds old data, returning a stale length. Subsequent writes go to the wrong position.

**Fix:** In `ajeeb_runtime.c`, `strSet` now always writes `buf[i+1] = '\0'` after writing `buf[i] = c`. This ensures `strlen` returns the correct length after each sequential write.

## LLVM Codegen Runtime Functions
Known to codegen: `getInt`, `setInt`, `getStateBuf`, `getOutbuf`, `charCode`, `len`, `strSet`, `writeFile`, `writeAppend`, `writeByte`, `itoa`, `println`, `readFile`, `strcmp_ajeeb`, `str_concat`, `substring`, `indexOf`, `contains`, `toUpperCase`, `toLowerCase`, `trim`, `startsWith`, `endsWith`, `replace`, `array_to_string`.
NOT known: `chr`, `rdPos`, `wrPos` (evaluator-only).

## Ajeeb Limitations in .ajb Self-Hosted Code
1. **No global variables:** `set` at module scope is parsed but `exprTy` cannot be
   referenced from inside functions. Use HIR buffer slot 509 as a type-communication
   channel instead (`bw(hirBuf, 509, val)` / `br(hirBuf, 509)`).
2. **No forward declarations:** `function foo(...): int;` (with `;`) is not supported.
   Omit `;` — Ajeeb resolves function references across the entire file at runtime.
3. **`set` requires initializer:** `set x: int;` is invalid. Must write `set x: int = 0;`.
4. **Duplicate `set` in same function:** Multiple `set` with the same variable name
   (even in different if-branches) is a duplicate variable error. Declare once at the
   function top, use plain assignments (`x = value;`) in branches.
