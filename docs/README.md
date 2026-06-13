# Ajeeb Compiler

A self-hosting compiler for the **Ajeeb** dynamic programming language, written in Ajeeb itself (with a Rust tree-walk interpreter as Stage 0).

## Architecture

```
┌──────────────────────────────────────────────────┐
│  Stage 0: Rust Interpreter (src/)               │
│  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
│  │  Lexer   │→│  Parser  │→│  Evaluator    │  │
│  │ (lexer.rs)│  │(parser.rs)│  │ (eval.rs)     │  │
│  └──────────┘  └──────────┘  └──────────────┘  │
│         ↓ executes compiler.ajb                  │
│  ┌──────────────────────────────────────────┐   │
│  │  compiler.ajb (47 functions)             │   │
│  │  → output.c (C codegen from Ajeeb src)   │   │
│  └──────────────────────────────────────────┘   │
│         ↓ GCC compile                           │
│  Stage 1: Native binary of compiler.ajb          │
│  → self-hosting: compiler.ajb compiles itself    │
└──────────────────────────────────────────────────┘
```

## Source Map

| File | Purpose |
|------|---------|
| `compiler/compiler.ajb` | **Self-hosting compiler** (1108 lines) — lexer + parser + C codegen in Ajeeb |
| `crates/ajeeb-compiler/src/main.rs` | Entry point: logo, CLI args, `.das` auto-load, Lex→Parse→Eval pipeline |
| `crates/ajeeb-compiler/src/token.rs` | Token enum: keywords, operators, literals |
| `crates/ajeeb-compiler/src/lexer.rs` | Lexer: character stream → token stream |
| `crates/ajeeb-compiler/src/parser.rs` | Parser: token stream → AST (recursive descent) |
| `crates/ajeeb-compiler/src/ast.rs` | AST types: Stmt, Expr, BinOp, TypeAnnot |
| `crates/ajeeb-compiler/src/eval.rs` | Tree-walk interpreter: RuntimeValue, builtins, user fn calls |
| `crates/ajeeb-compiler/src/codegen.rs` | LLVM IR codegen (native compilation via llc + gcc) |
| `crates/ajeeb-compiler/src/semantic.rs` | Semantic analyzer (type checking, scope resolution) |
| `crates/ajeeb-compiler/src/error.rs` | CompileError: line/col error reporting |
| `crates/ajeeb-compiler/src/das_parser.rs` | `.das` TOML-like config parser |
| `crates/ajeeb-compiler/src/interop.rs` | Cross-language FFI bridge (Python, C++) |
| `crates/parth/src/main.rs` | Package manager: new, init, build, run, test, publish, dependencies |
| `crates/parth/src/resolver.rs` | PubGrub-style dependency resolver with backtracking |
| `crates/parth/src/registry.rs` | Registry: login, download, sign, verify, cache, publish |
| `crates/parth/src/types.rs` | Version, VersionConstraint, LockEntry, Signature types |
| `crates/ajeeb-fmt/src/main.rs` | Code formatter |
| `crates/ajeeb-lsp/src/main.rs` | LSP server (diagnostics, hover, go-to-def, completions) |
| `crates/ajeeb-registry/src/main.rs` | Package registry HTTP server (axum) |
| `runtime/ajeeb_runtime.c` | C runtime (GC, string ops, file I/O, FFI stubs) |

## Language Features

- **Types**: `int`, `string`, `bool`, `void`, arrays `[]`, classes
- **Variables**: `let`, `const` with optional type annotations
- **Functions**: `function name(params): return_type { }`
- **Classes**: fields, methods, `self`, `new`
- **Enums**: variants with optional fields, pattern matching
- **Traits**: interfaces with default method implementations
- **Generics**: parametric polymorphism on functions, classes, traits
- **Control flow**: `if/else`, `while`, `for`, `match`, `return`
- **Operators**: arithmetic (`+ - * /`), comparison (`== != < > <= >=`), logical (`&& || !`)
- **Builtins**: `print`, `println`, `itoa`, `len`, `readFile`, `writeFile`, `writeAppend`, `getInt`/`setInt` (buffer I/O), `readArg`, string ops, TCP sockets, FFI

## Usage

```bash
# Run an Ajeeb source file directly (Rust interpreter)
cargo run -p ajeeb-compiler --bin ajeeb_compiler test.ajb

# Run the self-hosting compiler (generates output.c)
cargo run -p ajeeb-compiler --bin ajeeb_compiler -- compiler/compiler.ajb compiler/compiler.ajb build/output.c

# Build with package manager
parth new myapp && cd myapp && parth run
```

## Self-Hosting Bootstrap

Ajeeb is self-hosting — the compiler is written in Ajeeb itself.

### Quick Install (Self-Hosted)
```bash
bash scripts/install.sh
```

### Manual Bootstrap Steps
```bash
# Stage 0 — Rust interpreter compiles compiler.ajb → output.c
cargo run -p ajeeb-compiler --bin ajeeb_compiler -- \
    compiler/compiler.ajb compiler/compiler.ajb build/output.c

# Stage 1 — GCC compiles output.c to native binary
gcc build/output.c runtime/ajeeb_runtime.c -o build/ajeeb_native

# Stage 2 — Self-hosted! No Rust needed anymore
./build/ajeeb_native compiler/compiler.ajb build/output.c
gcc build/output.c runtime/ajeeb_runtime.c -o build/ajeeb_native
```

### Verify Bootstrap
```bash
bash tests/bootstrap_check.sh
# Expected: ✅ BOOTSTRAP SUCCESS — Self-hosting verified!
```

### Pipeline
1. **Stage 0**: Rust tree-walk interpreter runs `compiler/compiler.ajb` → produces `output.c`
2. **Stage 1**: GCC compiles `output.c` with `runtime/ajeeb_runtime.c` → `build/ajeeb_native` native binary
3. **Stage 2**: Native binary (the Ajeeb compiler, now running natively) re-compiles `compiler/ajb` → identical `output2.c`
4. **SHA256 match** proves self-hosting: `output.c ≡ output2.c`

## Status

- ✅ Rust interpreter: lexer, parser, evaluator with 30+ builtins
- ✅ Self-hosting compiler C codegen completes (~2M eval fn calls)
- ✅ `.das` config system and FFI bridge
- ✅ Bootstrap verified: Stage 1 ≡ Stage 2 SHA256 match
- ✅ Package manager (parth) with dependency resolution, registry, signing
- ✅ LSP server with diagnostics, hover, go-to-definition, completions
- ✅ Code formatter (ajeeb-fmt)
- 🔄 Performance optimization (eval per-call overhead, file I/O batching)
