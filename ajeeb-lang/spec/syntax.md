# Ajeeb Language — Syntax Reference

## 1. Comments

```
// Single-line comment (C++ style)
```

## 2. Literals

```
42              // int
-10             // negative int
"hello world"   // string (double quotes only)
true            // bool
false           // bool
```

### String Escape Sequences

| Escape | Meaning       |
|--------|---------------|
| `\n`   | Newline       |
| `\t`   | Tab           |
| `\"`   | Double quote  |
| `\\`   | Backslash     |
| `\0`   | Null byte     |

## 3. Variables

```
let x: int = 10;           // typed
let y = 20;                // type-inferred
const MAX: int = 100;      // immutable
let name: string = "foo";  // string type
```

## 4. Types

```
int          // integer (64-bit)
string       // mutable string
bool         // boolean (true/false)
void         // no return value
int[]        // array of int
string[]     // array of strings
ClassName    // class instance
```

## 5. Operators

### Arithmetic: `+` `-` `*` `/`
### Comparison: `==` `!=` `<` `>` `<=` `>=`
### Logical: `&&` `||` `!`
### Assignment: `=` `arr[0] = v` `obj.field = v`

## 6. Control Flow

### If / Else
```
if (condition) { ... } else if (other) { ... } else { ... }
```

### While
```
while (x < 10) { println(x); x = x + 1; }
```

### For
```
for (let i = 0; i < 10; i = i + 1) { println(i); }
```

### Break / Continue
```
for (let i = 0; i < 100; i = i + 1) {
    if (i == 5) { break; }
    if (i == 3) { continue; }
    println(i);
}
```

## 7. Functions

```
function add(a: int, b: int): int { return a + b; }
function greet(name: string): void { println("Hello " + name); }
function main(): int { return 0; }
```

## 8. Classes

```
class Counter {
    count: int;
    label: string;
    function increment(step: int): void { self.count = self.count + step; }
    function getValue(): int { return self.count; }
}
let c = new Counter;
c.count = 0;
c.increment(5);
println(c.getValue());
```

## 9. Arrays

```
let arr: int[] = [1, 2, 3];
let x = arr[0];
arr[1] = 42;
println(len(arr));
```

## 10. Config (.das files)

```
[package]
name = "my-project"
version = "0.1.0"

[dependencies]

[runtime]
max_threads = "8"
log_level = "info"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
```
