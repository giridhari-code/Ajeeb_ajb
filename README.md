# Ajeeb Compiler (অজীব কম্পাইলার)

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
| `src/main.rs` | Entry point: logo, CLI args, `.das` auto-load, Lex→Parse→Eval pipeline |
| `src/token.rs` | Token enum: keywords, operators, literals |
| `src/lexer.rs` | Lexer: character stream → token stream |
| `src/parser.rs` | Parser: token stream → AST (recursive descent) |
| `src/ast.rs` | AST types: Stmt, Expr, BinOp, TypeAnnot |
| `src/eval.rs` | Tree-walk interpreter: RuntimeValue, builtins, user fn calls |
| `src/error.rs` | CompileError: line/col error reporting |
| `src/das_parser.rs` | `.das` TOML-like config parser |
| `src/interop.rs` | Cross-language FFI bridge (Python, C++) |
| `compiler.ajb` | **Self-hosting compiler** (889 lines) — lexer + parser + C codegen in Ajeeb |
| `main.ajb` | OOP demo (Counter class) |
| `ajeeb.das` | Sample project config |

## Language Features

- **Types**: `int`, `string`, `bool`, `void`, arrays `[]`, classes
- **Variables**: `let`, `const` with optional type annotations
- **Functions**: `function name(params): return_type { }`
- **Classes**: fields, methods, `self`, `new`
- **Control flow**: `if/else`, `while`, `return`
- **Operators**: arithmetic (`+ - * /`), comparison (`== != < > <= >=`), logical (`&& || !`)
- **Builtins**: `print`, `println`, `itoa`, `len`, `readFile`, `writeFile`, `writeAppend`, `getInt`/`setInt` (buffer I/O), `readArg`, string ops

## Usage

```bash
# Run an Ajeeb source file directly (Rust interpreter)
cargo run test.ajb

# Run the self-hosting compiler (generates output.c)
cargo run compiler.ajb

# With custom .das config
cargo run compiler.ajb my_config.das
```

## Self-Hosting Pipeline

1. **Stage 0**: Rust interpreter runs `compiler.ajb` → produces `output.c`
2. **Stage 1**: `output.c` compiled with GCC → native binary
3. **Stage 2**: Native binary compiles `compiler.ajb` on itself → fully self-hosted

## Status

- ✅ Rust interpreter: lexer, parser, evaluator with 30+ builtins
- ✅ Self-hosting compiler C codegen completes (~2M eval fn calls)
- ✅ `.das` config system and FFI bridge
- 🔄 Performance optimization (eval per-call overhead, file I/O batching)
- 🔄 GCC verification of generated `output.c`
