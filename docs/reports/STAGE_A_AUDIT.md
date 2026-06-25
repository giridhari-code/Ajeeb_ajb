# STAGE A AUDIT — Compiler Feature Parity

## Executive Summary

**Stage A is NOT complete.** The self-hosted compiler can compile itself (bootstrap
works), but it is missing several language features that the Rust compiler supports
and that ecosystem packages (ajeeb-web, ajeeb-db, ajeeb-json, ajeeb-log) actively
use.

The self-hosted compiler handles only the subset of Ajeeb that the compiler itself
needs. To declare Stage A complete, the self-hosted compiler must handle ALL features
that the Rust compiler handles — or those features must be explicitly deferred.

## Parity Matrix

| Feature | Rust Compiler | Self-Hosted | Used by Compiler | Used by Ecosystem | Blocking Parity |
|---------|:---:|:---:|:---:|:---:|:---:|
| **Core Syntax** | | | | | |
| `function`/`fn` declarations | ✅ | ✅ | ✅ | ✅ | — |
| `set` variable declarations | ✅ | ✅ | ✅ | ✅ | — |
| `const` declarations | ✅ | ✅ | ❌ | ✅ (ajeeb-log) | — |
| `return` statements | ✅ | ✅ | ✅ | ✅ | — |
| `if`/`else` | ✅ | ✅ | ✅ | ✅ | — |
| `while` loops | ✅ | ✅ | ✅ | ✅ | — |
| `for` loops (range) | ✅ | ✅ | ❌ | ❌ | — |
| `break` | ✅ | ⚠️ tokenized only | ❌ | ❌ | — |
| `continue` | ✅ | ⚠️ tokenized only | ❌ | ❌ | — |
| `import` | ✅ | ✅ | ✅ | ✅ | — |
| **Types** | | | | | |
| `int`, `string`, `bool`, `void` | ✅ | ✅ | ✅ | ✅ | — |
| Array types `Type[]` | ✅ | ✅ | ❌ | ✅ | — |
| Custom type names | ✅ | ✅ | ✅ | ✅ | — |
| `float`/`double` | ✅ | ❌ | ❌ | ❌ | No |
| **Expressions** | | | | | |
| Arithmetic `+`,`-`,`*`,`/` | ✅ | ✅ | ✅ | ✅ | — |
| `%` modulo | ✅ | ❌ | ❌ | ❌ | No |
| `**` power | ✅ | ❌ | ❌ | ❌ | No |
| Comparison `==`,`!=`,`<`,`>`,`<=`,`>=` | ✅ | ✅ | ✅ | ✅ | — |
| Logical `&&`,`||`,`!` | ✅ | ✅ | ✅ | ✅ | — |
| Assignment `=` | ✅ | ✅ | ✅ | ✅ | — |
| String concatenation `+` | ✅ | ✅ | ✅ | ✅ | — |
| Array literals `[a, b]` | ✅ | ✅ | ❌ | ✅ | — |
| Array indexing `arr[i]` | ✅ | ✅ | ❌ | ✅ | — |
| Function calls `f(args)` | ✅ | ✅ | ✅ | ✅ | — |
| `print`/`println` multi-arg | ✅ | ✅ | ✅ | ✅ | — |
| Ternary `?:` | ✅ | ❌ | ❌ | ❌ | No |
| **OOP** | | | | | |
| `class` declarations | ✅ | ✅ | ✅ | ✅ | — |
| Class methods | ✅ | ✅ | ✅ | ✅ | — |
| Class fields | ✅ | ❌ | ❌ | ✅ (result, collections, ajeeb-web) | **YES** |
| `self` keyword | ✅ | ✅ | ✅ | ✅ (collections) | — |
| `new ClassName()` | ✅ | ✅ | ✅ | ✅ (result, ajeeb-web) | — |
| Constructor / `init` | ✅ | ❌ | ❌ | ❌ | No |
| Inheritance / `extends` | ✅ | ❌ | ❌ | ❌ | No |
| `struct` declarations | ✅ | ❌ | ❌ | ✅ (ajeeb-db, ajeeb-web, e2e) | **YES** |
| `enum` declarations | ✅ | ❌ | ❌ | ❌ | No |
| `trait` declarations | ✅ | ❌ | ❌ | ❌ | No |
| `impl` blocks | ✅ | ❌ | ❌ | ❌ | No |
| **Advanced** | | | | | |
| Generics `<T>` | ✅ | ❌ | ❌ | ❌ | No |
| Pattern matching `match` | ✅ | ❌ | ❌ | ❌ | No |
| Closures / lambdas | ✅ | ❌ | ❌ | ❌ | No |
| `type` aliases | ✅ | ❌ | ❌ | ❌ | No |
| `pub` access modifier | ✅ | ❌ | ❌ | ✅ (ajeeb-json, ajeeb-db, ajeeb-web) | **YES** |
| Global variables (module scope) | ✅ | ❌ | ❌ | ✅ (ajeeb-log, e2e) | **YES** |
| `where` clauses | ✅ | ❌ | ❌ | ❌ | No |
| Error handling `try`/`catch` | ✅ | ❌ | ❌ | ❌ | No |
| `async`/`await` | ✅ | ❌ | ❌ | ❌ | No |
| **Infrastructure** | | | | | |
| Type inference | ✅ | ❌ | N/A | N/A | No |
| Semantic analysis | ✅ | ❌ | N/A | N/A | No |
| MIR optimizer | ✅ | ❌ | N/A | N/A | No |
| LLVM codegen | ✅ | ❌ | N/A | N/A | No |
| C codegen | ✅ | ✅ | ✅ | ✅ | — |
| Interpreter | ✅ | ❌ | N/A | N/A | No |

## Feature Categories

### ✅ Fully Parity (implemented in both, used by compiler)
1. `function`/`fn` declarations
2. `set` variable declarations
3. `const` declarations
4. `return` statements
5. `if`/`else` (including `else if` chains)
6. `while` loops
7. `import` statements
8. `class` declarations with methods
9. `self` keyword
10. `new ClassName()`
11. All operators (`+`,`-`,`*`,`/`,`==`,`!=`,`<`,`>`,`<=`,`>=`,`&&`,`||`,`!`)
12. String concatenation
13. Array literals and indexing
14. Function calls
15. `print`/`println`
16. Type annotations
17. Comments (`//`, `/* */`)

### ⚠️ Partially Implemented
1. **`break`** — tokenized (45) but not dispatched in `parseStmt`. Works by accident
   (emitted as C `break` keyword via default expression handler).
2. **`continue`** — tokenized (46) but not dispatched. Same accidental behavior.
3. **Class fields** — class body parsing skips non-function members (line 241 of
   stmt.ajb does `nextTok` for non-function tokens). Field declarations are discarded.
4. **`for` loops** — parser handles `for` syntax but no .ajb file in the codebase
   actually uses `for` (all use `while`).

### ❌ Missing (not implemented in self-hosted compiler)
1. **`struct` declarations** — not tokenized, not parsed, not emitted
2. **`enum` declarations** — not tokenized, not parsed, not emitted
3. **`trait` declarations** — not tokenized, not parsed, not emitted
4. **`impl` blocks** — not tokenized, not parsed, not emitted
5. **Generics** — not tokenized, not parsed, not emitted
6. **Pattern matching** — not tokenized, not parsed, not emitted
7. **Closures/lambdas** — not tokenized, not parsed, not emitted
8. **`pub` access modifier** — not tokenized, not parsed, not emitted
9. **Global variables** (module scope) — not handled at top level
10. **`type` aliases** — not tokenized, not parsed, not emitted
11. **`%` modulo** — not tokenized
12. **`**` power** — not tokenized
13. **`float`/`double`** — not supported
14. **Forward declarations** — `emitFwdDecls` exists but is never called from `main()`

### ❌ Missing but NOT used by any .ajb file
These features exist in the Rust compiler but are NOT used by any .ajb file in the
entire codebase (compiler, stdlib, tests, packages):

| Feature | Used by .ajb files? |
|---------|---------------------|
| `enum` | ❌ No |
| `trait` | ❌ No |
| `generics` | ❌ No |
| `match`/pattern matching | ❌ No |
| `closures`/lambdas | ❌ No |
| `try`/`catch` | ❌ No |
| `async`/`await` | ❌ No |
| `type` aliases | ❌ No |
| `where` clauses | ❌ No |
| `float`/`double` | ❌ No |
| `%` modulo | ❌ No |
| `**` power | ❌ No |
| `for` loops | ❌ No (parsed but unused) |
| Ternary `?:` | ❌ No |
| Inheritance | ❌ No |
| Constructors | ❌ No |

## Critical Finding

**The Rust compiler's most advanced features (generics, traits, enums, pattern
matching, closures) are NOT used by any .ajb file in the codebase.** The entire
Ajeeb ecosystem — compiler, standard library, packages, tests — is written using
only: functions, set/const, if/else, while, class with methods, self, new, import,
arrays, and string operations.

This means Stage A "feature parity" can be defined in two ways:

1. **Strict parity**: Implement everything the Rust compiler has → massive work
2. **Practical parity**: Implement everything that .ajb code actually uses → small work

The recommended path is **practical parity**: implement only what's needed for the
ecosystem to compile, and defer unused features to later stages.
