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

## Key Bug Fix: appenInstr Arg Order for Goto (main.ajb)
**Root cause:** All `appendInstr(mirBuf, 6, target, 0, 0, 0)` calls placed the target block in `dst` (arg 3), but the C codegen reads `s1` (arg 4) for `goto block_{s1}`. Every Goto targeted block_0 regardless of intent, causing infinite loops.

**Fix:** Changed all 3 Goto emit sites to `appendInstr(mirBuf, 6, 0, target, 0, 0)`.

## Key Bug Fix: While Loop Exit Block vs Inner If-Else (main.ajb)
**Root cause:** `lowerWhile` hardcoded `exitBlk = blockCount + 2`. When the body contained if-else, `lowerIf` claimed block indices `{bc+1, bc+2}` for then/else, making `exitBlk` overlap with the then-block. The while condition would branch to the then-block instead of the exit on false.

**Fix:** In `lowerWhile`, after lowering the body (and the loop-back Goto), patch the Branch instruction's `s2` field with the actual exit block index (`blockCount`). This ensures the while condition's false-branch targets the correct exit block regardless of how many sub-blocks the body created.

## Key Bug Fix: LLVM Codegen String `==` Does Pointer Comparison
**Root cause:** The LLVM codegen's `Eq` operator (`icmp eq i64`) compares string POINTERS, not string contents. `substring` creates a new arena allocation, so `substring(src,1,8) == "package"` compares different addresses and returns false even when contents match.
**Fix:** Use `strcmp_ajeeb(str1, str2) == 0` instead of `str1 == str2` for all string equality checks in Ajeeb code compiled via the LLVM backend.

## Key Bug Fix: Parth Parser Slot Mapping
**Root cause:** `parseKeyValue` stores `(keyStart, keyLen, valStart, valLen)` at slots `(base, base+1, base+2, base+3)`, but `getConfigName/Version/Author` read value offsets at slots 0-1, 2-3, 4-5. Calling `parseKeyValue(src, lineStart, lineEnd, buf, 0)` for every package field overwrote key-value pairs — storing the field KEY name (e.g. "author") where `getConfigName` expected the value "my-project".
**Fix:** Inline value extraction for `[package]` and `[build]` sections: extract the value string directly (after `=`, quote-stripped), identify the key by name via `strcmp_ajeeb`, and store only value offset+length at the correct slot. Use `parseKeyValue` only for `[dependencies]` where both key (dep name) and value (version) are needed.
**Root cause:** The interpreter's `setInt`/`getInt` use the **string content** as the HashMap key for integer buffers. When `buf` (output) and `ast` (AST storage) are the same string object, writing to `buf` via `strSet` changes the string content, which changes the lookup key. Subsequent `getInt` calls fail (return 0) because the key no longer matches.

**Fix:** Always use separate strings for `buf` (output buffer) and `ast` (AST storage). Use `getOutbuf()` for output (character buffer) and `getStateBuf()` for AST (integer buffer). Never pass the same string as both `buf` and `ast` to codegen functions.

## Key Bug Fix: LLVM Runtime strSet Missing Null-Termination
**Root cause:** The C runtime's `strSet` writes a character at a given position but does not null-terminate. After `getOutbuf()` sets `buf[0] = '\0'`, writing `buf[0] = 'i'` leaves old data at positions 1..N. `strlen(buf)` scans past position 0 and finds old data, returning a stale length. Subsequent writes go to the wrong position.

**Fix:** In `ajeeb_runtime.c`, `strSet` now always writes `buf[i+1] = '\0'` after writing `buf[i] = c`. This ensures `strlen` returns the correct length after each sequential write.

## LLVM Codegen Runtime Functions
Known to codegen: `getInt`, `setInt`, `getStateBuf`, `getOutbuf`, `charCode`, `len`, `strSet`, `writeFile`, `writeAppend`, `writeByte`, `itoa`, `println`, `readFile`, `strcmp_ajeeb`, `str_concat`, `substring`, `indexOf`, `contains`, `toUpperCase`, `toLowerCase`, `trim`, `startsWith`, `endsWith`, `replace`, `array_to_string`, `exec`, `mkdir`.
NOT known: `chr`, `rdPos`, `wrPos` (evaluator-only).

## exec() / mkdir() — Ajeeb Runtime Functions
- `exec(cmd: string): int` — runs a shell command via `system()`, returns exit code
- `mkdir(path: string): int` — creates directory (including parents) via `mkdir -p`, returns exit code
- Both are 1-arg i64→i64 functions in LLVM codegen (`declare i64 @exec(i64)`, `declare i64 @mkdir(i64)`)
- C implementations in `runtime/ajeeb_runtime.c` (wrappers around `system()`)
- **Stale `build/runtime.o` must be deleted** after adding new runtime functions, or the linker won't find the new symbols

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
