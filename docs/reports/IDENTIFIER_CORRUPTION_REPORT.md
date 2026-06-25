# IDENTIFIER CORRUPTION REPORT

## Symptom

`set name: type = expr` emitted `0 = expr` instead of `name = expr`:

```c
// BEFORE (broken)
intptr_t x;
0 = 1;
intptr_t name;
0 = (intptr_t)"hello";
```

## Failing Example

```
set x: int = 1;
set name: string = "hello";
```

Generated invalid C where variable identifiers were replaced with the literal `0`.

## Pipeline Trace (Lexer → Parser → C codegen)

The self-hosted compiler is a direct parser → C codegen pipeline (no HIR/MIR). The identifier lifecycle for `set x: int = 1;`:

### Token stage (lexer.ajb)
Tokens produced: `SET(4)`, `ID:'x'(1)`, `COLON(31)`, `INT(13)`, `ASSIGN(41)`, `NUM:1(2)`, `SEMI(30)`

- `tokStrOff(buf)` for ID token points to "x" in source — **CORRECT**
- `tokStrLen(buf)` for ID token is 1 — **CORRECT**

### Parser stage (stmt.ajb `parseStmt`, type==4 handler)
1. `nextTok` → consumes SET, now at ID:'x'
2. `vname = identStr(src, tokStrOff, tokStrLen)` → **"x"** — **CORRECT**
3. `nextTok` → consumes ID:'x', now at COLON
4. `tokType == 31` → enters type annotation block
5. `nextTok` → consumes COLON, now at INT(13)
6. **BUG**: `tokType == 1` → **FALSE** (it's 13, not identifier)
7. Falls through — INT(13) token **NEVER CONSUMED**
8. Emits `intptr_t x` — **CORRECT** (vname is still "x")
9. Checks `tokType == 41` → **FALSE** (it's 13, not '=')
10. Emits `;` — returns

### Codegen stage
- Emits `intptr_t x;` — declaration correct
- Next `parseStmt` sees INT(13), falls to else-branch (expression statement)
- Expression parser sees `INT ASSIGN NUM` → emits `0 = 1`
- **NAME BECOMES 0 AT CODEGEN** — the `0` is the expression parser's default for an unrecognized token

### First stage where name becomes 0: **Parser** (stage 2)

The parser fails to consume the type keyword (INT/STRING/BOOL/VOID tokens 13-16) after COLON, leaving the token stream misaligned. The `=` check fails, so the variable name is never joined with the initializer. The leaked type keyword then gets parsed as an expression, producing `0`.

## Root Cause

In `compiler/stmt.ajb`, the `set` handler's type-annotation branch (lines 37-51) only handled `tokType == 1` (user-defined identifier types). Built-in type keywords (int=13, string=14, bool=15, void=16) were **not consumed**, leaving the token stream misaligned.

```ajb
// BROKEN: only tokType==1 consumed the type token
if (tokType(buf) == 31) {
    nextTok(src, buf);           // consume ':'
    if (tokType(buf) == 1) {     // only handles identifier types!
        // ... handles custom type, consumes it, returns
    }
    // FALLTHROUGH: int/string/bool/void tokens NOT consumed
}
emitStr(out, "intptr_t "); emitStr(out, vname);
if (tokType(buf) == 41) { ... }  // fails — tokType is still 13
```

## Fix Applied

Two changes in `compiler/stmt.ajb` (shared inode with `ajeebc/compiler/stmt.ajb`):

### Fix 1: Consume type keyword for built-in types
Added `nextTok(src, buf)` after the `if (tokType(buf) == 1)` block to consume the type token regardless of whether it's an identifier or keyword:

```ajb
if (tokType(buf) == 31) {
    nextTok(src, buf);
    if (tokType(buf) == 1) {
        // ... custom type handling ...
        return;
    }
    nextTok(src, buf);    // ← ADDED: consume built-in type keyword
}
```

### Fix 2: Consume trailing semicolon
The set handler emitted its own `;` but didn't consume the `;` token, causing extra `0;` statements. Added:

```ajb
if (tokType(buf) == 30) { nextTok(src, buf); }  // ← ADDED: consume ';'
emitStr(out, ";\n");
```

## Verification

```
set x: int = 1;
set name: string = "hello";
```

### AFTER (fixed)
```c
intptr_t x = 1;
intptr_t name = (intptr_t)"hello";
```

### Regression tests: 6/6 pass
- test_simple ✓
- test_math ✓
- test_if ✓
- test_while ✓
- test_for ✓
- test_strings ✓

### Self-hosted compilation of compiler.ajb: 363 lines output, no hangs
All `set` declarations in lexer functions (skipWS, matchKwd, readIdent, readNumber, readString, nextTok, identStr) now emit valid C identifiers.
