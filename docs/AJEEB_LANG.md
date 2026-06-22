# Ajeeb Language Reference

Ajeeb is a statically-typed, imperative programming language with C-like syntax. It supports structs, enums, classes, generics, traits, and pattern matching.

## 1. Data Types

### Primitive Types

| Type     | Description                          | Example              |
|----------|--------------------------------------|----------------------|
| `int`    | 64-bit signed integer                | `42`, `-7`, `0`      |
| `float`  | 64-bit floating-point number         | `3.14`, `-0.5`       |
| `string` | Immutable string of characters       | `"hello"`, `""`      |
| `bool`   | Boolean value                        | `true`, `false`      |
| `void`   | No return value (used for functions) | (used as return type)|

### Array Type

Arrays hold elements of a single type. Use `Type[]` syntax.

```
set arr: int[] = [1, 2, 3];
set names: string[] = ["a", "b"];
```

- Access elements: `arr[0]`
- Assign elements: `arr[1] = 99`
- Get length: `len(arr)` or `arr_len(arr)`

### Struct Type

Structs are value-type composite data. Defined with `struct`.

```
struct User {
    name: string,
    age: int
}
```

- Create instances: `set u = User { name: "Ajeeb", age: 1 };`
- Access fields: `u.name`
- Assign fields: `u.age = 2`

### Class Type

Classes are reference-type composite data with methods. Defined with `class`.

```
class MyClass {
    function doStuff(x: int): int {
        return x + 1;
    }
}
```

- Create instances: `set obj = new MyClass;` (uses `new` keyword)
- Access fields/methods: `obj.doStuff(5)`

### Enum Type

Enums define a type with a fixed set of variants. Variants can carry data.

```
enum Color {
    Red,
    Green,
    Blue
}

enum Option {
    Some(int),
    None
}
```

- Reference variants: `Color::Red`, `Option::Some(42)`

---

## 2. Variables

### `set` — Mutable Variable Declaration

```
set x: int = 10;
set name: string = "Ajeeb";
set flag: bool = true;
```

- Type annotation after the name: `set name: Type = value;`
- Type can be inferred: `set x = 10;` (infers `int`)
- Must have an initializer: `set x: int;` is **invalid**

### `const` — Constant Declaration

```
const MAX: int = 100;
const PI: float = 3.14;
```

- Same syntax as `set` but cannot be reassigned.

### Variable Scope

- Variables declared inside a function are local to that function.
- Variables declared inside a block (`{ }`) are local to that block.
- No global variables (module-scope `set` is not accessible from functions).

### Duplicate Declarations

Multiple `set` with the same variable name in the same function is a **duplicate variable error**, even if in different `if` branches. Declare once at the function top, then reassign with plain assignment:

```
set x: int = 0;
if (condition) {
    x = 10;  // assignment, not declaration
} else {
    x = 20;  // assignment, not declaration
}
```

---

## 3. Functions

### Function Definition

```
function greet(name: string): void {
    println("Hello " + name);
}

function add(x: int, y: int): int {
    return x + y;
}
```

- Keyword: `function` (or `fn`)
- Parameters: `(name: Type, ...)`
- Return type after `:`
- Body is a block `{ ... }`
- Use `return value;` to return a value
- Functions without `return` return void

### Function Calls

```
greet("World");
set result = add(10, 32);
```

### Recursive Functions

```
function factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}
```

### Generic Functions

Type parameters are declared with `[T]`:

```
function identity[T](x: T): T {
    return x;
}

// Call with explicit type argument:
set a = identity[Int](42);
set b = identity[String]("hello");
```

---

## 4. Control Flow

### if/else

```
if (x > 5) {
    println("greater");
} else {
    println("less or equal");
}
```

- Condition must be a boolean expression
- `else` is optional
- Supports nested if/else

### while Loop

```
set i: int = 0;
while (i < 10) {
    println(itoa(i));
    i = i + 1;
}
```

### for Loop

C-style for loop with init, condition, and update:

```
for (set i: int = 0; i < 10; i = i + 1) {
    println(itoa(i));
}
```

### match Expression

Pattern matching on enums, integers, or strings:

```
set c = Color::Red;
set msg = match c {
    Color::Red => "Red",
    Color::Green => "Green",
    Color::Blue => "Blue",
};
```

- Wildcard pattern: `_ => default_value,`
- Bind values from enum variants: `Option::Some(v) => v,`
- Match expressions return a value (used as an expression, not a statement)

### break and continue

```
while (true) {
    if (condition) { break; }     // exit loop
    if (other) { continue; }      // skip to next iteration
}
```

- `break` exits the innermost loop
- `continue` skips to the next iteration of the innermost loop

---

## 5. Operators

### Arithmetic

| Operator | Description       |
|----------|-------------------|
| `+`      | Addition          |
| `-`      | Subtraction       |
| `*`      | Multiplication    |
| `/`      | Division (integer)|
| `%`      | Modulo            |
| `**`     | Exponentiation    |

### Comparison

| Operator | Description       |
|----------|-------------------|
| `==`     | Equal             |
| `!=`     | Not equal         |
| `<`      | Less than         |
| `>`      | Greater than      |
| `<=`     | Less or equal     |
| `>=`     | Greater or equal  |

### Logical

| Operator | Description       |
|----------|-------------------|
| `&&`     | Logical AND       |
| `||`     | Logical OR        |
| `!`      | Logical NOT       |

### Assignment

| Operator | Description            |
|----------|------------------------|
| `=`      | Simple assignment      |
| `+=`     | Add and assign         |
| `-=`     | Subtract and assign    |
| `*=`     | Multiply and assign    |
| `/=`     | Divide and assign      |

### Increment / Decrement

| Operator | Description                    |
|----------|--------------------------------|
| `++`     | Prefix/postfix increment       |
| `--`     | Prefix/postfix decrement       |

### String Concatenation

```
set full = "Hello " + "World";
```

### Unary Operators

| Operator | Description       |
|----------|-------------------|
| `-`      | Negation          |
| `!`      | Logical NOT       |

### Precedence (highest to lowest)

1. `!`, `-` (unary)
2. `*`, `/`, `%`, `**`
3. `+`, `-`
4. `<`, `>`, `<=`, `>=`
5. `==`, `!=`
6. `&&`
7. `||`
8. `=` (assignment)

---

## 6. Imports and Modules

### Import Syntax

```
import lexer;
import emit;
import math;
import string;
```

- Imports are resolved relative to `./packages/ajeeb-std/` for standard library modules.
- For local files, use a relative path: `import mymodule;`
- Import statements make all functions from the imported file available.

### C Shared Library Import

```
@import "lib.so";
```

- The `@import` directive loads a C shared library for FFI (Foreign Function Interface).

### Import Resolution

Imports are resolved at two levels:

1. **Rust ModuleLoader** (compile time) — flattens all function definitions from all files
2. **Ajeeb-level import handler** (codegen) — recursively processes imported files during code generation

---

## 7. Standard Library

Available via `import module;` (resolves to `packages/ajeeb-std/module.ajb`).

### `io` — Input/Output

| Function           | Signature                        | Description                    |
|--------------------|----------------------------------|--------------------------------|
| `print`            | `(s: string): void`              | Print without newline          |
| `printLine`        | `(s: string): void`              | Print with newline             |
| `printInt`         | `(n: int): void`                 | Print integer with newline     |
| `printBool`        | `(b: bool): void`                | Print boolean with newline     |
| `readLine`         | `(): string`                     | Read a line from input         |
| `readFileLines`    | `(path: string): string[]`       | Read file lines into array     |
| `writeFileLines`   | `(path: string, lines: string[]): void` | Write lines to file   |

### `math` — Math Functions

| Function    | Signature                      | Description                    |
|-------------|--------------------------------|--------------------------------|
| `abs`       | `(n: int): int`                | Absolute value                 |
| `max`       | `(a: int, b: int): int`        | Maximum of two values          |
| `min`       | `(a: int, b: int): int`        | Minimum of two values          |
| `pow`       | `(base: int, exp: int): int`   | Exponentiation                 |
| `factorial` | `(n: int): int`                | Factorial (recursive)          |
| `gcd`       | `(a: int, b: int): int`        | Greatest common divisor        |
| `lcm`       | `(a: int, b: int): int`        | Least common multiple          |
| `isPrime`   | `(n: int): bool`               | Primality test                 |
| `clamp`     | `(val: int, lo: int, hi: int): int` | Clamp to range             |

### `string` — String Utilities

| Function      | Signature                            | Description                |
|---------------|--------------------------------------|----------------------------|
| `strEq`       | `(a: string, b: string): bool`       | String equality            |
| `strEmpty`    | `(s: string): bool`                  | Check if string is empty   |
| `strRepeat`   | `(s: string, n: int): string`        | Repeat string n times      |
| `strReverse`  | `(s: string): string`                | Reverse a string           |
| `strPadLeft`  | `(s: string, n: int, ch: string): string` | Pad left to length  |
| `strPadRight` | `(s: string, n: int, ch: string): string` | Pad right to length |
| `strJoin`     | `(parts: string[], sep: string): string` | Join array with separator |
| `strCount`    | `(s: string, sub: string): int`      | Count substring occurrences|

### `array` — Array Utilities

| Function         | Signature                       | Description                    |
|------------------|---------------------------------|--------------------------------|
| `arraySum`       | `(arr: int[]): int`             | Sum of elements                |
| `arrayMax`       | `(arr: int[]): int`             | Maximum element                |
| `arrayMin`       | `(arr: int[]): int`             | Minimum element                |
| `arrayContains`  | `(arr: int[], val: int): bool`  | Check if array contains value  |
| `arrayReverse`   | `(arr: int[]): int[]`           | Reverse array                  |
| `arraySort`      | `(arr: int[]): int[]`           | Sort array (bubble sort)       |

### `fs` — File System

| Function       | Signature                          | Description                    |
|----------------|------------------------------------|--------------------------------|
| `fileExists`   | `(path: string): bool`             | Check if file exists           |
| `appendLine`   | `(path: string, line: string): void` | Append line to file         |
| `copyFile`     | `(src: string, dst: string): void` | Copy file                      |
| `mkdirP`       | `(path: string): void`             | Create directory (recursive)   |
| `listDir`      | `(path: string): string`           | List directory contents        |

### `result` — Result/Option Types

| Function   | Signature                        | Description                    |
|------------|----------------------------------|--------------------------------|
| `ok`       | `(val: string): Result`          | Create Ok variant              |
| `err`      | `(msg: string): Result`          | Create Err variant             |
| `some`     | `(val: string): Option`          | Create Some variant            |
| `none`     | `(): Option`                     | Create None variant            |
| `isOk`     | `(r: Result): bool`              | Check if Result is Ok          |
| `isErr`    | `(r: Result): bool`              | Check if Result is Err         |
| `isSome`   | `(o: Option): bool`              | Check if Option is Some        |
| `isNone`   | `(o: Option): bool`              | Check if Option is None        |
| `unwrap`   | `(r: Result): string`            | Unwrap value or print error    |

### `collections` — Data Structures

| Class  | Methods                                | Description                |
|--------|----------------------------------------|----------------------------|
| `Stack`| `push(val)`, `pop()`, `peek()`, `isEmpty()` | String-based stack  |
| `Queue`| `enqueue(val)`, `dequeue()`, `peek()`, `isEmpty()` | String-based queue |

---

## 8. Built-in Functions

These functions are available without imports.

### Output

| Function   | Signature               | Description                          |
|------------|-------------------------|--------------------------------------|
| `println`  | `(s: string): void`     | Print string + newline               |
| `print`    | `(s: string): void`     | Print string without newline         |
| `itoa`     | `(n: int): string`      | Convert integer to string            |

### String Operations

| Function      | Signature                                | Description                   |
|---------------|------------------------------------------|-------------------------------|
| `len`         | `(s: string): int`                       | String length                 |
| `charCode`    | `(s: string, i: int): int`              | Byte value at index           |
| `strcmp_ajeeb` | `(a: string, b: string): int`           | String compare (-1/0/1)       |
| `str_concat`  | `(a: string, b: string): string`        | Concatenate two strings       |
| `substring`   | `(s: string, start: int, end: int): string` | Extract substring         |
| `indexOf`     | `(s: string, search: string): int`      | Find substring position (-1 if not found) |
| `contains`    | `(s: string, search: string): int`      | Check if substring exists (returns 0/1) |
| `toUpperCase` | `(s: string): string`                   | Convert to uppercase          |
| `toLowerCase` | `(s: string): string`                   | Convert to lowercase          |
| `trim`        | `(s: string): string`                   | Trim whitespace               |
| `replace`     | `(s: string, from: string, to: string): string` | Replace substring    |
| `startsWith`  | `(s: string, prefix: string): int`      | Check prefix (returns 0/1)    |
| `endsWith`    | `(s: string, suffix: string): int`      | Check suffix (returns 0/1)    |
| `split`       | `(s: string, delim: string): string[]`  | Split string into array       |
| `strSet`      | `(s: string, i: int, c: int): void`    | Set character at index        |
| `strcpy`      | `(dst: string, src: string): void`      | Copy string content           |

### Array Operations

| Function   | Signature                    | Description                    |
|------------|------------------------------|--------------------------------|
| `arr_len`  | `(arr: T[]): int`            | Array length (type-specific)   |

### File I/O

| Function      | Signature                         | Description                   |
|---------------|-----------------------------------|-------------------------------|
| `readFile`    | `(path: string): string`          | Read file contents            |
| `writeFile`   | `(path: string, content: string): void` | Write content to file  |
| `writeAppend` | `(path: string, content: string): void` | Append to file        |
| `writeByte`   | `(path: string, byte: int): void` | Append single byte           |
| `readArg`     | `(n: int): string`               | Read program argument N       |

### System

| Function | Signature                | Description                        |
|----------|--------------------------|------------------------------------|
| `exec`   | `(cmd: string): int`     | Run shell command, return exit code|
| `mkdir`  | `(path: string): int`    | Create directory, return exit code |

### Buffer Operations

| Function     | Signature                          | Description                    |
|--------------|------------------------------------|--------------------------------|
| `getStateBuf`| `(): string`                       | Get internal state buffer      |
| `getOutbuf`  | `(): string`                       | Get output string buffer       |
| `getInt`     | `(buf: string, off: int): int`     | Read int from buffer at offset |
| `setInt`     | `(buf: string, off: int, v: int): void` | Write int to buffer    |

### Character Classification

| Function    | Signature               | Description                    |
|-------------|-------------------------|--------------------------------|
| `isDigit`   | `(c: int): int`         | Check if digit (returns 0/1)   |
| `isAlpha`   | `(c: int): int`         | Check if letter/_ (returns 0/1)|
| `isAlphaNum`| `(c: int): int`         | Check if alphanumeric (returns 0/1) |
| `isSpace`   | `(c: int): int`         | Check if whitespace (returns 0/1) |

### Testing

| Function         | Signature                     | Description                    |
|------------------|-------------------------------|--------------------------------|
| `assert_eq`      | `(a: T, b: T): void`         | Assert equality (prints error if not) |
| `assert_neq`     | `(a: T, b: T): void`         | Assert inequality              |
| `assert_contains`| `(s: string, sub: string): void` | Assert substring exists    |

### Network (Evaluator-only)

| Function       | Signature                              | Description                |
|----------------|----------------------------------------|----------------------------|
| `tcp_listen`   | `(port: int): int`                     | Listen on TCP port         |
| `tcp_accept`   | `(fd: int): int`                       | Accept TCP connection      |
| `tcp_read`     | `(fd: int, max: int): string`          | Read from TCP stream       |
| `tcp_write`    | `(fd: int, data: string): void`        | Write to TCP stream        |
| `tcp_close`    | `(fd: int): void`                      | Close TCP connection       |
| `tcp_connect`  | `(host: string, port: int): int`       | Connect to TCP server      |
| `dns_lookup`   | `(hostname: string): string`           | DNS lookup                 |

### FFI (Evaluator-only)

| Function   | Signature                              | Description                    |
|------------|----------------------------------------|--------------------------------|
| `lib_open` | `(path: string): int`                  | Open shared library            |
| `lib_sym`  | `(handle: int, name: string): int`     | Look up symbol                 |
| `lib_call` | `(fn_ptr: int, args: T[], ret: int): int` | Call C function             |

---

## 9. Structs

### Definition

```
struct User {
    name: string,
    age: int
}
```

Fields are separated by commas (not semicolons).

### Instance Creation

```
set user = User { name: "Ajeeb", age: 1 };
```

### Field Access

```
print(user.name);
user.age = 2;
```

### Struct Methods (via `impl`)

```
struct User {
    name: string,
}

impl User {
    fn greet(self) {
        print("Hello from ");
        println(self.name);
    }
}

function main() {
    set u = User { name: "Ajmal" };
    u.greet();
}
```

### Generic Structs

```
struct Box[T] {
    value: T;
}

set b: Box[Int] = Box[Int] { value: 99 };
```

---

## 10. Enums

### Definition

```
enum Color {
    Red,
    Green,
    Blue
}
```

### With Data

```
enum Option {
    Some(int),
    None
}

enum Result {
    Ok(int),
    Err(string)
}
```

### Variant Construction

```
set c = Color::Red;
set opt = Option::Some(42);
```

### Pattern Matching

```
set msg = match c {
    Color::Red => "Red",
    Color::Green => "Green",
    Color::Blue => "Blue",
};

set val = match opt {
    Option::Some(v) => v,
    Option::None => 0,
};
```

---

## 11. Traits

### Definition

```
trait Greeter {
    function greet(self: Person): String;
}
```

### Implementation

```
struct Person {
    name: String;
    age: Int;
}

impl Greeter for Person {
    function greet(self: Person): String {
        return self.name;
    }
}
```

### Trait Dispatch

```
set p = Person { name: "Alice", age: 30 };
set msg = p.greet();
```

---

## 12. Generics

### Generic Functions

```
function identity[T](x: T): T {
    return x;
}

set a = identity[Int](42);
set b = identity[String]("hello");
```

### Generic Structs

```
struct Box[T] {
    value: T;
}

set b: Box[Int] = Box[Int] { value: 99 };
```

### Generic Enums

```
enum Option[T] {
    Some(T),
    None,
}

set v = Option[Int].Some(42);
```

### Trait Bounds

```
impl[T: Comparable] Sorter[T] {
    function sort(self, arr: T[]): T[] { ... }
}
```

---

## 13. Impls (Inherent and Trait)

### Inherent Impl

Methods attached to a type directly:

```
struct User {
    name: string,
}

impl User {
    fn greet(self) {
        println(self.name);
    }
}

set u = User { name: "Ajmal" };
u.greet();
```

### Trait Impl

```
trait Greeter {
    function greet(self: Person): String;
}

impl Greeter for Person {
    function greet(self: Person): String {
        return self.name;
    }
}
```

---

## 14. Comments

```
// Single-line comment

/* Block comment
   can span multiple lines */

/* Block comments /* can be nested */ */
```

---

## 15. Limitations

1. **No global variables:** `set` at module scope is not accessible from inside functions. Use buffer slots or pass values as parameters.

2. **No forward function declarations:** `function foo(...): int;` (with `;`) is not supported. Omit the semicolon — Ajeeb resolves function references across the entire file at runtime.

3. **`set` requires initializer:** `set x: int;` is invalid. Must write `set x: int = 0;`.

4. **Duplicate `set` in same function:** Multiple `set` with the same variable name (even in different `if` branches) is a duplicate variable error. Declare once at the function top, use plain assignments (`x = value;`) in branches.

5. **`len()` is for strings only:** For arrays, use `arr_len(arr)`.

6. **`class` has a semantic analyzer bug:** The first pass doesn't register class in `struct_defs`. Prefer `struct` + `impl` over `class`.

7. **LLVM codegen `__index` limitation:** Non-constant index expressions may not work correctly in the LLVM backend.

8. **String equality:** Use `strcmp_ajeeb(a, b) == 0` for string comparison. The `==` operator compares string pointers, not contents, in the LLVM backend.

---

## 16. Example Program

```
import math;

struct User {
    name: string,
    age: int,
}

impl User {
    fn greet(self) {
        print("Hello, I'm " + self.name + " and I'm ");
        println(itoa(self.age) + " years old.");
    }
}

function factorial(n: int): int {
    if (n <= 1) { return 1; }
    return n * factorial(n - 1);
}

function main(): int {
    set u = User { name: "Ajeeb", age: 25 };
    u.greet();

    set x: int = 5;
    println("factorial(5) = " + itoa(factorial(x)));

    for (set i: int = 1; i <= 5; i = i + 1) {
        println(itoa(i) + "! = " + itoa(factorial(i)));
    }

    set colors: string[] = ["Red", "Green", "Blue"];
    set i: int = 0;
    while (i < len(colors)) {
        println(colors[i]);
        i = i + 1;
    }

    return 0;
}
```
