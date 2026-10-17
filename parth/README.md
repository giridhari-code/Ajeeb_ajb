# Parth — Ajeeb package manager

Project + dependency manager for the Ajeeb language.

## Build Parth

```bash
cd parth
./build.sh
# binary: ./build/parth
```

## Commands

| Command | Description |
|---------|-------------|
| `parth init [name]` | Init project in current directory (`parth.das` + `src/main.ajb`) |
| `parth new <name>` | Create a new project folder |
| `parth build` | Compile with `ajeebc` |
| `parth run` | Build and run |
| `parth test` | Run tests under `tests/` |
| `parth install <tool>` | Install helper tool (`ajeebc` / `parthi`) |
| `parth generate-lockfile` | Write dependency lockfile |

## Manifest (`parth.das`)

```toml
[package]
name = "my-app"
version = "0.1.0"
author = ""

[dependencies]
# pkg = "0.1.0"

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
```

## Layout

```
parth/
├── src/
│   ├── main.ajb       # CLI
│   ├── parser.ajb     # parth.das parser
│   ├── resolver.ajb   # deps + lockfile
│   ├── builder.ajb    # build orchestration
│   └── runner.ajb     # run / test
├── tests/
├── build.sh
└── README.md
```
