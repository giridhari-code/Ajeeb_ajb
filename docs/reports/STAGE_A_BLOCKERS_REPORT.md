# STAGE A BLOCKERS REPORT

## Date: 2026-06-24

## Changes Made

### 1. P0-4: pub Access Modifier

**Files changed:**
- `ajeebc/compiler/lexer.ajb` — Added `pub` keyword as token type 50 (line 152-153)
- `ajeebc/compiler/stmt.ajb` — Added `pub` skip handler: when t==50, advance token and re-parse (line 449-452). Also added `pub` skip inside class body loop (line 311).
- `ajeebc/compiler/pass1.ajb` — Added `pub` skip in `collectFns` main loop and class body scanner (lines 93, 127)

**What it does:**
- `pub function foo()` → parsed as `function foo()` (pub skipped)
- `pub class Bar { ... }` → parsed as `class Bar { ... }` (pub skipped)
- `pub function method(self)` inside class → parsed as `function method(self)` (pub skipped)

**Verification:**
- `pub function`, `pub class`, `pub` inside class body all compile correctly
- Generated C has no `pub;` statements
- All 33 `pub` usages in ajeeb-db, ajeeb-json, ajeeb-log, ajeeb-http compile without pub-related errors

### 2. P0-6: User-Function Forward Declarations

**Files changed:**
- `ajeebc/compiler/compiler.ajb` — Added `scanForwardDecls()` function (token-based forward declaration scanner) and integrated it as PASS 1 before the main parse loop

**What it does:**
- Scans source using the token system (not raw character access like the old `collectFns`)
- For each `function` declaration, emits `intptr_t fname(intptr_t p1, intptr_t p2);` with correct parameter names and types
- For `main()`, emits `int main(int argc, char** argv);`
- Handles `pub` prefix, `class`/`struct` bodies, `import` statements, `set`/`const` declarations, `if`/`else`/`while`/`for` blocks

**Verification:**
- `function b(): int { return a(); } function a(): int { return 42; }` compiles and runs correctly (output: 42)
- `function add(x: int, y: int): int { return x + y; }` emits `intptr_t add(intptr_t x, intptr_t y);` forward declaration
- Self-hosting compiler rebuilds successfully
- All 7 core regression tests pass

### 3. Struct Field Type Mapping

**Files changed:**
- `ajeebc/compiler/stmt.ajb` — Added type name mapping in struct field handler (line 434-435)

**What it does:**
- Maps `String`, `Bool`, `Int`, `Array`, `ClassInstance` to `intptr_t` in struct field declarations
- Previously, these were emitted as-is (e.g., `String name;`) which GCC rejected as unknown types

**Verification:**
- `struct User { name: String, active: Bool, score: Int, tags: Array }` emits `intptr_t name; intptr_t active; intptr_t score; intptr_t tags;`
- Generated C compiles without type errors

## Tests Run

### Core Regression Tests (all pass)
| Test | Output | Status |
|------|--------|--------|
| test_simple | Hello World | ✅ |
| test_for | 0,1,2,4,5 | ✅ |
| test_if | bada hai | ✅ |
| test_while | 0,1,2 | ✅ |
| cross_simple | sum: 30, factorial(5): 120, Hello World | ✅ |
| struct_basic | Ajeeb, 1 | ✅ |
| struct_literal | 10, 20 | ✅ |

### Ecosystem Package Compilation
| Package | pub errors | Other errors | Status |
|---------|-----------|-------------|--------|
| ajeeb-db | 0 | Missing runtime (sqlite_*, log_*) | ✅ No pub/type errors |
| ajeeb-log | 0 | const array indexing | ✅ No pub/type errors |
| ajeeb-json | 0 | struct literal syntax | ✅ No pub/type errors |
| ajeeb-web | 0 | struct literal, missing runtime | ✅ No pub/type errors |

### Self-Hosting
- Rust interpreter rebuilds compiler: ✅
- Self-hosted binary compiles test files: ✅
- Forward reference test (b() calls a() defined later): ✅ Output 42

## Remaining Pre-Existing Issues (Not Stage A Blockers)

1. **`const` array literals** — `const LOG_LEVELS = ["DEBUG", ...]` emits as `const intptr_t LOG_LEVELS = {...}` which is invalid C. This is a pre-existing issue unrelated to P0-4/5/6.

2. **Struct literal in return statements** — `return {200, {...}, body}` is not valid C for non-struct return types. Pre-existing issue.

3. **Missing runtime functions** — `sqlite_open`, `log_info`, `arr_len`, `json_stringify` etc. are not in the C runtime. These are external package dependencies, not compiler bugs.

## Files Changed Summary

| File | Lines added | Purpose |
|------|------------|---------|
| `ajeebc/compiler/lexer.ajb` | 2 | `pub` keyword (token 50) |
| `ajeebc/compiler/stmt.ajb` | 5 | `pub` skip + struct type mapping |
| `ajeebc/compiler/compiler.ajb` | ~170 | `scanForwardDecls()` function + integration |
| `ajeebc/compiler/pass1.ajb` | 2 | `pub` skip in collectFns |
