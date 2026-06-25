# BLOCK EMISSION AUDIT

## Issues Found

### 1. Missing braces on if/while/else/for bodies

**Before:**
```c
if ((1 == 1))     println("ok");
while ((0 == 1))     println("loop");
if ((1 == 1))     intptr_t x = 1;
println(itoa(x));  // ← leaked outside if block
else     println("b");
```

**After:**
```c
if ((1 == 1)) {
    println("ok");
}
while ((0 == 1)) {
    println("loop");
}
if ((1 == 1)) {
    intptr_t x = 1;
    println(itoa(x));
}
if ((1 == 1)) {
    println("a");
} else {
    println("b");
}
```

### Root Cause

In `compiler/stmt.ajb`, the `if`/`while`/`else`/`for` handlers emitted `"    if (...) "` then processed body statements without wrapping in `{ }`.

## Fix Applied

Three changes in `compiler/stmt.ajb`:

1. **if handler** (lines 71-96): Changed `emitStr(out, ") ")` → `emitStr(out, ") {\n")`, added `emitStr(out, "    }\n")` after body. Changed `emitStr(out, " else ")` → `emitStr(out, "    } else {\n")`, added closing `}` for else block.

2. **while handler** (lines 97-110): Changed `emitStr(out, ") ")` → `emitStr(out, ") {\n")`, added `emitStr(out, "    }\n")` after body.

3. **for handler** (lines 136-144): Changed `emitStr(out, "++) ")` → `emitStr(out, "++) {\n")`, added `emitStr(out, "    }\n")` after body.

## Verification

- if/while/else blocks compile and run correctly
- set declarations inside if blocks work
- 6/6 regression tests pass
