# FORWARD DECLARATION AUDIT

## Pipeline Trace

```
collectFns (pass1.ajb:54)
  │
  ├─ Scans source for `function` keywords
  ├─ Calls addFn(nameStart) — stores only name + length
  ├─ Uses skipDepth to skip (...) parameter lists
  └─ Parameter info DISCARDED
  │
  ▼
emitFwdDecls (pass1.ajb:193)
  │
  ├─ Reads _fnnames.txt (names only)
  ├─ Emits: intptr_t fname();\n
  └─ No parameter types available
  │
  ▼
Generated C output
  intptr_t greet();          ← empty parens
  intptr_t greet(intptr_t name) { ... }  ← definition has params
```

## Root Cause

`collectFns` uses `skipDepth(src, pos, 40, 41)` to skip over `(...)` — parameter names and types are discarded. `emitFwdDecls` only has function names, so it hardcodes `()` for all declarations.

## GCC Error

```
error: conflicting types for 'isDigit'; have 'intptr_t(intptr_t)'
previous declaration of 'isDigit' with type 'intptr_t(void)'
```

GCC treats `intptr_t f();` as `intptr_t f(void)` in C11+, conflicting with the actual signature.

## Fix Applied

Removed `emitFwdDecls` and `collectFns` from the compilation pipeline in `compiler.ajb`:

- **Removed**: `collectFns(src, buf)` (pass 1)
- **Removed**: `emitFwdDecls(src, buf, out)` 
- **Removed**: `_fnnames.txt` file creation
- **Kept**: Built-in runtime declarations (getInt, setInt, parseExpr, parseStmt, etc.)

**Rationale**: The self-hosted compiler processes all imported files first (via the import handler in stmt.ajb:237). By the time the main file's functions are processed, all imported functions are already defined. No forward declarations needed.

## Verification

- `set x: int = 1;` → `intptr_t x = 1;` ✓
- `set name: string = "hello";` → `intptr_t name = (intptr_t)"hello";` ✓
- 6/6 regression tests pass
- No forward declarations in output (only built-in prototypes)
