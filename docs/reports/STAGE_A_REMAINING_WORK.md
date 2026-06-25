# STAGE A REMAINING WORK

## Definition of "Stage A Complete"

Stage A is complete when the self-hosted compiler can compile ALL .ajb files in the
codebase that the Rust compiler can compile, producing correct C output that compiles
with GCC and runs correctly.

## Current Status

- ✅ Self-hosted compiler compiles itself (bootstrap verified)
- ✅ Self-hosted compiler compiles standard library (io, math, string, array, fs, result, collections)
- ✅ Self-hosted compiler compiles Parth (package manager)
- ✅ Self-hosted compiler compiles test files
- ❌ Self-hosted compiler CANNOT compile ajeeb-web, ajeeb-db, ajeeb-json, ajeeb-log
  (these use structs, pub, global variables, class fields)

## Remaining Work by Priority

### P0 — Required for Stage A Completion

These features are used by ecosystem packages and MUST be implemented for parity.

#### 1. Class Field Declarations
**Used by:** result.ajb, collections.ajb, ajeeb-web/mod.ajb
**Current behavior:** stmt.ajb line 241 skips non-function members in class bodies
**Fix:** In the class body handler, when a non-function token is encountered, parse
it as a field declaration and emit a C struct field.

**Estimated effort:** Small (add ~20 lines to stmt.ajb class handler)

**C emission:** Class becomes a C struct with fields:
```c
typedef struct {
    intptr_t field1;
    intptr_t field2;
} ClassName;
```

#### 2. Struct Declarations
**Used by:** ajeeb-db/mod.ajb, ajeeb-web/mod.ajb, test_e2e.ajb
**Current behavior:** Not tokenized, not parsed
**Fix:** Add `struct` keyword to lexer (new token type), add struct parsing to
stmt.ajb, emit as C struct with typedef.

**Estimated effort:** Medium (add ~50 lines: lexer keyword, stmt handler, C emission)

**C emission:**
```c
typedef struct {
    intptr_t field1;
    intptr_t field2;
} StructName;
```

#### 3. Struct Literal Construction
**Used by:** ajeeb-db, ajeeb-web, test_e2e (e.g., `Route { method: "GET", path: path }`)
**Current behavior:** Not parsed
**Fix:** In parsePrimary, handle `StructName { field: value, ... }` syntax and emit
C compound literal: `(StructName){ val1, val2 }`

**Estimated effort:** Medium (add ~30 lines to expr.ajb parsePrimary)

#### 4. `pub` Access Modifier
**Used by:** ajeeb-json, ajeeb-db, ajeeb-web, ajeeb-log
**Current behavior:** Not tokenized
**Fix:** Add `pub` keyword to lexer, skip it during parsing (no semantic effect in
single-pass C codegen — all functions are global in C).

**Estimated effort:** Tiny (add 1 keyword to lexer, skip in stmt.ajb)

#### 5. Global Variables (Module Scope)
**Used by:** ajeeb-log/mod.ajb, test_e2e.ajb
**Current behavior:** `main()` only processes function/class/import at top level.
Module-scope `set` falls through to expression statement handler.
**Fix:** In the main parse loop, when `set` is encountered at top level, emit it
as a C global variable declaration.

**Estimated effort:** Small (add ~10 lines to main loop or stmt handler)

#### 6. Forward Declarations for User Functions
**Current behavior:** `emitFwdDecls` exists in pass1.ajb but is never called from main().
Functions that call other functions defined later in the file will produce GCC
"implicit declaration" errors.
**Fix:** Call `collectFns` + `emitFwdDecls` at the start of `main()` before the
parse loop, or emit forward declarations inline.

**Estimated effort:** Small (add 2 lines to main())

### P1 — Required for Ecosystem Completeness

These features are not blocking the core compiler but are needed for full ecosystem
support.

#### 7. `%` Modulo Operator
**Used by:** Not currently used, but standard in most languages
**Fix:** Add `%` to lexer, add to parseMul in expr.ajb, emit `a % b` in C.

**Estimated effort:** Tiny (~5 lines)

#### 8. `**` Power Operator
**Used by:** Not currently used
**Fix:** Add `**` to lexer, add to parseMul or new precedence level, emit `pow(a, b)`.

**Estimated effort:** Small (~10 lines)

#### 9. `break` and `continue` Dispatch
**Current behavior:** Tokenized but not dispatched. Works by accident (C keywords).
**Fix:** Add explicit dispatch in parseStmt for tokens 45 and 46, emit `break;` and
`continue;`.

**Estimated effort:** Tiny (~6 lines)

### P2 — Future Language Features (Not Required for Stage A)

These features exist in the Rust compiler but are NOT used by any .ajb file.
They can be deferred to post-Stage-A development.

| Feature | Effort | Priority |
|---------|--------|----------|
| `enum` declarations | Large | P2 |
| `trait` declarations | Large | P2 |
| `impl` blocks | Large | P2 |
| Generics `<T>` | Very Large | P2 |
| Pattern matching `match` | Large | P2 |
| Closures / lambdas | Large | P2 |
| `type` aliases | Small | P2 |
| `where` clauses | Medium | P2 |
| Error handling `try`/`catch` | Large | P2 |
| `async`/`await` | Very Large | P2 |
| `float`/`double` types | Medium | P2 |
| Inheritance / `extends` | Medium | P2 |
| Constructors / `init` | Medium | P2 |
| Ternary `?:` | Small | P2 |
| `for` loops (used by any file) | Already implemented | — |

## Minimum Viable Stage A

The minimum work to declare Stage A complete:

1. **Class fields** (P0-1) — required by result.ajb, collections.ajb
2. **Struct declarations** (P0-2) — required by ajeeb-db, ajeeb-web
3. **Struct literals** (P0-3) — required by ajeeb-db, ajeeb-web
4. **`pub` modifier** (P0-4) — required by ajeeb-json, ajeeb-db, ajeeb-web, ajeeb-log
5. **Global variables** (P0-5) — required by ajeeb-log, test_e2e
6. **Forward declarations** (P0-6) — required for any file with forward references

**Estimated total P0 effort:** ~2-3 hours of focused work

After P0 is complete, the self-hosted compiler should be able to compile ALL .ajb
files that the Rust compiler compiles, including ajeeb-web, ajeeb-db, ajeeb-json,
and ajeeb-log.

## Verification Plan

After implementing P0 features:

1. `cargo test` — Rust compiler tests still pass
2. `bootstrap_check.sh` — self-hosting still works
3. Self-hosted compiler compiles each package:
   - `./build/compiler packages/ajeeb-log/mod.ajb build/test_log.c && gcc build/test_log.c ...`
   - `./build/compiler packages/ajeeb-db/mod.ajb build/test_db.c && gcc build/test_db.c ...`
   - `./build/compiler packages/ajeeb-web/mod.ajb build/test_web.c && gcc build/test_web.c ...`
4. All compiled binaries run correctly
5. STAGE_A_COMPLETE.md written
