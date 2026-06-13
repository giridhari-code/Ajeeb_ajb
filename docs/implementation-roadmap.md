# Ajeeb Tooling Implementation Roadmap

## Phase 1: `ajeeb fmt`

### Step 1: Create crate skeleton
- Cargo.toml with ajeeb-compiler dependency
- lib.rs, main.rs, formatter.rs, config.rs, comment.rs

### Step 2: Comment extraction
- Pre-scan source for // and /* */ comments
- Record position + text
- Attach to nearest AST node as leading/trailing

### Step 3: Core formatter
- Walk Stmt nodes: FnDef, Class, Struct, Enum, Import, Let, Const, If, While, For, Return, Break, Continue, Expr
- Walk Expr nodes: all 22 variants
- Handle indentation, newlines, spacing

### Step 4: Import sorting
- Collect all imports
- Sort alphabetically by path
- Emit in sorted order

### Step 5: CLI + tests
- --check, --write, --stdout flags
- Test suite covering all constructs
- Verify idempotency (format twice → same output)

## Phase 2: Language Server

### Step 1: Create crate skeleton
- Cargo.toml with lsp-server + ajeeb-compiler
- Main LSP event loop

### Step 2: Document manager
- Track open documents
- Parse on change
- Cache AST + symbol table

### Step 3: Diagnostics
- Run semantic analysis
- Publish diagnostics via LSP

### Step 4: Navigation + completion
- Go-to-definition
- Hover info
- Autocomplete
- Document symbols

## Phase 3: VSCode Extension

### Step 1: Extension scaffold
- package.json
- activation events

### Step 2: Syntax highlighting
- TextMate grammar
- Language configuration

### Step 3: LSP client
- Launch ajeeb-lsp
- Forward diagnostics

### Step 4: Formatter + snippets
- On-save formatting
- Code snippets
