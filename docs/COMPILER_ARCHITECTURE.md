# Ajeeb Compiler Architecture

## Overview

Ajeeb is a self-hosting compiler for the Ajeeb programming language. The Rust compiler (`ajeeb-compiler`) compiles `.ajb` source files through a multi-stage pipeline, with the final stage targeting either LLVM IR (compiled to native via `llc`/`as`/`ld`) or C source (compiled via GCC). The compiler can also run programs directly via a tree-walking interpreter.

## Pipeline Stages

```
Source (.ajb) → Lexer → Parser → AST → Semantic → HIR → THIR → MIR → Codegen
                                                                            ├─→ LLVM IR → llc → as → ld → native binary
                                                                            └─→ C source → gcc → native binary
```

1. **Lexer** (`lexer.rs`, `token.rs`): Character-level tokenization. Produces `Vec<Token>` with span information (line, column).

2. **Parser** (`parser/`): Token-level parsing. Produces `Vec<Stmt>` (the AST). Handles expressions, statements, declarations, type annotations, generics, and pattern matching.

3. **Module Loading** (`module.rs`): Resolves `import` declarations by finding `.ajb` files on configured import paths. Recursively lexes/parses imported files. Flattens all statements into a single list for semantic analysis.

4. **Semantic Analysis** (`semantic/`): Name resolution and type checking on the AST. Registers function signatures, struct definitions, enum variants, and trait implementations in a type environment. Reports type errors.

5. **HIR Lowering** (`hir_lower.rs`): Desugars AST into High-Level IR. Removes syntactic sugar (e.g., `for` loops → `while` loops, method calls → function calls). Resolves struct/enum types. Produces `HirProgram`.

6. **THIR Check** (`thir.rs`): Typed HIR verification. Walks the HIR and verifies all expressions have correct types, function call arguments match signatures, struct field access is valid, and enum patterns are exhaustive. Pure validation — does not transform the IR.

7. **MIR Lowering** (`thir_to_mir.rs`): Converts HIR to Mid-Level IR — a CFG-based representation with basic blocks, SSA-like assignments, and explicit terminators (goto, switch, return). Resolves control flow to block indices.

8. **MIR Optimization** (`mir.rs`): Constant folding and dead block elimination on the MIR.

9. **Codegen**: Two backends, both operating on MIR:
   - **LLVM** (`llvm/`): Generates LLVM IR strings. Linked with the C runtime (`runtime/ajeeb_runtime.c`) via `llc` → `as` → `cc`.
   - **C** (`c_codegen.rs`): Generates C source strings. Compiled with GCC. Serves as fallback when LLVM is unavailable.

## Module Structure

```
ajeebc/crates/ajeeb-compiler/src/
├── main.rs                  # CLI entry point, pipeline orchestration
├── lib.rs                   # Public module re-exports
├── ast.rs                   # AST node definitions (Stmt, Expr, TypeAnnot, Pattern, etc.)
├── token.rs                 # Token enum (Keywords, Literals, Operators, Delimiters)
├── lexer.rs                 # Lexer: source → Vec<Token>
├── error.rs                 # CompileError type
├── module.rs                # Module loader (import resolution)
├── das_parser.rs            # Parth .das config file parser
├── hir.rs                   # HIR node definitions (HirProgram, HirFn, HirStmt, HirExpr, HirType)
├── hir_lower.rs             # AST → HIR lowering
├── thir.rs                  # THIR type checker (validation pass)
├── thir_to_mir.rs           # HIR → MIR lowering (CFG construction)
├── mir.rs                   # MIR node definitions + optimization passes
├── interop.rs               # FFI interop helpers
├── cache/
│   ├── mod.rs               # Module cache (serialize/deserialize AST to disk)
│   └── serialize.rs         # Binary serialization format for cached ASTs
├── parser/
│   ├── mod.rs               # Parser entry point, token stream management
│   ├── decls.rs             # Declaration parsing (fn, struct, enum, trait, impl, import)
│   ├── expr.rs              # Expression parsing (precedence climbing)
│   ├── stmt.rs              # Statement parsing (if, while, for, return, set, etc.)
│   ├── types.rs             # Type annotation parsing
│   ├── generics.rs          # Generic type parameter parsing
│   └── patterns.rs          # Pattern matching parsing
├── semantic/
│   ├── mod.rs               # SemanticAnalyzer entry, name resolution pass
│   ├── typecheck.rs         # Type checking pass
│   ├── generics.rs          # Generic instantiation checking
│   ├── traits.rs            # Trait resolution (stub)
│   └── modules.rs           # Module-level semantic analysis (stub)
├── llvm/
│   ├── mod.rs               # LLVM Codegen entry, IR buffer management
│   ├── mir.rs               # MIR → LLVM IR: function/block/instruction emission
│   ├── expr.rs              # Expression codegen (binary ops, calls, indexing)
│   ├── stmt.rs              # Statement codegen (if, while, for, return)
│   ├── strings.rs           # String literal management and runtime call codegen
│   ├── types.rs             # LLVM type mapping (i64, double, i8*, etc.)
│   ├── methods.rs           # Method call codegen (struct methods, string methods)
│   └── generic.rs           # Generic function monomorphization
├── c_codegen.rs             # MIR → C source code generation
└── eval/
    ├── mod.rs               # Evaluator entry, program execution
    ├── expr.rs              # Expression evaluation
    ├── stmt.rs              # Statement execution
    ├── builtins.rs          # Built-in functions (println, len, strSet, etc.)
    ├── functions.rs         # User-defined function dispatch
    ├── traits.rs            # Trait dispatch (stub)
    └── modules.rs           # Module-level eval (stub)
```

**Total: 44 source files** (41 compiler + main.rs + lib.rs + das_parser.rs)

## Key Data Structures

### AST (`ast.rs`)

The abstract syntax tree mirrors source-level syntax directly.

- **`Stmt`**: Top-level and block statements — `Function`, `Set`, `Return`, `If`, `While`, `For`, `Expr`, `Struct`, `Enum`, `Trait`, `Impl`, `Import`, `Break`, `Continue`
- **`Expr`**: Expressions — `Int`, `Float`, `Str`, `Bool`, `Var`, `BinOp`, `UnaryOp`, `Call`, `Index`, `FieldAccess`, `MethodCall`, `ArrayLit`, `StructLit`, `Match`, `Lambda`
- **`TypeAnnot`**: Type annotations — `Int`, `Float`, `String`, `Bool`, `Void`, `Array`, `Class`, `Generic`, `Parameterized`
- **`Pattern`**: Match patterns — `Literal`, `Variable`, `Wildcard`, `EnumVariant`, `StructDestructure`

### HIR (`hir.rs`)

High-Level IR — desugared, named types resolved, but still tree-structured (no explicit control flow).

- **`HirProgram`**: `functions: Vec<HirFn>`, `structs`, `enums`, `impls`, `traits`, `globals`
- **`HirFn`**: `name`, `params: Vec<(String, HirType)>`, `return_type`, `body: Vec<HirStmt>`, `type_params`
- **`HirType`**: `Int`, `Float`, `Bool`, `Str`, `Void`, `Named(String)`, `Array`, `Generic`, `Unknown`
- **`HirExpr`**: Typed expressions with `ty: HirType` annotations on every node
- **`HirStmt`**: `Set`, `Return`, `If`, `While`, `For`, `Expr`, `Break`, `Continue`

### THIR (`thir.rs`)

Typed HIR — same structure as HIR, but verified by `ThirChecker`. The checker maintains:
- `fn_signatures: HashMap<String, (Vec<HirType>, HirType)>`
- `struct_fields: HashMap<String, Vec<(String, HirType)>>`
- `enum_variants: HashMap<String, Vec<(String, Vec<HirType>)>>`
- `type_env: HashMap<String, HirType>` (local variable types)

### MIR (`mir.rs`)

Mid-Level IR — control-flow graph with basic blocks, SSA-like assignments, explicit terminators.

- **`MirProgram`**: `functions: Vec<MirFn>`, `structs`, `enums`
- **`MirFn`**: `name`, `params`, `return_type`, `blocks: Vec<BasicBlock>`, `locals`
- **`BasicBlock`**: `id: usize`, `statements: Vec<MirStmt>`, `terminator: Terminator`
- **`MirStmt`**: `Assign { dest, value }`, `Call { dest, func, args }`
- **`MirRvalue`**: `Use(MirOperand)`, `BinaryOp(MirBinOp, MirOperand, MirOperand)`, `Const(MirConst)`
- **`Terminator`**: `Goto(usize)`, `SwitchInt { cond, targets, default }`, `Return(Option<MirOperand>)`, `Unreachable`
- **`MirBinOp`**: `Add`, `Sub`, `Mul`, `Div`, `Eq`, `Neq`, `Lt`, `Gt`, `Le`, `Ge`, `And`, `Or`
- **`MirConst`**: `Int(i64)`, `Float(f64)`, `Str(String)`, `Bool(bool)`

**MIR Optimizations** (in `mir.rs`):
- `constant_fold`: Evaluates binary ops on two constants at compile time
- `dead_block_elim`: Removes unreachable blocks (no predecessor)

## Entry Point (`main.rs`)

CLI usage: `ajeeb_compiler <input.ajb> [output] [--llvm|--gcc|--interpret] [--run] [--skip-compile]`

Backend detection order:
1. `--llvm` → LLVM
2. `--gcc` → GCC/C codegen
3. Auto-detect: checks for `llc`, then `gcc`, falls back to interpreter

Execution flow:
1. Read source file
2. Lex → tokens
3. Parse → AST
4. Module loading → resolve imports, flatten all statements
5. Semantic analysis → type/scope checking
6. HIR lowering
7. THIR type verification
8. MIR lowering + optimization
9. Interpreter run (if `--interpret`/`--run` or backend is interpreter)
10. Codegen → LLVM IR or C → native binary

## Interpreter Mode

The evaluator (`eval/`) is a tree-walking interpreter that operates directly on the AST. It evaluates expressions, executes statements, and manages a stack-based environment. Built-in functions (61+ in `eval/builtins.rs`) provide I/O, string manipulation, array operations, and system calls. The interpreter is used when no compiler backend is available, or when `--interpret`/`--run` is passed.

## Codegen Modes

### LLVM Backend (`llvm/`)

Generates LLVM IR targeting the `i64`-boxed value representation (all Ajeeb values are boxed as `i64` — ints unboxed, floats bitcast, strings as `i8*` pointers). The `Codegen` struct maintains an IR buffer and emits:
- Runtime function declarations (`declare i64 @println(i64)`, etc.)
- Global string constants
- Function definitions with basic blocks
- Instructions: `add`, `sub`, `mul`, `sdiv`, `icmp eq`, `call`, `br`, `ret`, `getelementptr`, `load`/`store`, `bitcast`

Compilation pipeline: `output.ll` → `llc -O2` → `output.s` → `as` → `output.o` → `cc` with `runtime.o` → binary.

### C Backend (`c_codegen.rs`)

Generates C source from MIR. Each MIR function becomes a C function. Basic blocks become `goto`-based control flow. Values are boxed as `long` (matching the runtime's `int64_t`). Compiled with: `gcc output.c runtime.c -o binary -ldl -lm`.

## Module System

Ajeeb uses file-based modules with `import` declarations:

```
import math;           // resolves to packages/ajeeb-std/math.ajb
import io;             // resolves to packages/ajeeb-std/io.ajb
import lexer;          // resolves to compiler/lexer.ajb (relative to entry file)
```

**Resolution** (`module.rs`):
1. The entry file is loaded as the root module
2. Import paths are searched in order: entry file directory, `packages/ajeeb-std/`, `../packages/ajeeb-std/`
3. Each import lexes and parses the target file recursively
4. All statements from imported modules are flattened into the root module's statement list
5. Circular imports are detected via a loading stack

**Caching** (`cache/`): Parsed ASTs are serialized to `build/cache/` for incremental builds. On subsequent runs, if source files haven't changed, the cached AST is loaded directly, skipping lexing/parsing.

## Runtime (`runtime/ajeeb_runtime.c`)

A 1,452-line C runtime providing:
- Value boxing/unboxing (`BOX_INT`, `UNBOX_INT`, `BOX_FLOAT`, etc.)
- Memory management (arena allocator)
- String operations (`strcmp_ajeeb`, `str_concat`, `substring`, `indexOf`, etc.)
- I/O (`println`, `readFile`, `writeFile`)
- System calls (`exec`, `mkdir`)
- Collections (`array_to_string`)
- Cross-platform support via `#ifdef` for Linux, macOS, Windows
