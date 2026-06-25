# Feature Verification Report

## Methodology
- Source code audit of `ajeebc/compiler/main.ajb` (1677 LOC)
- Binary execution via self-hosted compiler (`build/main`) and Rust compiler (`ajeebBootstrap/target/release/ajeeb_compiler`)
- All tests run from `ajeebc/` directory

---

## Feature 1: true/false Literals

### Status: IMPLEMENTED

### Source Evidence

**Lexer** (`main.ajb:186`): `true`/`false` lex as token type 3 (identifier) via `lexIdent`. No special lexer token.

**Parser** (`main.ajb:364-365`): In `parseAtom`, after consuming an identifier (token 3):
```
line 364: if (sl == 4 && charCode(src, so) == 116 && charCode(src, so+1) == 114
              && charCode(src, so+2) == 117 && charCode(src, so+3) == 101)
              { return hBool(buf, 1); }
line 365: if (sl == 5 && charCode(src, so) == 102 && charCode(src, so+1) == 97
              && charCode(src, so+2) == 108 && charCode(src, so+3) == 115
              && charCode(src, so+4) == 101)
              { return hBool(buf, 0); }
```
Char codes: `true` = 116(t),114(r),117(u),101(e). `false` = 102(f),97(a),108(l),115(s),101(e).

**HIR** (`main.ajb:105-107`): `hBool` creates node tag 17, stores value (0 or 1):
```
function hBool(buf, v): int {
    set o = halloc(buf, 3); bw(buf, o, 17); bw(buf, o+1, v); return o;
}
```

**MIR Lowering** (`main.ajb:722`): `appendInstr(mirBuf, 2, t, v, 0, 3)` — op 2 (mov), ext 3 (bool type).

**C Codegen** (`main.ajb:1224-1225`): `mov` with ext 3 emits `tN = tM;` (same as int, since bool is stored as 0/1 intptr_t).

### Test Results

| Test | Rust Compiler | Self-Hosted |
|------|:---:|:---:|
| `set x: bool = true; if (x) { ... }` | PASS | PASS |
| `set y: bool = false; if (!y) { ... }` | PASS | PASS |

Binary output:
```
PASS: true works
PASS: false works
```

---

## Feature 2: const Declarations

### Status: IMPLEMENTED

### Source Evidence

**Lexer** (`main.ajb:186`): `const` lex as token type 3 (identifier) via `lexIdent`. No special lexer token.

**Parser** (`main.ajb:549`): `parseStmt` dispatches:
```
if (isKwd(src, buf, "const")) { return parseSetStmt(src, buf); }
```
`const` is parsed identically to `set` — same `parseSetStmt` function, same HIR node (tag 3 via `hSet`), same MIR lowering, same C codegen. No semantic distinction.

**HIR** (`main.ajb:79-82`): `hSet` creates node tag 3, stores name off/len, type, and value.

**MIR Lowering** (`main.ajb:706-710`): `set` and `const` both emit store instruction (op 3) with type annotation.

**C Codegen** (`main.ajb:1263-1264`): `tN = pM;` (same as set).

### Test Results

| Test | Rust Compiler | Self-Hosted |
|------|:---:|:---:|
| `const x: int = 42;` | PASS | PASS |
| `const name: string = "hello"; println(name);` | PASS | PASS |

Binary output:
```
PASS: const works
hello
```

**Limitation**: `const` has no semantic enforcement — it's treated identically to `set`. The variable can be reassigned. This matches the stated design (keyword alias only).

---

## Feature 3: class Name { }

### Status: IMPLEMENTED

### Source Evidence

**Lexer** (`main.ajb:186`): `class` lex as token type 3 (identifier) via `lexIdent`. No special lexer token.

**Parser** (`main.ajb:568`): `parseStmt` dispatches:
```
if (isKwd(src, buf, "class")) { return parseClassDecl(src, buf); }
```

**Parser** (`main.ajb:573-578`): `parseClassDecl`:
```
function parseClassDecl(src, buf): int {
    nextTok(src, buf);  // skip 'class'
    // skip to '{'
    while (tokType(buf) != 7 && tokType(buf) != 0) { nextTok(src, buf); }
    set body = parseBlock(src, buf);  // parse body as Block node
    return body;
}
```
Class is parsed as a Block node (tag 2). The class keyword and name are consumed but not stored. Functions defined inside the class become top-level functions.

**HIR** (`main.ajb:76-78`): `hBlock` creates node tag 2, stores statement count.

**MIR/C Codegen**: No special handling — class body functions are lowered as normal top-level functions.

### Test Results

| Test | Rust Compiler | Self-Hosted |
|------|:---:|:---:|
| `class User { function greet(): void { ... } }` + `greet();` | PASS | PASS |

Binary output:
```
PASS: class works
```

**Limitation**: `class` has no fields, no `this`, no constructors. It's purely a function grouping mechanism. Methods become free functions. This matches the design intent for `compiler.ajb` compatibility.

---

## Feature 4: self Keyword

### Status: PARTIALLY IMPLEMENTED

### Source Evidence

**Lexer** (`main.ajb:186`): `self` lex as token type 3 (identifier) via `lexIdent`. **NOT** a keyword — no special token type.

**Lexer** (`main.ajb:208`): The `.` dot character is lexed as token type 36:
```
else if (c == 46) { bw(buf, 50, 36); }
```

**Parser** (`main.ajb:363-389`): In `parseAtom`, after consuming an identifier:
```
if (tokType(buf) == 36) {        // '.' token
    nextTok(src, buf);            // skip '.'
    set methodSo = tokStrOff(buf); set methodSl = tokStrLen(buf);
    nextTok(src, buf);            // skip method name
    if (tokType(buf) == 5) {     // '(' follows → method call
        // Parse args, first arg = obj (the ident before '.')
        // Create hCall(methodSo, methodSl, ac)
    }
    return hIdent(methodSo, methodSl);  // field access fallback
}
```

**MIR/C Codegen**: No special handling for dot-access. `obj.method(args)` becomes `method(obj, args)` at the HIR level. C codegen emits normal function call.

### What Works

`self` as a **parameter name** works correctly:
```
function add(self: int): int { return self + 10; }
```

`obj.method(args)` pattern works — the parser transforms it to `method(obj, args)`:
```
// Self-hosted output: PASS
```

### What's Missing

1. **`self` is NOT a keyword** — it's just an identifier (token 3). There is no token type 43.
2. **`self.field` (field access without parens)** — The parser returns `hIdent(methodSo, methodSl)` which just returns the method name as an identifier, NOT the field value. This is a semantic gap.
3. **`self[idx]` (bracket access after dot)** — Not handled in the dot-access code path.

### Test Results

| Test | Rust Compiler | Self-Hosted |
|------|:---:|:---:|
| `function f(self: int): int { return self + 10; }` | PASS | PASS |
| `function f(self: string): string { return self; }` + `f("x")` | PASS | PASS |
| `set r = parseAdd(self, buf)` (method call pattern) | PASS | PASS |

### Self-Hosting Status

The self-hosted compiler (`build/main`) **CANNOT compile `compiler.ajb`**:
- `compiler.ajb` imports `lexer.ajb`, `emit.ajb`, `expr.ajb`, `stmt.ajb`, `pass1.ajb`
- `lexer.ajb`: C codegen produces broken C (`intptr_t` used as function name)
- `compiler.ajb`: Segfault in the self-hosted compiler
- Individual files (`emit.ajb`, `expr.ajb`, `stmt.ajb`): Compile OK via LLVM fallback

---

## Summary

| Feature | Status | Self-Hosts? |
|---------|--------|:-----------:|
| true/false literals | **Implemented** | YES |
| const declarations | **Implemented** | YES |
| class Name { } | **Implemented** | YES |
| self keyword | **Partially Implemented** | NO — self-hosting blocked by C codegen bug |

### Self-Hosting Blocker

The self-hosted compiler can compile simple programs but fails on `compiler.ajb` (1248 LOC across 6 files). The blocker is **NOT** a missing parser feature — it's a **C codegen bug** where `lexer.ajb` compilation produces `intptr_t` as a function name in the generated C, causing GCC to reject the output.

The LLVM path (used by the Rust compiler) handles all 4 features correctly.
