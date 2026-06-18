# Ajeeb Language — Syntax Reference

## 1. Comments

```
// Single-line comment (C++ style)

/* Block comments are NOT supported yet */
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

### Arithmetic

| Operator | Meaning |
|----------|---------|
| `+`      | Add / concat |
| `-`      | Subtract |
| `*`      | Multiply |
| `/`      | Divide (integer) |

### Comparison

| Operator | Meaning |
|----------|---------|
| `==`     | Equal |
| `!=`     | Not equal |
| `<`      | Less than |
| `>`      | Greater than |
| `<=`     | Less or equal |
| `>=`     | Greater or equal |

### Logical

| Operator | Meaning |
|----------|---------|
| `&&`     | AND |
| `\|\|`   | OR |
| `!`      | NOT |

### Assignment

```
x = 5;           // variable assignment
arr[0] = 10;     // array index assignment
obj.field = v;   // field assignment
```

## 6. Control Flow

### If / Else

```
if (condition) {
    // then block
} else if (other) {
    // else-if block
} else {
    // else block
}
```

### While

```
while (x < 10) {
    println(x);
    x = x + 1;
}
```

### For

```
for (let i = 0; i < 10; i = i + 1) {
    println(i);
}
```

Init can be `let` declaration, expression, or empty. Condition and update are expressions.

### Break / Continue

```
for (let i = 0; i < 100; i = i + 1) {
    if (i == 5) { break; }     // exit loop
    if (i == 3) { continue; }  // next iteration
    println(i);
}
```

## 7. Functions

```
function add(a: int, b: int): int {
    return a + b;
}

function greet(name: string): void {
    println("Hello " + name);
}

function main(): int {
    return 0;
}
```

`main()` is the program entry point. Return type defaults to `void` if omitted.

## 8. Classes

```
class Counter {
    count: int;
    label: string;

    function increment(step: int): void {
        self.count = self.count + step;
    }

    function getValue(): int {
        return self.count;
    }
}

let c = new Counter;
c.count = 0;
c.increment(5);
println(c.getValue());
```

- Fields must have type annotations
- `self` refers to the current instance
- Methods are called with dot syntax: `obj.method(args)`

## 9. Arrays

```
let arr: int[] = [1, 2, 3];
let x = arr[0];        // read index → 1
arr[1] = 42;           // write index
println(len(arr));     // array length → 3
```

Arrays are 0-indexed. Accessing out of bounds returns 0.

## 10. Built-in Functions

### I/O

| Function | Description |
|----------|-------------|
| `print(val, ...)` | Print values without newline |
| `println(val, ...)` | Print values with newline |
| `readFile(path)` | Read file → string |
| `writeFile(path, content)` | Write string to file |
| `writeAppend(path, content)` | Append string to file |
| `writeByte(path, byte)` | Write single byte |

### String Operations

| Function | Description |
|----------|-------------|
| `len(s)` | String length |
| `charCode(s, i)` | Get byte at index |
| `strSet(s, i, c)` | Set byte at index |
| `strcpy(dst, src)` | Copy string |
| `strcmp(a, b)` | Compare strings (-1, 0, 1) |
| `strcmp_ajeeb(a, b)` | Nul-terminated compare |
| `itoa(n)` | Integer to string |
| `chr(s, i)` | Alias for charCode |
| `str_concat(a, b)` | Concatenate strings |
| `substring(s, start, end)` | Extract substring |
| `indexOf(s, search)` | Find substring position (-1 if not found) |
| `contains(s, search)` | Check if substring exists (0/1) |
| `toUpperCase(s)` | Convert to uppercase |
| `toLowerCase(s)` | Convert to lowercase |
| `trim(s)` | Remove leading/trailing whitespace |
| `split(s, delim)` | Split string → string[] |
| `replace(s, from, to)` | Replace all occurrences |
| `startsWith(s, prefix)` | Check prefix (0/1) |
| `endsWith(s, suffix)` | Check suffix (0/1) |

### CLI & Buffer

| Function | Description |
|----------|-------------|
| `readArg(n)` | Read n-th CLI argument |
| `getStateBuf()` | Get state buffer pointer |
| `getOutbuf()` | Get output buffer (used internally by compiler) |
| `rdB(buf, off)` / `getInt(buf, off)` | Read i64 from buffer |
| `wrB(buf, off, v)` / `setInt(buf, off, v)` | Write i64 to buffer |
| `rdPos(buf)` | Read position from buffer |
| `wrPos(buf, v)` | Write position to buffer |

### Character Classification

| Function | Description |
|----------|-------------|
| `isDigit(c)` | Is '0'-'9' (0/1) |
| `isAlpha(c)` | Is a-z, A-Z, or _ (0/1) |
| `isAlphaNum(c)` | Is alphanumeric or _ (0/1) |
| `isSpace(c)` | Is space/tab/newline/cr (0/1) |

## 11. Complete Program Example

```
function factorial(n: int): int {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

function main(): int {
    let result = factorial(5);
    println("factorial(5) = " + itoa(result));

    for (let i = 0; i < 5; i = i + 1) {
        println(i);
    }

    let arr: int[] = [10, 20, 30];
    arr[1] = 99;
    println(arr[1]);  // 99

    return 0;
}
```

## 12. Type Annotations

```
let x: int = 5;
let s: string = "hi";
let b: bool = true;
let arr: int[] = [1, 2, 3];
function f(): void { }
function g(): int { return 0; }
```

Array type uses `[]` suffix: `int[]`, `string[]`, etc.

## 13. Config (.das files)

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
