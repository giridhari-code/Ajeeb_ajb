# STAGE A FINAL GAP REPORT

## Audit Date: 2026-06-24

## Methodology

For each P0-4/5/6, checked:
- A. Used by any .ajb file? (grep across entire codebase)
- B. Required for self-hosting? (compiler.ajb compilation)
- C. Required by ajeeb-db?
- D. Required by ajeeb-web?
- E. Already implemented accidentally?
- F. Can be deferred to Stage B?

Also compiled each package with the self-hosted compiler + GCC to identify real errors.

---

## Feature Audit Table

| Feature | A. Used | B. Self-host | C. ajeeb-db | D. ajeeb-web | E. Accidental? | F. Deferrable? | Decision |
|---------|---------|-------------|-------------|-------------|----------------|----------------|----------|
| **P0-4: pub modifier** | ✅ 113 occurrences in 13 files (ajeeb-lang/std: 83, packages: 30) | ❌ compiler never uses pub | ✅ 8 functions | ❌ not used | ❌ not tokenized | ❌ blocks package compilation | **REQUIRED** |
| **P0-5: global variables** | ✅ ajeeb-log/mod.ajb (2 set + 1 const), ajeebc/tests (15+), tests/test_counter.ajb | ❌ compiler avoids globals | ❌ no globals | ❌ no globals | ✅ **ALREADY WORKS** | ✅ already works | **ALREADY DONE** |
| **P0-6: forward declarations** | ✅ ajeeb-web/mod.ajb (20+ call sites), ajeeb-db (2 sites), ajeeb-lang/std/json.ajb (2 sites) | ❌ compiler uses import ordering | ✅ db_select→db_escape_table | ✅ web_listen→web_handle_request, mutual recursion web_json_val↔web_json_stringify_obj | ❌ collectFns/emitFwdDecls exist but are dead code | ❌ blocks package compilation | **REQUIRED** |

---

## Detailed Findings

### P0-4: pub Access Modifier

**Status: REQUIRED — Simple fix (~10 lines)**

**Used by:**
- `ajeeb-lang/std/*.ajb` — 83 functions use `pub` (math, io, string, fs, collections, array, result, option, json, test, process, time, path)
- `packages/ajeeb-db/mod.ajb` — 8 functions
- `packages/ajeeb-json/mod.ajb` — 8 functions
- `packages/ajeeb-log/mod.ajb` — 7 functions
- `packages/ajeeb-http/mod.ajb` — 7 functions

**NOT used by:**
- `compiler/` — zero pub usage
- `packages/ajeeb-web/mod.ajb` — zero pub usage
- `tests/` — zero pub usage

**Current behavior:** `pub` is not tokenized (no keyword in lexer.ajb). Treated as identifier (token type 1). Falls through to the `else` clause in parseStmt (stmt.ajb:451), which calls `parseExpr` and emits `pub;` as a C statement. GCC error: `type defaults to 'int' in declaration of 'pub'`.

**Fix:**
1. Add `pub` keyword to lexer.ajb (after `struct`): `if (matchKwd(src, buf, "pub") == 1) { setTok(buf, 50, 0, 0, 0); ... }`
2. Add skip in stmt.ajb parseStmt: `else if (t == 50) { nextTok(src, buf); parseStmt(src, buf, out); }` — skip `pub` and re-parse the next statement

**Self-hosting:** Not required — compiler never uses `pub`.

**Verification:** After fix, ajeeb-db, ajeeb-json, ajeeb-log, ajeeb-http should compile without `pub;` errors.

---

### P0-5: Global Variables (Module Scope)

**Status: ALREADY IMPLEMENTED — No work needed**

**Verification:** Compiled `packages/ajeeb-log/mod.ajb` with self-hosted compiler. Generated C output contains:
```c
intptr_t log_current_level = 1;
intptr_t log_file_path = (intptr_t)"";
```
These are valid C global variable declarations. The self-hosted compiler's parseStmt `set` handler (stmt.ajb:58) processes module-scope `set` declarations identically to function-scope ones, which produces correct C globals.

**Remaining issues with ajeeb-log (not P0-5):**
1. `pub;` statements (P0-4)
2. `const` array literal `LOG_LEVELS = ["DEBUG", ...]` emitted as `const intptr_t LOG_LEVELS = {...}` — invalid C (scalar initialized with array initializer, then indexed). This is a separate issue from global variables — it's about array literal emission for `const` arrays.

---

### P0-6: Forward Declarations

**Status: REQUIRED — Medium fix (~30 lines)**

**Used by:**
- `packages/ajeeb-web/mod.ajb` — 20+ forward reference call sites (web_listen→web_handle_request, web_handle_request→web_run_middleware/web_dispatch_route, mutual recursion web_json_val↔web_json_stringify_obj/arr, etc.)
- `packages/ajeeb-db/mod.ajb` — 2 sites (db_select/db_insert→db_escape_table)
- `ajeeb-lang/std/json.ajb` — 2 sites (json_parse_array_val/json_parse_object_val→json_parse_value)

**NOT used by:**
- `compiler/` — all functions defined in dependency order (imported files emit first, then main file)
- Any test file

**Current behavior:** The self-hosted compiler (compiler.ajb) does NOT emit forward declarations for user functions. It only emits built-in runtime declarations (getInt, setInt, etc.). Functions are emitted inline as they are parsed. If function A calls function B and B is defined after A, the generated C has an implicit declaration error.

**Existing dead code:** `collectFns` + `emitFwdDecls` in pass1.ajb exist but are never called from main(). They also have a known limitation: they discard parameter types during scanning, producing `intptr_t f();` declarations that conflict with actual signatures in GCC C11+.

**Fix options:**

Option A (Minimal — inline forward decls):
1. Before the main parse loop in compiler.ajb, do a pre-scan pass to collect all function names
2. Emit `intptr_t fname();` forward declarations at the top of the C output
3. This has the "conflicting types" GCC warning but works in practice with `-w` or `-fpermissive`

Option B (Correct — two-pass):
1. First pass: scan source for `function` keywords, extract name + param count + param types
2. Emit correct forward declarations: `intptr_t fname(intptr_t p1, intptr_t p2);`
3. Second pass: parse and emit function bodies
4. This is ~50 lines but produces clean GCC output

**Self-hosting:** Not required — compiler uses import ordering that avoids forward references.

**Verification:** After fix, ajeeb-web/mod.ajb should compile and GCC should not produce "implicit declaration of function" errors.

---

## Discovered Issues (Not P0)

These are real issues found during the audit but NOT part of P0-4/5/6:

1. **Custom type names in struct fields** — `String`, `Bool`, `Int`, `Array`, `ClassInstance` in struct field declarations are emitted as-is instead of mapping to `intptr_t`. GCC error: `unknown type name 'String'`. Affects ajeeb-db, ajeeb-web, ajeeb-log.

2. **`const` array literals** — `const LOG_LEVELS = ["DEBUG", ...]` emitted as `const intptr_t LOG_LEVELS = {...}` which is invalid C (scalar initializer with multiple values). Affects ajeeb-log.

3. **Struct field type mapping** — Related to #1 but broader: the struct handler (stmt.ajb:418-447) hardcodes `intptr_t` for all fields, but doesn't map `String`→`intptr_t`, `Bool`→`intptr_t`, `Array`→`intptr_t`, `Int`→`intptr_t`.

---

## Decision Matrix

| Scenario | Required Items | Estimated Effort |
|----------|---------------|-----------------|
| Compile ajeeb-db | P0-4 (pub) + type mapping fix | ~15 lines |
| Compile ajeeb-log | P0-4 (pub) + const array fix | ~15 lines |
| Compile ajeeb-web | P0-6 (fwd decls) + type mapping fix | ~50 lines |
| Compile ajeeb-json | P0-4 (pub) only | ~10 lines |
| Compile ajeeb-http | P0-4 (pub) + P0-6 (fwd decls) | ~40 lines |
| Self-hosting compiler | **NOTHING** — already works | 0 |

---

## OPTION 1 Assessment: Is Stage A Actually Complete?

**Self-hosting: YES** — The self-hosted compiler compiles itself, all core tests pass, method dispatch works.

**Full package compilation: NO** — The self-hosted compiler cannot compile ajeeb-db, ajeeb-web, ajeeb-json, ajeeb-log, ajeeb-http due to:
1. `pub` keyword not recognized (P0-4)
2. Forward references within single files (P0-6)
3. Custom type names in struct fields (discovered issue)

---

## OPTION 2: Remaining Stage A Blockers

### Must-fix blockers (to compile all ecosystem packages):

1. **P0-4: pub modifier** — ~10 lines. Add keyword to lexer, skip in parser.
2. **P0-6: forward declarations** — ~30-50 lines. Add pre-scan pass or inline forward decls.
3. **Type mapping for struct fields** — ~10 lines. Map `String`/`Bool`/`Int`/`Array`/`ClassInstance` to `intptr_t` in struct field emission.

### Nice-to-fix (not blocking compilation):

4. **`const` array literal emission** — ~20 lines. Need to emit as `static const char* arr[] = {...}` instead of `const intptr_t arr = {...}`.

### Already done (no work needed):

5. **P0-5: global variables** — Already works.
6. **P0-1: class fields** — Already works (completed earlier).
7. **P0-2: struct declarations** — Already works (completed earlier).
8. **P0-3: method dispatch** — Already works (completed earlier).

---

## Recommended Path

**Minimum to declare Stage A complete:**
1. Implement P0-4 (pub) — 10 lines
2. Implement P0-6 (forward declarations) — 30 lines
3. Fix type mapping for struct fields — 10 lines
4. Verify: compile ajeeb-db, ajeeb-web, ajeeb-json, ajeeb-log with self-hosted compiler
5. Write STAGE_A_COMPLETE.md

**Total estimated effort: ~50 lines of code, 1-2 hours of focused work.**
