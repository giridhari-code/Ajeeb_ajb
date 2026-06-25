# P0-3: Method Dispatch — Implementation Report

## Status: COMPLETE

## Summary
Method calls on class instances now emit correctly prefixed C function calls.

**Before:** `c.inc()` → `inc(&c)` (undefined function)
**After:** `c.inc()` → `Counter_inc(&c)` (correctly resolved)

## Verification Case
```ajeeb
class Counter {
    set value: int;
    function inc(self): int { return self.value + 1; }
}

function main(): int {
    set c: Counter = Counter();
    c.value = 10;
    println(itoa(c.inc()));  // Output: 11
    return 0;
}
```

Generated C:
```c
typedef struct { intptr_t value; } Counter;
intptr_t Counter_inc(Counter* self) { return self->value + 1; }
int main(int argc, char** argv) {
    Counter c = (Counter){0};
    c.value = 10;
    println(itoa(Counter_inc(&c)));
    return 0;
}
```

## Implementation Details

### 1. Method declaration prefixing (stmt.ajb)
Method declarations use `ClassName_methodName` format:
```
Counter_inc(Counter* self)
```
- Class name extracted from `class Name { ... }` block
- Method identifier extracted from `function methodName(...)` inside class
- Output format: `ClassName_methodName(ClassName* self)`

### 2. Method call resolution (expr.ajb:188-209)
When `c.inc()` is parsed (dot/arrow token followed by `(`):
1. Look up receiver variable name via `getVarType(id, buf)`
2. If type found (e.g. "Counter"), emit `Counter_inc(&c)`
3. If type not found, fallback to `inc(&c)` (backward compatible)

### 3. Variable type tracking (stmt.ajb:12-30)
**Buffer slot layout:**
- Slot 1000 (offset 1000): count of registered variable types
- Slots 1008+i*16: name pointer (string)
- Slots 1008+i*16+8: class name pointer (string)

**Registration points:**
- `set c: Counter = ...` → explicit type annotation
- `set c = Counter()` → constructor call detection
- `set p = Point { ... }` → struct literal detection

**Functions:**
- `registerVarType(name, className, buf)` — stores name→className mapping
- `getVarType(name, buf)` — retrieves className for a variable name

### 4. Forward declaration (pass1.ajb)
Class methods registered via `addMethodFn()` instead of `addFn()`:
- Writes `ClassName_methodName` to `_fnnames.txt`
- Class name tracked during `class { ... }` block scanning

## Bug Fixed: Buffer Offset Collision

**Root cause:** Variable type tracking used buffer offsets 199-399, which overlapped with the function table at offset 96+ (24 bytes/entry). Function #4 (offset 192-215) corrupted variable type data at offset 199.

**Symptom:** Segfault when calling any method on a class instance.

**Fix:** Moved variable type storage to offsets 1000+ (count) and 1008+ (pairs), well beyond the function table range.

## Regression Tests
| Test | Result |
|------|--------|
| test_simple | ✓ Hello World |
| test_for | ✓ 0,1,2,4,5 |
| test_if | ✓ bada hai |
| test_while | ✓ 0,1,2 |
| cross_simple | ✓ sum, factorial, Hello World |
| struct_basic | ✓ Ajeeb, 1 |
| struct_literal | ✓ 10, 20 |

## Self-Hosting
- Rust interpreter rebuilds compiler: ✓
- Self-hosted binary compiles test files: ✓
- Method dispatch compiles correctly: ✓

## Files Modified
- `compiler/stmt.ajb` — `getVarType`/`registerVarType` offsets changed from 199-399 to 1000-1168
