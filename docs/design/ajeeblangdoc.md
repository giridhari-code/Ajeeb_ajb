# Ajeeb Language Reference

## Compile & Run
```bash
cd ajeebc
cargo run -p ajeeb-compiler --bin ajeeb_compiler path/to/file.ajb
./build/<filename>        # run compiled binary
```

---

## 1. Variables

```ajb
set x: int = 10;           // mutable
const y: string = "hello"; // immutable

set z = 42;                // type optional
set w: int[] = [1, 2, 3];  // array
```

**⚠️ Gotchas:**
- `set x: int;` ❌ (must have initializer) → `set x: int = 0;` ✅
- Same variable name declared twice in one function = error
- No `++`, `--`, `+=`, `-=` etc. Use `i = i + 1;`

---

## 2. Types

| Type | Example | Notes |
|------|---------|-------|
| `int` | `42` | All ints are 64-bit |
| `string` | `"hello"` | Arena-allocated |
| `bool` | `true` / `false` | Same as int (1/0) |
| `void` | — | Return type only |
| `int[]` | `[1, 2, 3]` | Array (LLVM codegen may not support) |

---

## 3. Functions

```ajb
function add(a: int, b: int): int {
    return a + b;
}

fn greet(name: string): void {
    println("Hello, " + name);
}

function main(): int {
    return 0;
}
```

- No forward declarations — Ajeeb collects all functions first, then runs
- `main()` is auto-called, returns `int`

---

## 4. Control Flow

```ajb
// If-else
if (x > 0) {
    println("positive");
} else if (x == 0) {
    println("zero");
} else {
    println("negative");
}

// While
while (i < 10) {
    i = i + 1;
}

// For
for (set i: int = 0; i < 10; i = i + 1) {
    println(itoa(i));
}
```

---

## 5. Operators (precedence high→low)

| Operator | Description |
|----------|-------------|
| `-` `!` | Unary negate, NOT |
| `*` `/` | Multiply, divide |
| `+` `-` | Add, subtract (or string concat) |
| `<` `>` `<=` `>=` | Comparison |
| `==` `!=` | Equality |
| `&&` | Logical AND |
| `\|\|` | Logical OR |
| `=` | Assignment |

**⚠️ String equality:** Use `strcmp_ajeeb(a, b) == 0` — `==` compares pointers in LLVM!

---

## 6. Built-in Functions

### Print / I/O
```ajb
println("hello");          // print with newline
print("no newline");
writeFile("path", content);
writeAppend("path", more);
readFile("path");          // returns "" if not found
readArg(1);                // command-line argument
```

### String operations
```ajb
len(s);                     // string length
charCode(s, i);             // char at index
strSet(s, i, 65);           // set char at index (writes 'A')
substring(s, start, end);   // [start, end)
indexOf(s, "search");       // position or -1
contains(s, "sub");         // 1 if found
strcmp_ajeeb(a, b);         // 0 = equal (use this for string compare!)
str_concat(a, b);           // concatenate
itoa(42);                   // int → string
toUpperCase(s);
toLowerCase(s);
trim(s);
startsWith(s, "pre");
endsWith(s, "suf");
replace(s, "old", "new");
```

### System
```ajb
exec("ls -la");             // run shell command, returns exit code
mkdir("/tmp/x");            // mkdir -p, returns exit code
```

---

## 7. Classes (Legacy)
```ajb
class Point {
    x: int;
    y: int;
    function move(self, dx: int, dy: int): void {
        self.x = self.x + dx;
        self.y = self.y + dy;
    }
}
```

---

## 8. Common Gotchas Summary
1. **`==` on strings** → use `strcmp_ajeeb(a, b) == 0` (LLVM bug)
2. **`set x: int;`** → invalid, must init: `set x: int = 0;`
3. **No `++` / `+=`** → use `i = i + 1;`
4. **No dynamic arrays** → `int[]` literal works but no `.push()`
5. **No global vars in self-hosted** → can't reference module-level `set` inside functions
6. **Duplicate `set`** → same variable name twice in one function = error
7. **Division by zero** → returns 0, doesn't crash
8. **`strSet` LLVM bug** → `strSet(buf, 2, 65)` may not work for non-sequential indices
9. **`__array_lit`** not in LLVM codegen → some array features may not work

---

## 9. Quick Template

```ajb
function main(): int {
    set name: string = readArg(1);
    if (len(name) == 0) {
        name = "world";
    }
    println("Hello, " + name + "!");
    return 0;
}
```
