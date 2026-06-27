# Ajeeb Programming Language

Ajeeb ek self-hosting programming language hai! File extension: `.ajb`

## Tools

| Tool   | Description                | Location               |
|--------|----------------------------|------------------------|
| ajeebc | Ajeeb Compiler (Rust → native) | `ajeebc/`          |
| parth  | Package Manager            | `parth/`               |
| piri   | MIR Interpreter            | `piri/`                |

## Quick Start

```bash
parth init my-project
cd my-project
parth run    # interpret with piri
parth build  # compile with ajeebc
```
