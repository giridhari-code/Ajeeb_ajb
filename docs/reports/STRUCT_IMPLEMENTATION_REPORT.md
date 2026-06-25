# Struct Implementation Report (P0-2)

## Summary
Struct declarations and struct literal construction are now fully working in the self-hosted compiler.

## What Works

### 1. Struct Declaration
```ajb
struct Point {
    x: int,
    y: int
}
```
Emits C typedef: `typedef struct { intptr_t x; intptr_t y; } Point;`

### 2. Struct Literal Construction (with type inference)
```ajb
set p = Point { x: 10, y: 20 };
```
Emits C: `Point p = (Point){.x = 10, .y = 20};`

Type inference detects struct literals via lookahead:
- After `=`, peek at next token
- If it's an identifier followed by `{` and is a known struct name → use struct type
- Otherwise default to `intptr_t`

### 3. Struct Field Access
```ajb
println(itoa(p.x));
println(itoa(p.y));
```
Emits C: `p.x`, `p.y` — standard C struct member access.

### 4. Struct Name Registration
Struct names are registered in buffer slot 400 (same as class names), using comma-delimited string with `isClassName()` lookup.

## Test Results
| Test | Status | Output |
|------|--------|--------|
| struct_basic.ajb | PASS | Ajeeb\n1 |
| struct_literal.ajb | PASS | 10\n20 |
| test_simple.ajb | PASS | Hello World |
| test_math.ajb | PASS | 42 |
| test_if.ajb | PASS | bada hai |
| test_while.ajb | PASS | 0\n1\n2 |
| test_for.ajb | PASS | 0\n1\n2\n4\n5 |
| test_strings.ajb | PASS | Hello World\nHELLO\najeeb\n-1\n1\nHello |
| Self-hosting | PASS | compiler recompiles itself |

## Key Implementation Details

### Files Modified
- `compiler/lexer.ajb`: Added `struct` keyword as token type 49
- `compiler/stmt.ajb`: Added struct declaration handler (t==49), `isClassName()`, struct name registration in slot 400
- `compiler/expr.ajb`: Added struct literal handler (t==37 + isClassName check)

### The Type Inference Problem
The `set` handler needed lookahead to detect `set v = StructName { ... }`. The solution:
1. After `=`, check if next token is identifier
2. Save token position (`tokStrOff`, `tokStrLen`, `rdPos`)
3. Advance to peek at next token
4. If `{` and `isClassName` → set vtype to struct name
5. Restore token state via `setTok` + `wrPos`
6. Let `parseExpr` re-parse the identifier and struct literal

### Known Limitation
- `print` function (not `println`) has a broken temp-file mechanism that corrupts output
- Struct field types all emit as `intptr_t` (no type info at codegen level)
- No explicit struct type annotation (`set p: Point = ...`) — only implicit via `set p = Point { ... }`
- Fields are limited to int/string/bool/void/identifier types in declaration
