# Ajeeb

A self-hosting programming language that compiles itself — with Hinglish error messages.

## What is Ajeeb?

Ajeeb (अजीब, meaning "strange" in Hindi) is a TypeScript-inspired programming language that bootstrapped from Rust and now compiles its own compiler. File extension: `.ajb`

Like Rust bootstrapped from OCaml, Ajeeb bootstrapped from Rust — and now produces identical output from its self-hosted compiler.

## Features

- **TypeScript-style syntax** — familiar, modern, clean
- **Static typing** — int, string, bool, float, void
- **Classes and structs** — object-oriented programming
- **Enums and pattern matching** — match expressions
- **Traits** — interface-like type constraints
- **Generics** — type parameters
- **Arrays** — dynamic arrays with built-in functions
- **Closures** — first-class functions
- **Hinglish error messages** — fun, bilingual compiler output
- **Self-hosting** — compiler compiles itself

## Installation

### Quick Install (Recommended)

```bash
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/scripts/install.sh | bash
```

### Manual Install

1. Download the latest release for your platform
2. Add to PATH:
   ```bash
   export PATH="$HOME/.ajeeb/bin:$PATH"
   ```

### Requirements

- `gcc` or `clang` — for linking native binaries
- `llc` (LLVM) — for assembly generation

```bash
# Ubuntu/Debian
sudo apt install gcc llvm

# Fedora
sudo dnf install gcc llvm

# macOS
brew install gcc llvm
```

## Building

### Using Parth (Recommended)

```bash
parth build file.ajb        # Compile to native binary
parth run file.ajb           # Run directly with interpreter
parth test                   # Run test suite
parth bootstrap              # Verify self-hosting
```

### Using Make

```bash
cd ajeebc
make native                  # Build compiler
make test                    # Run tests
make bootstrap               # Verify self-hosting
```

### Manual Compilation

```bash
ajeebc file.ajb --skip-run   # Generate LLVM IR
llc file.ll -o file.s        # Compile to assembly
gcc file.s runtime/ajeeb_runtime.c -o file  # Link
./file                       # Run
```

## Using Parth

Parth is the Ajeeb package manager (like Cargo for Rust).

```bash
parth init my-project        # Create new project
parth build                  # Build project
parth run                    # Run project
parth test                   # Run tests
parth bootstrap              # Verify self-hosting
parth clean                  # Clean build artifacts
```

## Examples

### Hello World

```ajeeb
function main(): int {
    println("Hello, World!");
    return 0;
}
```

### Functions

```ajeeb
function add(a: int, b: int): int {
    return a + b;
}

function main(): int {
    set x: int = add(10, 32);
    println(itoa(x) + "\n");
    return 0;
}
```

### Classes

```ajeeb
class Point {
    set x: int;
    set y: int;
    
    function new(x: int, y: int): Point {
        set p: Point = new Point();
        p.x = x;
        p.y = y;
        return p;
    }
    
    function distance(self, other: Point): int {
        set dx: int = self.x - other.x;
        set dy: int = self.y - other.y;
        return dx * dx + dy * dy;
    }
}
```

### Control Flow

```ajeeb
function main(): int {
    set x: int = 10;
    
    if (x > 5) {
        println("x is greater than 5\n");
    } else {
        println("x is 5 or less\n");
    }
    
    set i: int = 0;
    while (i < 5) {
        println(itoa(i) + "\n");
        i = i + 1;
    }
    
    return 0;
}
```

## Architecture

```
.ajb → Lexer → Parser → AST → HIR → THIR → MIR → LLVM IR → Binary
                                        ↓
                                    C Code (fallback)
```

### Compilation Pipeline

1. **Lexer** — Tokenizes source code
2. **Parser** — Builds Abstract Syntax Tree (AST)
3. **HIR** — High-level Intermediate Representation
4. **THIR** — Typed HIR with type checking
5. **MIR** — Mid-level IR with optimizations
6. **Code Generation** — LLVM IR or C code

### Backends

| Backend | Description | Status |
|---------|-------------|--------|
| LLVM | Primary backend, production-ready | Default |
| C | Fallback backend, for platforms without LLVM | Supported |
| Interpreter | Direct AST execution (AJB interpreter) | Supported |

## Self-Hosting

Ajeeb is fully self-hosting. The bootstrap chain:

```
Gen0 (Rust, 15MB) → Gen1 (Ajeeb, 142KB) → Gen2 (Ajeeb, 142KB)
```

- Gen1 and Gen2 produce **identical** C output (2,705 lines)
- Binary size reduced by **99%** (15MB → 142KB)
- No Rust required for normal development

## Project Structure

```
ajeeb_compiler/
├── ajeebc/           # Compiler source (Rust + Ajeeb)
├── compiler/         # Ajeeb compiler source
├── runtime/          # C runtime library
├── tests/            # Test suite
├── docs/             # Documentation
├── scripts/          # Install scripts
├── parth/            # Package manager (Ajeeb)
├── piri/             # MIR interpreter
```

## Error Messages (Hinglish)

```
Arre bhai! ';' lagana bhool gaye!
Ye variable pehle se exist karta hai!
Type galat hai — int chahiye tha!
```

## Performance

| Metric | Value |
|--------|-------|
| Compile time (single file) | ~0.9s |
| Full compiler build | ~2.3s |
| Bootstrap chain | ~48s |
| Binary size (compiler) | 142 KB |
| Memory usage | ~5 MB |

## Status

**v1.0.0** — First stable release. Self-hosting verified, all tests passing.

## License

MIT

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Issues and PRs welcome!

## Links

- [Documentation](docs/)
- [Language Specification](ajeeb-lang/)
- [Changelog](CHANGELOG.md)
- [Release Notes](RELEASE_NOTES_v1.0.0.md)
