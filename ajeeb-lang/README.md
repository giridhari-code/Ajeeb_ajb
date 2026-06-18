# Ajeeb Programming Language

Ajeeb ek self-hosting programming language hai! File extension: `.ajb`

## Tools

| Tool   | Description                | Location               |
|--------|----------------------------|------------------------|
| ajeebc | Ajeeb Compiler (Rust → native) | `ajeebc/`          |
| parth  | Package Manager            | `parth/`               |
| parthi | MIR Interpreter            | `parthi/`              |

## Quick Start

```bash
parth init my-project
cd my-project
parth run    # interpret with parthi
parth build  # compile with ajeebc
```
