# Ajeeb

Self-hosting compiler for the **Ajeeb** programming language.

Pipeline: **Lex → Parse → HIR → MIR → C codegen → native binary**

## Layout

```
.
├── ajeebc/                 # Compiler package
│   ├── compiler/src/       # Ajeeb sources (self-hosted compiler)
│   │   ├── compiler.ajb    # CLI + pipeline orchestrator
│   │   ├── parser.ajb      # Lexer + parser
│   │   ├── hir.ajb         # High-level IR
│   │   ├── mir.ajb         # Mid-level IR
│   │   ├── lower.ajb       # HIR → MIR
│   │   ├── codegen_c.ajb   # MIR → C
│   │   └── cache.ajb       # Compile cache
│   ├── runtime/            # C runtime (buffers, I/O, syscalls)
│   ├── crates/             # Optional Rust Gen0 bootstrap
│   ├── tests/              # Language & compiler tests
│   ├── bootstrap/          # Bootstrap helpers / prebuilts
│   └── Makefile            # Build, test, bootstrap targets
├── Makefile                # Delegates to ajeebc/
├── LICENSE
└── README.md
```

## Requirements

- A C compiler (`cc`) for the final link step of generated code
- Optional: `rustc` only if rebuilding the Gen0 bootstrap from source

## Build

```bash
cd ajeebc
make native          # Gen0 → native compiler binary
make test            # Run test suite
make bootstrap       # Gen0 → Gen1 → Gen2 self-host check
```

From repo root:

```bash
make native
make test
```

## Usage

```bash
./ajeebc/build/compiler path/to/program.ajb
./ajeebc/build/compiler path/to/program.ajb --skip-run   # compile only
./ajeebc/build/compiler path/to/program.ajb -o out       # output path
```

## Bootstrap stages

| Stage | Role |
|-------|------|
| Gen0  | Bootstrap compiler (Rust or prebuilt) |
| Gen1  | Compiler built from Ajeeb sources by Gen0 |
| Gen2  | Compiler rebuilt by Gen1 (self-host target) |

## Language (core)

- Functions, `set` / assign, `if` / `else`, `while`, `break` / `continue`
- `int`, `string`, `bool`, `void`
- Calls, recursion, imports (`import module;`)
- C backend via MIR


## Parth (package manager)

**Parth** is the Ajeeb package and project manager.

```bash
cd parth && ./build.sh          # build parth binary
./build/parth new my-app        # create project
cd my-app
parth init                      # write parth.das + src/main.ajb
parth build                     # compile with ajeebc
parth run                       # build + run
parth install ajeebc            # install toolchain helper
parth generate-lockfile         # lock dependencies
```

Project manifest: `parth.das`

```toml
[package]
name = "my-app"
version = "0.1.0"

[dependencies]
# name = "version"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
```

## Status

Active development toward full self-host. Core arithmetic, control flow, and multi-arg calls work for many programs; some edge cases (deep expression chains, nested control-flow patterns) are still being hardened.

## License

MIT — see [LICENSE](LICENSE)
