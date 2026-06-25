# Release Notes — Ajeeb v1.0.0

**First Stable Release**

Ajeeb v1.0.0 is the first production-ready release of the Ajeeb programming language — a self-hosting, TypeScript-inspired language that compiles itself, with Hinglish error messages.

## Highlights

- **Self-hosting verified** — The compiler compiles itself with identical output
- **99% smaller binaries** — 15MB Rust bootstrap → 142KB native compiler
- **No Rust required** — Normal development uses pre-built binaries
- **Production-ready LLVM backend** — Fast, optimized code generation
- **Package manager** — Parth for project management and packages

## New Features

### Compiler

- **Self-hosting bootstrap chain** — Gen0 (Rust) → Gen1 (Ajeeb) → Gen2 (Ajeeb) produces identical output
- **LLVM backend** — Primary backend with full optimization
- **C fallback backend** — For platforms without LLVM
- **Interpreter mode** — Direct execution without compilation
- **Multi-file imports** — Proper module system with forward declarations
- **Type checking** — Full type system with trait bounds
- **Pattern matching** — Match expressions with exhaustive checking
- **Generics** — Type parameters with constraints
- **Classes and structs** — Object-oriented programming
- **Enums** — Algebraic data types
- **Traits** — Interface-like type constraints
- **Closures** — First-class functions
- **Arrays** — Dynamic arrays with built-in functions
- **String operations** — concat, substring, indexOf, contains, etc.
- **65+ built-in functions** — Math, I/O, strings, arrays, files

### Parth (Package Manager)

- **Project initialization** — `parth init`
- **Build system** — `parth build`
- **Test runner** — `parth test`
- **Interpreter mode** — `parth run`
- **Self-hosting verification** — `parth bootstrap`
- **Package registry** — Publish and install packages
- **Dependency resolution** — Automatic dependency management
- **Workspace support** — Multi-package projects
- **Lockfile support** — Reproducible builds
- **Cache management** — Local package cache
- **Package signing** — Ed25519 signatures
- **Security audit** — Vulnerability scanning

### MIR Interpreter

- **Direct AST execution** — No LLVM required
- **Fast development cycle** — Instant feedback
- **Full language support** — All features supported

### Standard Library

- **io.ajb** — Input/output operations
- **math.ajb** — Mathematical functions (abs, max, min, pow, etc.)
- **string.ajb** — String utilities (strEq, strRepeat, strReverse, etc.)
- **array.ajb** — Array utilities (arraySum, arrayMax, arraySort, etc.)
- **fs.ajb** — File system operations (fileExists, appendLine, etc.)
- **result.ajb** — Result/Option types
- **collections.ajb** — Stack, Queue data structures

## Compiler Improvements

- **LLVM backend** — Production-ready with optimizations
- **C fallback** — For platforms without LLVM
- **Type inference** — Better type inference
- **Error messages** — Bilingual Hinglish error messages
- **Performance** — 0.9s compile time for single files
- **Memory usage** — ~5MB for compiler, ~5MB for runtime

## Parth Improvements

- **Project templates** — Quick project setup
- **Dependency management** — Automatic resolution
- **Build caching** — Faster rebuilds
- **Test integration** — Built-in test runner
- **Registry support** — Package publishing and installation

## Breaking Changes

None — this is the first stable release.

## Known Limitations

- **C backend** — Cannot compile full compiler.ajb (variable limit)
- **Global variables** — Not supported in self-hosted code
- **Forward declarations** — Required for imported functions
- **`set` requires initializer** — `set x: int;` invalid, must use `set x: int = 0;`
- **LLVM `__index`** — Non-constant index expressions not fully supported

## Performance

| Metric | Value |
|--------|-------|
| Compile time (single file) | 0.9s |
| Full compiler build | 2.3s |
| Bootstrap chain | 48s |
| Binary size (compiler) | 142 KB |
| Memory usage | ~5 MB |

## Platforms

- Linux x86_64
- Linux aarch64
- macOS x86_64
- macOS aarch64
- Windows x86_64

## Installation

```bash
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
```

## Quick Start

```bash
# Create a new project
parth init hello-ajeeb
cd hello-ajeeb

# Run the project
parth run

# Or compile and run manually
ajeebc src/main.ajb --skip-run
llc build/output.ll -o build/output.s
gcc build/output.s runtime/ajeeb_runtime.c -o build/hello
./build/hello
```

## What's Next

- v1.1.0: Enhanced pattern matching, closures improvements
- v1.2.0: Async/await support
- v1.3.0: Improved error messages, better IDE support

## Thank You

Thank you to all contributors who made this release possible!

## License

MIT
