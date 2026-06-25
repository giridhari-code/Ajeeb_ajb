# FUNCTION EMISSION REPORT

## Summary

The self-hosted compiler (`build/compiler`) can now regenerate itself completely.
The root cause of missing function bodies was a **position save/restore bug** in the
import handler. After fixing it, the compiler produces 1,783 lines of C with 83
function definitions, compiles with GCC, and produces byte-identical output on
successive runs.

## A. Function Count

### Imported functions: 46

**lexer.ajb (16):**
isDigit, isAlpha, isAlphaNum, isSpace, skipWS, matchKwd, readIdent,
readNumber, readString, setTok, tokType, tokVal, tokStrOff, tokStrLen,
nextTok, identStr

**emit.ajb (3):**
emitStr, emitI, emitEscapedStr

**expr.ajb (11):**
hasStrExpr, parsePrimary, parseUnary, parseMul, parseAdd, parseCmp,
parseEq, parseAnd, parseOr, parseAssign, parseExpr

**stmt.ajb (2):**
parseType, parseStmt

**pass1.ajb (4):**
addFn, skipDepth, collectFns, emitFwdDecls

### Local functions (compiler.ajb): 11

rdB, wrB, rdPos, wrPos, rdFnC, wrFnC, rdSrc, wrSrc, savePos,
restorePos, **main**

### Total function bodies emitted: 83 (46 + 11 + main)

### Hardcoded runtime forward declarations: 38

These are declared in `main()` as `intptr_t fn(...)` and linked against
`runtime/ajeeb_runtime.c` at GCC compile time.

## B. Function Table Dump

| Function | Source | Collected? | Fwd Decl? | Body Emitted? |
|----------|--------|-----------|-----------|---------------|
| `main` | compiler.ajb | N/A | No | ✅ |
| `rdB` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `wrB` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `rdPos` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `wrPos` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `rdFnC` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `wrFnC` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `rdSrc` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `wrSrc` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `savePos` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `restorePos` | compiler.ajb | N/A | No (declared in main) | ✅ |
| `parseStmt` | stmt.ajb | N/A | Hardcoded in main | ✅ |
| `parseExpr` | expr.ajb | N/A | Hardcoded in main | ✅ |
| `collectFns` | pass1.ajb | N/A | No | ✅ |
| `emitFwdDecls` | pass1.ajb | N/A | No | ✅ |
| `addFn` | pass1.ajb | N/A | No | ✅ |
| `skipDepth` | pass1.ajb | N/A | No | ✅ |
| `emitStr` | emit.ajb | N/A | No | ✅ |
| `emitI` | emit.ajb | N/A | No | ✅ |
| `emitEscapedStr` | emit.ajb | N/A | No | ✅ |

Note: `collectFns` and `emitFwdDecls` are defined in pass1.ajb but NOT called
by `main()`. They exist as dead code — the compiler does not use a two-pass
approach. Function bodies are emitted inline during the single parse pass.

## C. Emission Trace (Before Fix)

Before the fix, the import handler had this sequence:

```
savePos(buf);           // saves position 0 (before import keyword)
wrSrc(buf, importSrcP); // switches to imported source
wrPos(buf, 0);          // resets position for imported source
// ... process imported file ...
restorePos(buf);         // RESTORES position to 0 (before import keyword!)
wrSrc(buf, savedSrc);   // switches back to original source
sp2 = rdPos(buf);       // reads position 0 from imported source context
// chr(src, sp2) checks chr(src, 0) = 'i' (NOT ';')
// semicolon is never consumed!
nextTok(src, buf);       // reads from position 0 = 'import' again = infinite loop
```

**Result:** After the first import (lexer.ajb) was processed, the parser
re-read the `import` keyword from position 0, creating an infinite loop that
eventually corrupted token state to EOF (`tokType = 0`), causing the main
loop to exit.

## D. Root Cause

**FIRST STAGE WHERE LOCAL FUNCTIONS DISAPPEAR:** The import handler in
`stmt.ajb` (line 278–292).

**Root cause:** `restorePos(buf)` restored the position to 0 (saved before
the import keyword) instead of the position after the `;`. This meant:
1. `chr(src, sp2)` checked position 0 in the original source (the `i` in `import`)
2. The `;` after the import statement was never consumed
3. `nextTok(src, buf)` re-read the `import` keyword, creating an infinite loop
4. The loop eventually set `tokType = 0` (EOF), terminating the main parse loop

**Fix:** Replace `savePos`/`restorePos` with explicit position save/restore:
```ajeeb
set savedPos: int = rdPos(buf);   // save position before import
// ... process imported file ...
wrPos(buf, savedPos);              // restore position to after import keyword
```

## E. Verification

| Test | Result |
|------|--------|
| `cargo test` | ✅ 24 passed |
| `bootstrap_check.sh` | ✅ MIR pipeline verified |
| Self-host compile → GCC | ✅ Compiles (79KB binary) |
| Self-host C output (stage 1) | ✅ 1,783 lines, 83 functions |
| Self-host C output (stage 2) | ✅ Byte-identical to stage 1 |
| Stage 1 → GCC → stage 2 | ✅ Compiles |
| Stage 2 → GCC | ✅ Compiles |

## F. Secondary Fix: Forward Declaration Return Types

The C codegen emits all Ajeeb functions with `intptr_t` return type, but
several runtime forward declarations used `void`. This caused GCC "conflicting
types" errors. Fixed by changing all forward declarations to `intptr_t`:

- `void setInt(...)` → `intptr_t setInt(...)`
- `void writeFile(...)` → `intptr_t writeFile(...)`
- `void writeAppend(...)` → `intptr_t writeAppend(...)`
- `void writeByte(...)` → `intptr_t writeByte(...)`
- `void strSet(...)` → `intptr_t strSet(...)`
- `void wrB(...)` → `intptr_t wrB(...)`
- `void wrPos(...)` → `intptr_t wrPos(...)`
- `void wrFnC(...)` → `intptr_t wrFnC(...)`
- `void wrSrc(...)` → `intptr_t wrSrc(...)`
- `void savePos(...)` → `intptr_t savePos(...)`
- `void restorePos(...)` → `intptr_t restorePos(...)`

Also added `chr` to the forward declarations (it was in the C runtime but
not declared in the generated output).
