# Ajeeb v0.2 Changelog

**Release Date:** 2026-06-22
**Version:** v0.2.4 (current)

---

## New Features

### Multi-Platform Support (v0.2.3+)
- CI builds for **linux-x86_64**, **linux-aarch64**, **macos-arm64**, **macos-x86_64**, and **windows-x86_64**
- ARM64 (aarch64) builds with install.sh source fallback
- Cross-platform C runtime (`ajeeb_runtime.c`) with `#ifdef` guards for Linux, macOS, Windows

### Parth Package Manager Refactor (v0.2.3)
- 42 commands rewritten in Ajeeb (`parth.ajb`) — removes Cargo dependency for self-hosted version
- `entry` field support in package metadata
- `generate-lockfile` command
- Rust version retains 35 commands; Ajeeb version ships 7 core commands

### Standard Library (v0.2.2)
- 7 modules, 418 lines total in `packages/ajeeb-std/`:
  - `io.ajb` — print, readLine, readFileLines, writeFileLines
  - `math.ajb` — abs, max, min, pow, factorial, gcd, lcm, isPrime, clamp
  - `string.ajb` — strEq, strEmpty, strRepeat, strReverse, strPadLeft/Right, strJoin, strCount
  - `array.ajb` — arraySum, arrayMax, arrayMin, arrayContains, arrayReverse, arraySort
  - `fs.ajb` — fileExists, appendLine, copyFile, mkdirP, listDir
  - `result.ajb` — ok, err, some, none, isOk, isErr, isSome, isNone, unwrap
  - `collections.ajb` — Stack (push/pop/peek), Queue (enqueue/dequeue/peek)
- Module import system with file-based resolution

### Compiler Improvements
- Full pipeline: HIR → THIR → MIR → LLVM IR → native
- C codegen fallback
- Braceless `if`/`while`/`for`/`else` support via `parseBody()`
- Self-hosting verified via Rust → LLVM → native pipeline

---

## Bug Fixes

### Runtime Fixes
- **`chr()` fix** — returned string pointer instead of char code integer
- **`strSet` null-termination** — C runtime now writes null terminator after each character write
- **`println(str_concat(...))` fix** — no longer prints "true"/"false" instead of string content
- **`indexOf` 3rd arg** — start position parameter fixed, unsigned underflow patched
- **`substring` string comparison** — LLVM codegen now uses `strcmp_ajeeb(str1, str2) == 0` instead of pointer comparison

### Compiler Fixes
- **Operator precedence** — audit fixes for correct evaluation order
- **Break/continue** — control flow statements in loops
- **Type safety** — dead block elimination, improved error reporting
- **Block comment EOF safety** — unterminated block comments handled correctly
- **String temp tracking** — prevents dangling references during codegen
- **`println` runtime** — correct output via C runtime path
- **C header declarations** — proper extern declarations for runtime functions

### Build Fixes
- `cargo build` instead of `make rustc` (multi-file crates require cargo)
- `install.sh` source build uses `cargo` for correct compilation
- Show `cargo build` errors instead of swallowing them
- Dereference `&i64` in `n.max(0)` — Rust build error E0308/E0606

---

## Breaking Changes

None in v0.2 series. All changes are additive or internal fixes.

---

## Known Issues

See [KNOWN_ISSUES.md](KNOWN_ISSUES.md) for the full list. Highlights:

1. `println(str_concat(...))` may still have edge cases with 3+ nested calls
2. `set` requires initializer (`set x: int = 0;` not `set x: int;`)
3. No forward declarations in Ajeeb
4. `class` has semantic analyzer bug (first pass doesn't register in `struct_defs`)
5. LLVM codegen `__index` limitation for non-constant index expressions
6. Ajeeb-level Parth has 7 commands vs Rust version's 35
7. Windows bootstrap check not verified in CI
8. No CI test step in release workflow

---

## Migration Notes

### From v0.1 to v0.2

1. **Standard library import path changed.** Use `import math;` (resolves to `packages/ajeeb-std/math.ajb`).
2. **`set` syntax.** Always provide an initializer: `set x: int = 0;`
3. **String equality.** `==` on strings now uses content comparison (strcmp), not pointer comparison.
4. **Parth CLI.** Single entry point: `parth` CLI with `ajeebc` and `parthi` sub-binaries.

### Upgrade Steps

```bash
# Update to v0.2.4
git pull origin main
cargo build --release
# Verify bootstrap
make bootstrap
# Run tests
cargo test
```

---

## Codebase Metrics (v0.2.4)

| Component | Files | Lines |
|-----------|-------|-------|
| Rust compiler (`ajeeb-compiler`) | 41 | 13,568 |
| LLVM codegen | 8 | 2,641 |
| C codegen | 1 | 417 |
| Runtime (`ajeeb_runtime.c`) | 1 | 1,452 |
| Compiler .ajb (self-hosted) | 7 | 2,885 |
| Standard library | 7 | 418 |
| Parth Rust | 5 | 4,601 |
| Parth Ajeeb | 5 | 840 |
| Parth tests | 13 | 383 |
| **Total** | **88** | **27,205** |
