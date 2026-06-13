# Ajeeb Tooling Architecture

## Overview

Three tooling phases to make Ajeeb developer-ready:

1. **`ajeeb fmt`** — AST-based code formatter
2. **Language Server (LSP)** — IDE intelligence
3. **VSCode Extension** — Editor integration

Each phase builds on the previous. All tools share the compiler's existing AST and parser.

---

## Phase 1: `ajeeb fmt`

### Architecture

```
┌─────────────────────────────────────────────────┐
│                  ajeeb-fmt                       │
│                                                   │
│  ┌──────────┐   ┌──────────────┐   ┌──────────┐  │
│  │  Parser   │──▶│  FormatVisitor│──▶│  Output  │  │
│  │(compiler) │   │  (stmts/exprs)│   │  Writer  │  │
│  └──────────┘   └──────────────┘   └──────────┘  │
│                        │                          │
│                        ▼                          │
│                 ┌──────────────┐                   │
│                 │   Config    │                    │
│                 │ indent/wrap │                    │
│                 └──────────────┘                   │
└─────────────────────────────────────────────────┘
```

### Design

**Core approach**: Parse source → AST → walk AST → emit formatted source.

**Comment handling** (deferred): Comments are discarded by the current lexer. For MVP, formatting without comment preservation. Comments will be added in a follow-up via a pre-lex pass that extracts comment positions and attaches them to nearest AST nodes.

### Crate Structure

```
crates/ajeeb-fmt/
├── Cargo.toml          # depends on ajeeb-compiler
└── src/
    ├── lib.rs           # public API: format_source(), format_file()
    ├── formatter.rs     # AST walker + output builder
    ├── config.rs        # FormatConfig struct
    ├── ir.rs            # Intermediate representation (annotated tokens)
    └── main.rs          # CLI binary
```

### Formatting Rules

| Construct | Rule |
|-----------|------|
| Braces | Same line (K&R): `function foo() {` |
| Indentation | Configurable (default: 4 spaces) |
| Line width | Configurable (default: 100 chars) |
| Operators | Space around binary: `a + b`, `a == b` |
| Function params | Space after comma: `(a: Int, b: String)` |
| If/while/for | Space after keyword: `if (cond) {` |
| Type annotations | `name: Type` (space after colon) |
| Array literals | `[1, 2, 3]` (space after commas) |
| Struct literals | `Point { x: 1, y: 2 }` |
| Match arms | `pattern => expr,` (comma between arms) |
| Strings | Preserve original content, escape as needed |
| Imports | Sorted alphabetically by path |
| Blank lines | Max 1 consecutive blank line |
| Trailing whitespace | Removed |

### CLI Interface

```
ajeeb fmt [OPTIONS] [FILES...]

Options:
  --check       Check formatting without modifying files (exit code 1 if unformatted)
  --write       Write formatted output in-place (default)
  --stdout      Write formatted output to stdout
  --indent N    Indentation width (default: 4)
  --width N     Max line width (default: 100)
  --tab         Use tabs for indentation
```

### Implementation Plan

1. Create crate skeleton + AST export review
2. Implement `FormatConfig` and basic output primitives (indent, newlines)
3. Implement statement formatters (FnDef, Class, Struct, Enum, Import, Let, If, While, For, Return)
4. Implement expression formatters (all 22 Expr variants)
5. Implement import sorting
6. Implement `--check` mode
7. Write formatting test suite
8. Integrate into build system

---

## Phase 2: Language Server (LSP)

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                     ajeeb-lsp                            │
│                                                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Document    │  │  Analyzer    │  │  LSP Server  │   │
│  │  Manager     │  │  (semantic)  │  │  (protocol)  │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                  │           │
│         ▼                 ▼                  ▼           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Parse Cache │  │  Symbol      │  │  JSON-RPC    │   │
│  │  (incremental)│  │  Table      │  │  Transport   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Capabilities

| Feature | Implementation |
|---------|---------------|
| Diagnostics | Run semantic analysis on file change, report errors |
| Go-to-definition | Symbol table lookup for identifiers |
| Hover | Type info + doc comments for symbols |
| Autocomplete | Scope-aware identifier completion |
| Document symbols | Outline of functions, classes, structs, enums |
| Document highlights | All references to symbol at cursor |
| Formatting | Delegates to `ajeeb fmt` |

### Implementation Plan

1. Create LSP crate with lsp-server dependency
2. Implement document manager with parse cache
3. Implement diagnostics endpoint
4. Implement go-to-definition
5. Implement hover with type information
6. Implement completion provider
7. Implement document symbols
8. Integration test suite

---

## Phase 3: VSCode Extension

### Architecture

```
editors/vscode-ajeeb/
├── package.json           # Extension manifest
├── language-configuration.json  # Comment toggles, brackets
├── syntaxes/
│   └── ajeeb.tmLanguage.json    # TextMate grammar
├── snippets/
│   └── ajeeb.json               # Code snippets
├── src/
│   ├── extension.ts       # Activation entry point
│   ├── lspClient.ts       # LSP client setup
│   ├── formatter.ts       # Formatter integration
│   ├── debugger.ts        # Debug adapter protocol
│   └── statusBar.ts       # Status bar items
└── test/
    └── suite/
        └── extension.test.ts
```

### Features

| Feature | How |
|---------|-----|
| Syntax highlighting | TextMate grammar for .ajb files |
| Code formatting | On-save via `ajeeb fmt` |
| Intellisense | LSP integration |
| Snippets | Common patterns (fn, class, struct, enum, if, for) |
| Debugging | DAP adapter hooks |
| Problem reporting | LSP diagnostics in Problems panel |

### Implementation Plan

1. Create extension scaffold with yo code
2. Write TextMate grammar for syntax highlighting
3. Implement LSP client
4. Implement formatter integration (on-save)
5. Write code snippets
6. Add debug adapter configuration
7. Package and test

---

## Dependency Graph

```
ajeeb-compiler (lib)
    ↑
ajeeb-fmt ──────────┐
    ↑                │
ajeeb-lsp ───────────┤
    ↑                │
vscode-ajeeb ────────┘
```

- `ajeeb-fmt` depends on `ajeeb-compiler` (parser + AST)
- `ajeeb-lsp` depends on `ajeeb-compiler` (parser + semantic analysis) + `lsp-server` crate
- `vscode-ajeeb` is a TypeScript extension that launches `ajeeb-lsp` and calls `ajeeb fmt`
