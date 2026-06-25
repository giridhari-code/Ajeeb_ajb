# ROOT_BLOCKER.md — Self-Hosted Build Failure

## Summary

The self-hosted build (`build/compiler compiler/compiler.ajb`) hangs with an infinite loop.
The root cause is in `parseStmt`'s `if`/`while`/`for` handlers: they call `parseStmt(src, buf, out)`
for the body **without first consuming the opening `{` token**.

---

## Exact Stage

**Code generation / statement parsing** — specifically, the `import` handler's recursive parse
of imported files. The parser enters an infinite loop processing a stray `}` token.

## Exact Function

`parseStmt` in `compiler/stmt.ajb`, the `if` handler branch (token type 6).

## Exact Source Line

`compiler/stmt.ajb:71`:
```
        if (tokType(buf) == 37) {
            parseStmt(src, buf, out);
        }
```

The same bug exists at:
- `compiler/stmt.ajb:89` — `while` handler
- `compiler/stmt.ajb:118` — `for` handler

## Root Cause Analysis

The `if` handler (type 6) parses the condition, then checks for `{` (type 37). When found,
it calls `parseStmt(src, buf, out)` **without consuming the `{`**. This causes:

1. `parseStmt` is called with `{` as the current token
2. `{` (type 37) is not a recognized statement type — falls to the `else` clause (line 261)
3. The `else` clause calls `parseExpr`, which calls `parsePrimary`
4. `parsePrimary` doesn't recognize `{` — falls to catch-all at `expr.ajb:195`: emits `"0"`, consumes `{`
5. The if body's `{` is consumed as expression `"0"`
6. The if handler returns without processing the actual body statements
7. The calling function body loop processes the body statements as top-level
8. The if body's closing `}` causes the **function body loop to exit prematurely**
9. The function's actual `}` leaks to the import handler's loop
10. `parseStmt` for `}` (type 38, `stmt.ajb:259`) does nothing — **infinite loop**

## Reproduction Steps

```bash
# 1. Build Rust compiler (works)
make rust  # or: cargo build -p ajeeb-compiler

# 2. Rust compiler compiles compiler.ajb → native binary (works)
./build/ajeeb_compiler compiler/compiler.ajb --skip-run
# Output: build/compiler created successfully

# 3. Native binary tries to re-compile compiler.ajb (HANGS)
timeout 30 ./build/compiler compiler/compiler.ajb
# Hangs forever, exit code 124 (timeout)
```

## Proof Log

### Step 1: Instrumented compiler.ajb with file-based progress markers

Added `writeAppend("build/_progress.txt", ...)` markers at each pipeline stage:

```
MARK 1: setup done
MARK 2: collectFns done
MARK 3: nextTok done
MARK 4: headers done
MARK 5: fwdDecls done
MARK 5a: entering parseStmt    ← main parse loop
```

### Step 2: Rebuilt and ran instrumented native binary

```
$ rm -f build/_progress.txt && timeout 60 build/compiler compiler/compiler.ajb
EXIT: 124

$ cat build/_progress.txt
MARK 1: setup done
MARK 2: collectFns done
MARK 3: nextTok done
MARK 4: headers done
MARK 5: fwdDecls done
MARK 5a: entering parseStmt
STMT type=48    ← import lexer
STMT type=9     ← function (from lexer.ajb, e.g., isDigit)
STMT type=10    ← return (inside function body)
STMT type=10    ← return (LEAKED to import handler level)
STMT type=38    ← } → INFINITE LOOP (123,975 iterations)
```

### Step 3: Token type analysis

| Type | Token | Count | Significance |
|------|-------|-------|-------------|
| 48   | import | 1    | First statement, processed correctly |
| 9    | function | 1  | From lexer.ajb, function isDigit |
| 10   | return | 2   | Two returns: one from if body, one leaked |
| 38   | }     | 123,975 | Infinite loop — `parseStmt` doesn't consume `}` |

### Step 4: Call trace for isDigit

The function `isDigit` in `lexer.ajb`:
```
function isDigit(c: int): int {
    if (c >= 48 && c <= 57) { return 1; } return 0;
}
```

Execution trace:
1. Function handler enters, consumes `function isDigit(...)`, consumes `{`
2. Function body loop: calls parseStmt for `if (c >= 48 && c <= 57) { return 1; } return 0;`
3. `parseStmt` type=6 (if): consumes `if`, parses condition `c >= 48 && c <= 57`
4. Sees `{` (type 37), calls `parseStmt` **without consuming `{`**
5. Inner `parseStmt`: `{` falls to else clause, `parseExpr` consumes `{` as `"0"`
6. If handler returns. Function body loop continues with `return 1;`
7. `return 1;` is processed, then `}` (if body's closing brace) is seen
8. Function body loop exits (thinks it found function's `}`)
9. Function handler consumes `}` (actually the if body's `}`)
10. Function handler returns to import handler loop
11. Import handler processes `return 0;` (leaked from function body)
12. Import handler sees `}` (function body's closing brace)
13. `parseStmt` type=38: empty handler, no `nextTok` → **infinite loop**

## Affected Code Locations

| File | Line | Handler | Issue |
|------|------|---------|-------|
| `compiler/stmt.ajb` | 71 | `if` (type 6) | Calls `parseStmt` without consuming `{` |
| `compiler/stmt.ajb` | 89 | `while` (type 8) | Same pattern |
| `compiler/stmt.ajb` | 118 | `for` (type 47) | Same pattern |
| `compiler/stmt.ajb` | 259 | `}` (type 38) | Empty handler, no `nextTok` — causes infinite loop |

## Why the Rust Interpreter Works

The Rust interpreter handles `import` statements at the Rust level (ModuleLoader), not through
the Ajeeb `parseStmt` import handler. So the Ajeeb-level import bug is never exercised during
the first compilation. Only the self-hosted build (native binary compiling compiler.ajb) triggers
this code path.

## Status

**BLOCKER IDENTIFIED. NO FIX APPLIED.**
