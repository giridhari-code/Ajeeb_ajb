# Ajeeb 🎉

A self-hosting programming language written in itself — with Hinglish error messages!

## What is Ajeeb?

Ajeeb (अजीब) is a TypeScript-inspired programming language that compiles itself. File extension: `.ajb`

Like Rust bootstrapped from OCaml, Ajeeb bootstrapped from Rust — and now compiles its own compiler.

## Ecosystem

| Tool | Description |
|------|-------------|
| `ajeebc` | The Ajeeb compiler (LLVM backend) |
| `parth` | Package manager (like Cargo) |
| `parthi` | MIR interpreter (no LLVM needed) |
| `packages/ajeeb-std` | Standard library |

## Requirements

| Tool | Kyun |
|------|------|
| `gcc` or `clang` | Native binary link karne ke liye |
| `llc` (LLVM) | Assembly generate karne ke liye |

> 💡 Agar nahi hain to install karo:
> **Ubuntu/Debian:** `sudo apt install gcc llvm`
> **Fedora:** `sudo dnf install gcc llvm`
> **Arch:** `sudo pacman -S gcc llvm`
> **macOS:** `brew install gcc llvm`

## Quick Start (End Users — no Rust, no cargo!)

```bash
curl -sSf https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/install.sh | bash

# Naya terminal kholo, phir:
parth init hello-ajeeb
cd hello-ajeeb
parth run
```

Ya fir manually:

```bash
export PATH="$HOME/.ajeeb/bin:$PATH"

# Compile karo
ajeebc file.ajb     # .ll file banega
llc file.ll -o file.s
gcc file.s -o file  # ajeeb_runtime.c bhi link karna

# Ya parthi se direct run
parthi file.ajb
```

## Language Features

- TypeScript-style syntax
- Static typing (int, string, bool, float, void)
- Classes, structs, enums, traits
- Generics
- Pattern matching (match)
- Generics, arrays, closures (in progress)
- **Hinglish error messages** 😄

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

## Architecture

```
Stage 0: Rust Bootstrap Compiler
Stage 1: Ajeeb Lexer + Parser (self-hosted)
Stage 2: Ajeeb HIR + THIR (self-hosted)
Stage 3: Ajeeb MIR + Full Pipeline (self-hosted)

.ajb → Lexer → Parser → AST → HIR → THIR → MIR → LLVM IR → Binary
```

## Project Structure

```
ajeeb-lang/    Language spec + docs
ajeebc/        Compiler source
parth/         Package manager
parthi/        MIR interpreter
packages/      Standard library + packages
tests/         Test suite
```

## Error Messages (Hinglish)

```
Arre bhai! ';' lagana bhool gaye!
Ye variable pehle se exist karta hai!
Type galat hai — int chahiye tha!
```

## Status

🚧 Active development — self-hosting bootstrap complete, expanding language features.

## License

MIT (or your choice)

## Contributing

Issues and PRs welcome!

## Building from Source (kewal contributors ke liye)

Ajeeb compiler Rust mein likha hai (bootstrap phase). End users ko Rust ki zaroorat nahi — pre-built binaries milte hain. Lekin agar aap compiler mein modification kar rahe hain:

```bash
git clone https://github.com/giridhari-code/Ajeeb_ajb
cd Ajeeb_ajb

# Rust compiler build karo (sirf development ke liye)
cd ajeebc
make rust

# Self-hosting check
cd ../ajeebc
bash tests/bootstrap_check.sh
```
