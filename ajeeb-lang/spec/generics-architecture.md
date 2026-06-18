# Ajeeb Generics — Architecture & Implementation Plan

## 1. Compiler Audit Summary

### 1.1 Current Architecture

| Layer | File | Lines | Description |
|-------|------|-------|-------------|
| Token | `token.rs` | 61 | 61 token variants, no generic-related tokens |
| Lexer | `lexer.rs` | 323 | Hand-written, single-pass, Unicode chars |
| AST | `ast.rs` | 260 | `TypeAnnot` enum, `Stmt` enum (11 variants), `Expr` enum (24 variants) |
| Parser | `parser.rs` | 1150 | Recursive descent, Pratt-style expression parsing |
| Semantic | `semantic.rs` | 744 | Name resolution + type checking, `types_match` for subtyping |
| Eval | `eval.rs` | 1528 | AST interpreter + built-in fns + C codegen via WASM |
| Module | `module.rs` | 222 | File-based module loader, topological sort |
| Error | `error.rs` | 24 | Simple line+col+message error type |
| Main | `main.rs` | 178 | CLI entry: lex → parse → module load → semantic → exec |

### 1.2 Key Data Types

```rust
pub enum TypeAnnot {
    Int, Float, String, Bool, Void,
    Array(Box<TypeAnnot>),
    Class(String),         // User-defined type name
}
```

**Critical observation**: `TypeAnnot` has no mechanism for type parameters. There is no `Generic(T)` or `Parameterized { base, args }`.

```rust
pub enum Stmt {
    Let, Const, If, While, ForLoop, Break, Continue, Return,
    Expr(Expr),
    FnDef { name, params: Vec<(String, TypeAnnot)>, return_type: TypeAnnot, body, pub_, line, col },
    Class { name, fields: Vec<ClassField>, methods, pub_, line, col },
    StructDef { name, fields: Vec<StructField>, pub_, line, col },
    EnumDef { name, variants: Vec<EnumVariantDef>, pub_, line, col },
    Import(ImportDecl),
}
```

**Critical observation**: No `type_params: Vec<String>` on `FnDef`, `StructDef`, or `EnumDef`.

### 1.3 Runtime Value Representation

```rust
pub enum RuntimeValue {
    Int(i64), Float(f64), String(Rc<RefCell<String>>), Bool(bool), Void,
    Array(Rc<RefCell<Vec<RuntimeValue>>>),
    ClassInstance { class_name: String, fields: HashMap<String, RuntimeValue> },
    StructInstance { name: String, fields: HashMap<String, RuntimeValue> },
    EnumVariant { enum_name: String, variant: String, data: Vec<RuntimeValue> },
    Return(Box<RuntimeValue>), Break, Continue,
}
```

**Good**: Runtime uses `RuntimeValue` enum — generic functions work without monomorphization at the interpreter level.

---

## 2. Architectural Blockers

### 2.1 Blocker: Angle Bracket Conflict (Severity: HIGH)

`<` and `>` are already `BinOp::Lt` and `BinOp::Gt`. Using `<T>` for generics would require the parser to distinguish:
- `x < y` → comparison (binary expression)
- `List<Int>` → generic parameter (type context)

**Resolution strategy**: Use `[T]` syntax for generics in type positions. Square brackets are currently only used for array indexing `arr[i]` and array literals `[1, 2, 3]` — they are unambiguous in type annotation context because arrays use postfix `[]` (e.g., `Int[]`).

### 2.2 Blocker: `TypeAnnot` Has No Generic Representation (Severity: HIGH)

`TypeAnnot` must grow to support:
- `TypeAnnot::Generic(String)` — a type parameter name (e.g., `T`)
- `TypeAnnot::Parameterized { base: Box<TypeAnnot>, args: Vec<TypeAnnot> }` — e.g., `List[Int]`

### 2.3 Blocker: No Type Parameter Tracking (Severity: HIGH)

`FnDef`, `StructDef`, `EnumDef` need `type_params: Vec<String>` fields.

### 2.4 Blocker: No Type Substitution Logic (Severity: HIGH)

Semantic analyzer needs to:
- Track which type params are in scope
- Substitute concrete types for type params at call sites
- Check bounds (future: Phase 2 with traits)

### 2.5 Blocker: Parser Cannot Parse Generics (Severity: HIGH)

`parse_type()` must handle `List[Int]` and `T` (where `T` is in scope as a generic param).

### 2.6 Non-Blocker: Runtime Interpreter

Generic functions work automatically at the interpreter level because `RuntimeValue` already provides dynamic dispatch. No changes needed in `eval.rs` for basic generics.

### 2.7 Non-Blocker: C Codegen

The C compiler path (via `src/main.ajb` self-hosted compiler) does monomorphization by generating C code per concrete type. This is out of scope for Phase 1.

---

## 3. Implementation Plan: Phase 1 (Generics)

### 3.1 Phase 1 Syntax Design

```ajeeb
// Generic function
function identity[T](x: T): T {
    return x;
}

// Generic struct
struct Box[T] {
    value: T;
}

// Generic enum
enum Option[T] {
    Some(T),
    None,
}

// Usage
let b: Box[Int] = Box { value: 42 };
let result = identity[Int](42);
```

Design decisions:
- `[T]` after function/struct/enum name declares type params
- `TypeName[Arg]` instantiates a generic type
- `FnName[Arg](...)` calls a generic function with explicit type args
- Type inference for generics is deferred (Phase 1 requires explicit type args)

### 3.2 AST Changes

```rust
// Add to TypeAnnot:
pub enum TypeAnnot {
    // ... existing variants ...
    Generic(String),                          // Type parameter reference: T
    Parameterized {                           // Instantiated generic: List[Int]
        base: Box<TypeAnnot>,
        args: Vec<TypeAnnot>,
    },
}

// Add type_params to definitions:
pub enum Stmt {
    FnDef {
        name: String,
        type_params: Vec<String>,             // NEW
        params: Vec<(String, TypeAnnot)>,
        return_type: TypeAnnot,
        body: Vec<Stmt>,
        pub_: bool,
        line: usize, col: usize,
    },
    StructDef {
        name: String,
        type_params: Vec<String>,             // NEW
        fields: Vec<StructField>,
        pub_: bool,
        line: usize, col: usize,
    },
    EnumDef {
        name: String,
        type_params: Vec<String>,             // NEW
        variants: Vec<EnumVariantDef>,
        pub_: bool,
        line: usize, col: usize,
    },
    // Class changes deferred — classes are already complex
}
```

### 3.3 Lexer Changes

No new tokens needed. `[` and `]` already exist.

### 3.4 Parser Changes

**`parse_type()`** (currently line 183-217):
```
After reading base type name:
  if peek == '[' :
    advance('[')
    args = parse_type_args()   // comma-separated types
    expect(']')
    if base is identifier and known as generic param:
      return Parameterized { base: Generic(param), args }
    else:
      return Parameterized { base, args }
  else if peek == '[' (for arrays):
    // existing Array handling
```

**`parse_fn_def()`** (currently line 431-474):
```
After reading function name:
  if peek == '[' :
    advance('[')
    type_params = read comma-separated identifiers
    expect(']')
  // rest unchanged, but params and return_type can reference type_params
```

**`parse_struct_def()`** and **`parse_enum_def()`**: Same pattern.

**`parse_primary()`** — handle `Ident[...]` as generic function call / type instantiation:
```
After reading identifier:
  if peek == '[' :
    // This could be:
    // 1. Generic function call: identity[Int](42)
    // 2. Generic type in expression: SomeType[Int]{...}
    // Parse args, then check what follows
```

### 3.5 Semantic Analyzer Changes

**In `analyze()` first pass**: Collect generic function signatures with their type params.

**In `check_stmt()` / `infer_expr_type()`**:
- When entering a generic function scope, add type params as valid types (not variables)
- `Generic(name)` resolves to itself (type param is valid in scope)
- At call site: record the concrete type args for checking

**Type matching**:
- `Generic(a)` matches `Generic(a)` (same name) — used when checking function body
- `Parameterized { base, args }` matches structurally
- During monomorphization (future): substitute `Generic(name)` with concrete type

### 3.6 Evaluator Changes

**Minimal changes needed**:
- `RuntimeValue` already handles all types dynamically
- Generic function calls: store type params but ignore at runtime (same compiled body works for all types)
- `Expr::FnCall` with `[args]` prefix: parse type args but don't pass to evaluator

**Actually for Phase 1, simplest approach**: In the evaluator, just ignore type arguments. Since `RuntimeValue` already supports all types, `identity(x)` works regardless of whether `T` is `Int`, `String`, etc. The type params serve only static checking purposes.

### 3.7 Implementation Order

| Step | Description | Files | Effort |
|------|-------------|-------|--------|
| 1 | Add `Generic` and `Parameterized` to `TypeAnnot` | `ast.rs` | Small |
| 2 | Add `type_params` to `FnDef`, `StructDef`, `EnumDef` | `ast.rs` | Small |
| 3 | Update `parse_type()` for `Generic` and `Parameterized` | `parser.rs` | Medium |
| 4 | Update `parse_fn_def()` to read `[T]` type params | `parser.rs` | Small |
| 5 | Update `parse_struct_def()` / `parse_enum_def()` for type params | `parser.rs` | Small |
| 6 | Update `parse_call()` for generic fn calls `fn[T](args)` | `parser.rs` | Medium |
| 7 | Update semantic: track type params in scope, basic checking | `semantic.rs` | Large |
| 8 | Update eval: strip type args, ignore for runtime | `eval.rs` | Small |
| 9 | Update formatter for new AST nodes | `formatter.rs` | Medium |
| 10 | Write tests for all generic features | `tests/*.ajb` | Medium |
| 11 | Run full test suite, fix regressions | — | Medium |

### 3.8 Estimated Total: ~400-600 lines changed

---

## 4. Post-Phase 1 Roadmap

### Phase 2: Traits/Interfaces
- Add `TraitDef` and `Impl` to AST
- Trait bounds on type params
- Dynamic dispatch via trait object vtables
- Required: method lookup by trait, impl matching

### Phase 3: Error System
- `Result[T, E]` and `Option[T]` as enum generics
- `try` operator: desugar to match
- Stack traces: capture in RuntimeValue::Return

### Phase 4: Async
- Requires: continuation support or event loop
- `async fn`, `await` expression
- Futures as poll-based state machines
- TCP built on async

### Phase 5: FFI
- `extern` block syntax
- C ABI compatibility layer
- Dynamic library loading

### Phase 6: Tooling
- Formatter (in progress)
- LSP: `ajeeb-lsp` crate
- Diagnostics

### Phase 7: VSCode Extension
- Syntax highlighting
- LSP client
- Snippets

### Phase 8: Registry
- Package hosting
- Search
- Docs hosting
