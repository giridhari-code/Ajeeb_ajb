# Ajeeb Compiler — Agent Guide

## ⚠️ Stabilization Mode (June 2026)

**Releases temporarily disabled.** The release pipeline (`.github/workflows/release.yml.disabled`)
has been paused during stabilization. No GitHub Releases are created automatically.

**What still works:**
- Local source builds: `cd ajeebc && make native`
- All tests: `make test`
- Bootstrap verification: `make bootstrap`
- Source install: clone + `make native`

**What's disabled:**
- Automated GitHub Releases on tag push
- Cross-platform binary builds in CI
- Installer script downloads (until releases resume)

To re-enable releases, rename `.github/workflows/release.yml.disabled` back to `release.yml`.

See [Stabilization Plan](#stabilization-plan) below for the checklist.

## Quick Start
```bash
# Build (no Rust needed if binaries exist)
cd ajeebc && make native

# Run tests
make test

# Verify self-hosting
make bootstrap

# Or use Parth
parth build file.ajb
parth run file.ajb
parth test
```

## Bootstrap Self-hosting Check
```bash
bash tests/bootstrap_check.sh
```
This runs the verification pipeline:
1. `ajeebc` compiles `compiler/compiler.ajb` → native binary
2. Native binary re-compiles `compiler/compiler.ajb` → C code
3. Verify Gen1 and Gen2 produce identical output

## Ajeeb Interpreter Tests
Run individual .ajb files:
```bash
cd ajeebc && ./build/ajeebc tests/<test_file>.ajb --interpret
```
Key test files: test_simple, test_small, test_strings, test_math, test_for, test_if, test_while, test_array, cross_simple, compiler_test

## After Any Change
1. Run `cd ajeebc && make test`
2. Run `bash tests/bootstrap_check.sh`
3. Run a few key .ajb interpreter tests (e.g. `cross_simple.ajb`, `test_strings.ajb`)
4. Update runtime symlinks: `cp runtime/ajeeb_runtime.c /root/.ajeeb/bin/ajeeb_runtime.c`

## Diagnostic Output Cleanup (June 2026)
**Compiler (+ajeebc):** Silent by default. Pass `--verbose` / `-v` to see diagnostics (Lexer, Parser, Modules, Semantic, HIR, THIR, MIR, Backend, Codegen, Build summary).
**Runtime (`ajeeb_runtime.c`):** Silent by default. Set `AJEEB_PROFILE=1` to see [Perf] and [Ajeeb Runtime] diagnostics.
**Parth (build/run):** Silent by default. Pass `--quiet` / `-q` to suppress build status messages (Project, Entry, Compile, Build safal, etc.) during `parth build` / `parth run` / `parth run --quiet`. Use `parth run --quiet` for program-only output.

## Key Bug Fix: appenInstr Arg Order for Goto (main.ajb)
**Root cause:** All `appendInstr(mirBuf, 6, target, 0, 0, 0)` calls placed the target block in `dst` (arg 3), but the C codegen reads `s1` (arg 4) for `goto block_{s1}`. Every Goto targeted block_0 regardless of intent, causing infinite loops.

**Fix:** Changed all 3 Goto emit sites to `appendInstr(mirBuf, 6, 0, target, 0, 0)`.

## Key Bug Fix: While Loop Exit Block vs Inner If-Else (main.ajb)
**Root cause:** `lowerWhile` hardcoded `exitBlk = blockCount + 2`. When the body contained if-else, `lowerIf` claimed block indices `{bc+1, bc+2}` for then/else, making `exitBlk` overlap with the then-block. The while condition would branch to the then-block instead of the exit on false.

**Fix:** In `lowerWhile`, after lowering the body (and the loop-back Goto), patch the Branch instruction's `s2` field with the actual exit block index (`blockCount`). This ensures the while condition's false-branch targets the correct exit block regardless of how many sub-blocks the body created.

## Key Bug Fix: LLVM Codegen String `==` Does Pointer Comparison
**Root cause:** The LLVM codegen's `Eq` operator (`icmp eq i64`) compares string POINTERS, not string contents. `substring` creates a new arena allocation, so `substring(src,1,8) == "package"` compares different addresses and returns false even when contents match.
**Fix:** Use `strcmp_ajeeb(str1, str2) == 0` instead of `str1 == str2` for all string equality checks in Ajeeb code compiled via the LLVM backend.

## Key Bug Fix: Import Handler savePos Clobbered by nextTok (stmt.ajb)
**Root cause:** The import handler used `savePos(buf)`/`restorePos(buf)` to save/restore the lexer position across recursive parses. But `savePos` writes to buffer slot 8 (byte offset 8), and `nextTok` in `lexer.ajb` also uses `savePos`/`restorePos` internally for backtracking (lines 113-157). Every `nextTok` call during the recursive parse of the imported file overwrote the import handler's saved position. After restore, the position pointed into the middle of the next statement (e.g., at `int` in `function main(): int {`), causing the parser to enter the wrong branch and eventually hit the empty `}` handler (t==38) which is a no-op, creating an infinite loop.

**Additional sub-bug:** `identStr()` returns `getOutbuf()`, a shared buffer. After reading the module name, `getOutbuf()` is called again for `scratchO`, clobbering the module name. The `savedModName = str_concat(modName, "")` copy fixed this.

**Additional sub-bug:** After `restorePos` + `wrSrc`, `tokType(buf)` is stale (0 from EOF of the imported file). The main loop `while (tokType(buf) != 0)` would exit immediately after the first import.

**Fix (3 parts):**
1. Use `savedModName = str_concat(modName, "")` before `getOutbuf()` clobbers it
2. Use `wrB(buf, 900, rdPos(buf))` / `wrPos(buf, rdB(buf, 900))` instead of `savePos`/`restorePos` — slot 900 is not used by `nextTok`
3. After restore, call `nextTok(src, buf)` to re-sync the token state from the original source

**Verification:** Single import works ✓, multiple imports work ✓, `compiler.ajb` produces 2661 lines / 51 functions ✓, bootstrap check passes ✓.

## compiler.ajb Split — Phase 7 Complete
**compiler.ajb (1279L) → 6 files (max 364L):**

| File | Lines | Contains |
|---|---|---|
| `compiler/compiler.ajb` | 115 | Imports, helpers (rdB/wrB), main() |
| `compiler/lexer.ajb` | 228 | nextTok, skipWS, matchKwd, readIdent/Number/String, setTok/tokType, identStr |
| `compiler/emit.ajb` | 33 | emitStr, emitI, emitEscapedStr |
| `compiler/expr.ajb` | 297 | parsePrimary, parseUnary, parseMul/Add/Cmp/Eq/And/Or/Assign, parseExpr |
| `compiler/stmt.ajb` | 364 | parseType, parseStmt (all statement types incl. import/class) |
| `compiler/pass1.ajb` | 211 | addFn, skipDepth, collectFns, emitFwdDecls |

**Verification:** `cargo test` ✓, `bash tests/bootstrap_check.sh` ✓ (MIR/LLVM pipeline, 78KB binary).

**Critical design:** The import statements (`import lexer;`) are resolved at TWO levels simultaneously:
1. **Rust ModuleLoader** (Step 1) — flattens all function definitions from all files into evaluator
2. **Ajeeb-level parseStmt import handler** (Step 3) — recursively processes imported files during C codegen

Both levels must handle the same `import ident;` syntax identically. The path construction in both handlers uses the `compiler/` prefix.

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
Added: `allocBuf` — `declare i64 @allocBuf(i64)`, allocates N+1 zero-initialized bytes from arena.

## exec() / mkdir() — Ajeeb Runtime Functions
- `exec(cmd: string): int` — runs a shell command via `system()`, returns exit code
- `mkdir(path: string): int` — creates directory (including parents) via `mkdir -p`, returns exit code
- Both are 1-arg i64→i64 functions in LLVM codegen (`declare i64 @exec(i64)`, `declare i64 @mkdir(i64)`)
- C implementations in `runtime/ajeeb_runtime.c` (wrappers around `system()`)
- **Stale `build/runtime.o` must be deleted** after adding new runtime functions, or the linker won't find the new symbols

## Monster File Split — Phase 6 Complete
All 3 compiler-core Rust files >1000 LOC have been split into modular directories:

- **`cache.rs`** (1050L) → `cache/` (2 files: mod.rs 179L, serialize.rs 877L)
- **`eval.rs`** (1872L) → `eval/` (5 files: mod.rs 293L, builtins.rs 988L, expr.rs 455L, stmt.rs 111L, functions.rs 46L)
- **`parser.rs`** (1930L) → `parser/` (7 files: mod.rs 223L, decls.rs 687L, expr.rs 686L, stmt.rs 173L, types.rs 79L, generics.rs 66L, patterns.rs 62L)

Pre-existing monster files NOT split (deferred): `stage2/src/hir_lower.ajb` (1103L), `stage3/src/main.ajb` (1066L), `parth/src/main.rs` (1563L), `parth/src/registry.rs` (1454L).

Verification: `cargo test` ✓, `bash tests/bootstrap_check.sh` ✓ (bootstrap success, 77KB binary).

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

## Standard Library (packages/ajeeb-std/)

| File | Description |
|------|-------------|
| `packages/ajeeb-std/io.ajb` | Input/output — print, readLine, readFileLines, writeFileLines |
| `packages/ajeeb-std/math.ajb` | Math — abs, max, min, pow, factorial, gcd, lcm, isPrime, clamp |
| `packages/ajeeb-std/string.ajb` | String utilities — strEq, strEmpty, strRepeat, strReverse, strPadLeft/Right, strJoin, strCount |
| `packages/ajeeb-std/array.ajb` | Array utilities — arraySum, arrayMax, arrayMin, arrayContains, arrayReverse, arraySort |
| `packages/ajeeb-std/fs.ajb` | File system — fileExists, appendLine, copyFile, mkdirP, listDir |
| `packages/ajeeb-std/result.ajb` | Result/Option types — ok, err, some, none, isOk, isErr, isSome, isNone, unwrap |
| `packages/ajeeb-std/collections.ajb` | Data structures — Stack (push/pop/peek), Queue (enqueue/dequeue/peek) |

**Use:** `import math;` (resolves to `packages/ajeeb-std/math.ajb` via built-in `./packages/ajeeb-std` search path).

**Notes:**
- `struct`-based (not `class`) — `class` has a semantic analyzer bug (first pass doesn't register class in `struct_defs`)
- `len()` is string-only; arrays use `arr_len()`
- LLVM codegen has `__index` limitation for non-constant index expressions
- Test files: `tests/test_std_math.ajb`, `tests/test_std_string.ajb`, `tests/test_std_array.ajb`
- Run tests: `cargo run -p ajeeb-compiler --bin ajeeb_compiler -- --interpret tests/test_std_<module>.ajb`

## Stabilization Plan

### Phase 1: Compiler Bugs
- [ ] Fix `__index` limitation for non-constant array index expressions
- [ ] Fix `class` semantic analyzer bug (first pass doesn't register class in `struct_defs`)
- [ ] Fix output truncation in C codegen for large files (~4324+ lines)
- [ ] Verify self-hosting: ajeebc compiles compiler.ajb → native binary
- [ ] Run full test suite: `make test`

### Phase 2: Parth Bugs
- [ ] Verify `parth init` / `parth build` / `parth run` pipeline end-to-end
- [ ] Test dependency resolution with standard library packages
- [ ] Cross-platform parity: Linux aarch64, x86_64, macOS arm64, x86_64, Windows x86_64

### Phase 3: Installer Bugs
- [ ] Test `install.sh` on fresh Linux (no existing ajeeb installation)
- [ ] Test `install.sh` on fresh macOS (arm64 + x86_64)
- [ ] Test `install.ps1` on fresh Windows (PowerShell)
- [ ] Verify SHA256 checksum verification works
- [ ] Verify PATH setup in `.bashrc` / `.zshrc`

### Phase 4: Fresh-Machine Testing
- [ ] Clone repo on clean Ubuntu 24.04 → `make native` → `make test`
- [ ] Clone repo on clean macOS 15 (M1) → `make native` → `make test`
- [ ] Clone repo on clean Windows (MSYS2) → `make native` → `make test`
- [ ] Verify Gen0 bootstrap binary runs on aarch64

### Phase 5: ARM64 Validation
- [ ] `make native` on aarch64 Linux (native)
- [ ] `make native` on macOS ARM64 (native)
- [ ] Cross-compile: Gen0 aarch64 → `llc --march=x86-64` on x86_64 host
- [ ] Cross-compile: Gen0 aarch64 → `llc --march=x86-64` on macOS x86_64

### Phase 6: Self-Hosted Toolchain Validation
- [ ] Gen0 compiles compiler.ajb → Gen1
- [ ] Gen1 re-compiles compiler.ajb → Gen2
- [ ] Gen1 and Gen2 produce identical output
- [ ] `make bootstrap-full` passes on all platforms

### Re-enabling Releases
1. All checkboxes above must be checked
2. Rename `release.yml.disabled` → `release.yml`
3. Delete old tags and releases (start fresh at v0.1.0)
4. Push tag `v0.1.0` to trigger first release
5. Verify all 5 platform binaries in GitHub Release
6. Verify installer downloads work end-to-end
