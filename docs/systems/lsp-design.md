# Language Server Protocol (LSP) — Design Document

## 1. Audit Summary

### Current State
- **No LSP**: no language server protocol implementation
- **No IDE support**: any editor support is manual text editing
- **Compiler architectured as CLI binary** (`main.rs`): `file_path` → lex → parse → module_load → semantic → exec; the pipeline is one-shot, not incremental
- **Semantic analysis is fresh per run**: `SemanticAnalyzer` is created, runs `analyze()`, produces errors — no caching, no persistence
- **Formatter exists** as separate binary: `ajeeb-fmt`; no `--stdin` mode in the compiler
- **No symbol table persistence**: all scopes are ephemeral within a single `analyze()` call

### Identified Gaps
| Gap | Severity | Description |
|-----|----------|-------------|
| No LSP | BLOCKER | No IDE support, no autocomplete, no diagnostics in editor |
| Compiler is one-shot only | HIGH | Can't incrementally re-parse on file change |
| No symbol table query API | HIGH | No way to look up defs, types, completions after analysis |
| No file watcher / document sync | HIGH | LSP requires open-document tracking |

### Breaking Changes
1. **Compiler must expose a library API** — currently only has a CLI binary; must export `pub fn compile(source: &str) -> CompileResult` usable from LSP
2. **SemanticAnalyzer state must be persistable** — symbol table, scope chain, function signatures must be queryable after analysis
3. **No new syntax** — LSP is purely a tooling addition, zero language changes

---

## 2. Design: Crate Structure

### 2.1 New Crate: `ajeeb-lsp`

```
crates/ajeeb-lsp/
├── Cargo.toml
└── src/
    ├── main.rs           # LSP binary entry point
    ├── server.rs         # LSP event loop + protocol handler
    ├── documents.rs      # Document manager (open files, parse cache)
    ├── diagnostics.rs    # Semantic error → LSP Diagnostic conversion
    ├── completion.rs     # Autocomplete provider
    ├── goto_def.rs       # Go-to-definition
    ├── hover.rs          # Hover information
    └── symbols.rs        # Document symbols
```

### 2.2 Dependencies

```toml
[package]
name = "ajeeb-lsp"
version = "0.1.0"

[dependencies]
ajeeb-compiler = { path = "../ajeeb-compiler" }
lsp-server = "0.7"        # JSON-RPC based LSP framework
lsp-types = "0.95"        # LSP type definitions
serde_json = "1"
```

---

## 3. Design: Document Manager

### 3.1 `DocumentManager`

```rust
struct Document {
    uri: Url,
    version: i32,
    source: String,
    ast: Vec<Stmt>,
    tokens: Vec<(Token, usize, usize)>,
    semantic: SemanticSnapshot,
    errors: Vec<CompileError>,
    dirty: bool,
}

struct DocumentManager {
    docs: HashMap<Url, Document>,
    config: LspConfig,
}

impl DocumentManager {
    fn open(&mut self, uri: Url, source: String)      // initial parse
    fn change(&mut self, uri: Url, version: i32, changes: Vec<TextDocumentContentChangeEvent>)
    fn close(&mut self, uri: Url)
    fn get(&self, uri: &Url) -> Option<&Document>
    fn get_mut(&mut self, uri: &Url) -> Option<&mut Document>
}
```

### 3.2 Incremental Re-parse

On `textDocument/didChange`:
1. Apply text changes to source
2. Re-lex (full re-lex — cheap, ~2ms for 10k lines)
3. Re-parse (full re-parse — ~10ms for 10k lines)
4. Re-run semantic analysis (full analysis — ~50ms for 10k lines)
5. Publish diagnostics

**Not incremental at first**: Ajeeb files are small. Full re-parse is acceptable for <10k lines. Incremental parsing is a Phase 2 optimization.

### 3.3 SemanticSnapshot

```rust
struct SemanticSnapshot {
    errors: Vec<CompileError>,
    symbols: SymbolTable,
    functions: HashMap<String, (Vec<(String, TypeAnnot)>, TypeAnnot)>,
    struct_defs: HashMap<String, Vec<(String, TypeAnnot)>>,
    enum_defs: HashMap<String, Vec<EnumVariantDef>>,
    traits: HashMap<String, Vec<TraitMethod>>,
    scopes: Vec<HashMap<String, TypeAnnot>>,
}

struct SymbolTable {
    symbols: Vec<Symbol>,
    // For quick lookup by position:
    position_index: IntervalTree<SymbolId>,  // position → symbol at that position
}

struct Symbol {
    id: SymbolId,
    name: String,
    kind: SymbolKind,        // Function, Variable, Class, Struct, Enum, Trait, Field, Param
    type_ann: TypeAnnot,
    range: Range,            // line/col in source
    full_range: Range,       // entire declaration
    detail: String,          // extra info for hover
    doc_comment: Option<String>,
}
```

---

## 4. Design: LSP Capabilities

### 4.1 Diagnostics (`textDocument/publishDiagnostics`)

```rust
fn build_diagnostics(errors: &[CompileError]) -> Vec<Diagnostic> {
    errors.iter().map(|e| Diagnostic {
        range: Range {
            start: Position { line: e.line - 1, character: e.col - 1 },
            end: Position { line: e.line - 1, character: e.col + 10 }, // span approximation
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: e.message.clone(),
        ..Default::default()
    }).collect()
}
```

Triggered on every `didChange` and `didOpen`.

### 4.2 Go-to-Definition (`textDocument/definition`)

```rust
fn goto_definition(doc: &Document, position: Position) -> Option<Location> {
    let ident = find_identifier_at_position(&doc.ast, position)?;
    let symbol = doc.semantic.symbols.find_definition(&ident)?;
    Some(Location {
        uri: doc.uri.clone(),
        range: symbol.range,
    })
}
```

**Identifier resolution**: Walk AST at position → find `Expr::Ident` or pattern binding → look up its definition in `SymbolTable`.

### 4.3 Hover (`textDocument/hover`)

```rust
fn hover(doc: &Document, position: Position) -> Option<Hover> {
    let symbol = find_symbol_at_position(&doc, position)?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```ajeeb\n{}: {}\n```\n{}", 
                symbol.name, format_type(&symbol.type_ann), symbol.detail),
        }),
        range: Some(symbol.range),
    })
}
```

### 4.4 Autocomplete (`textDocument/completion`)

```rust
fn completions(doc: &Document, position: Position) -> Vec<CompletionItem> {
    let prefix = extract_prefix(&doc.source, position);
    let scope = find_scope_at_position(&doc.semantic, position);
    
    scope.iter()
        .filter(|(name, _)| name.starts_with(&prefix))
        .map(|(name, ty)| CompletionItem {
            label: name.clone(),
            kind: Some(symbol_kind_to_completion_kind(&ty)),
            detail: Some(format_type(ty)),
            ..Default::default()
        })
        .chain(builtin_completions(&prefix))
        .collect()
}
```

### 4.5 Document Symbols (`textDocument/documentSymbol`)

```rust
fn document_symbols(doc: &Document) -> Vec<DocumentSymbol> {
    doc.semantic.symbols.iter().map(|sym| DocumentSymbol {
        name: sym.name.clone(),
        kind: symbol_kind_to_symbol_kind(&sym.kind),
        range: sym.full_range,
        selection_range: sym.range,
        detail: Some(sym.detail.clone()),
        children: None,  // Phase 2: nested symbols
    }).collect()
}
```

---

## 5. Implementation Plan

### Phase 1A: Library API for Compiler (~1 week)

1. Add `pub fn compile_source(source: &str) -> CompileResult` to `ajeeb-compiler/src/lib.rs`
2. Add `pub struct CompileResult { ast, tokens, errors, symbols, semantic }` 
3. Add `pub fn query_symbol_at_position(ast, position) -> Option<Symbol>` 
4. Add `pub fn get_scope_at_position(semantic, position) -> Vec<(String, TypeAnnot)>`
5. Ensure all AST nodes carry line/col

### Phase 1B: LSP Scaffold (~1 week)

1. Create `crates/ajeeb-lsp/` with `Cargo.toml`
2. Add `lsp-server` and `lsp-types` dependencies
3. Implement `main.rs`: initialize LSP server, enter event loop
4. Implement `server.rs`: handle `initialize`, `initialized`, `shutdown`
5. Test with `neovim` or VSCode — verify server starts and responds

### Phase 1C: Diagnostics (~1 week)

1. Implement `documents.rs` — `DocumentManager` with open/change/close
2. Wire `textDocument/didOpen` → parse → publish diagnostics
3. Wire `textDocument/didChange` → re-parse → publish diagnostics
4. Wire `textDocument/didClose` → cleanup
5. Test: open a file with errors → see diagnostics in editor

### Phase 1D: Go-to-Definition (~1 week)

1. Implement `goto_def.rs` — find identifier at position → locate definition
2. Build `SymbolTable` during semantic analysis
3. Wire `textDocument/definition` → return location
4. Test: Ctrl+click on function call → jump to definition

### Phase 1E: Hover (~1 week)

1. Implement `hover.rs` — look up symbol → format type info
2. Wire `textDocument/hover` → return Hover with Markdown
3. Test: hover over variable → see type annotation

### Phase 1F: Autocomplete (~1-2 weeks)

1. Implement `completion.rs` — scope-aware identifier completion
2. Add builtin function completions
3. Add keyword completions
4. Wire `textDocument/completion` → return CompletionList
5. Test: type `pri` → see `print`, `println` suggestions

---

## 6. Complexity Estimate

| Step | Lines Changed | Files | Effort |
|------|--------------|-------|--------|
| Library API for compiler | ~100 | `lib.rs`, `semantic.rs` | 1 week |
| LSP scaffold | ~150 | `main.rs`, `server.rs`, `Cargo.toml` | 1 week |
| Document manager | ~200 | `documents.rs` | 1 week |
| Diagnostics | ~100 | `diagnostics.rs`, `server.rs` | 1 week |
| Go-to-definition | ~200 | `goto_def.rs`, `semantic.rs` (symbols) | 1 week |
| Hover | ~100 | `hover.rs` | 1 week |
| Autocomplete | ~200 | `completion.rs` | 1-2 weeks |
| Document symbols | ~100 | `symbols.rs` | 3 days |
| **Total** | **~1150** | **~12 files** | **7-9 weeks** |

---

## 7. Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Full re-parse too slow for large files | Benchmark; optimize parser if >50ms; add incremental parse later |
| LSP JSON-RPC complexity | Use `lsp-server` crate which handles transport |
| No doc comments in lexer | Lexer discards comments; Phase 2: add comment capture |
| SymbolTable linear search too slow on hover | Use interval tree for position-based lookup; <1000 symbols, linear is fine |
| Windows pipe transport | `lsp-server` handles stdio; cross-platform by default |

---

## 8. Testing

```rust
// Integration test: start LSP server, send requests, verify responses
#[test]
fn test_diagnostics() {
    let (mut server, mut client) = lsp_server::Connection::memory();
    // Send didOpen with erroneous source
    // Receive diagnostics notification
    // Assert diagnostic messages
}

#[test]
fn test_goto_definition() {
    // Parse file with function def + call
    // Query definition at call site
    // Assert returns function def location
}
```
