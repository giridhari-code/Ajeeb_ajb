use ajeeb_compiler::ast::*;
use ajeeb_compiler::lexer::Lexer;
use ajeeb_compiler::parser::Parser;
use ajeeb_compiler::semantic::SemanticAnalyzer;
use ajeeb_compiler::token::Token;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

// ── Types ───────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LspMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<LspError>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct LspError {
    code: i64,
    message: String,
}

#[derive(Debug, Clone)]
struct SymbolInfo {
    name: String,
    kind: String,
    line: usize,
    col: usize,
}

struct Document {
    uri: String,
    text: String,
    version: i64,
    stmts: Vec<Stmt>,
    symbols: Vec<SymbolInfo>,
    analyzer: Option<SemanticAnalyzer>,
}

struct LspServer {
    documents: HashMap<String, Document>,
    capabilities: serde_json::Value,
}

impl LspServer {
    fn new() -> Self {
        LspServer {
            documents: HashMap::new(),
            capabilities: serde_json::json!({
                "textDocumentSync": {
                    "openClose": true,
                    "change": 1,
                    "save": { "includeText": false }
                },
                "hoverProvider": true,
                "definitionProvider": true,
                "referencesProvider": true,
                "renameProvider": { "prepareProvider": true },
                "completionProvider": {
                    "resolveProvider": false,
                    "triggerCharacters": [".", ":", ">"]
                },
                "documentSymbolProvider": true,
                "codeActionProvider": { "codeActionKinds": ["quickfix", "source"] },
                "workspace": { "workspaceFolders": { "supported": true } }
            }),
        }
    }

    fn analyze_document(&mut self, uri: &str) {
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return,
        };
        let contents = doc.text.clone();

        let mut lexer = Lexer::new(&contents);
        let mut tokens = Vec::new();
        let mut token_lines = Vec::new();
        let mut token_cols = Vec::new();
        loop {
            match lexer.next_token_spanned() {
                Ok((Token::Eof, _, _)) => break,
                Ok((tok, line, col)) => { tokens.push(tok); token_lines.push(line); token_cols.push(col); }
                Err(_) => break,
            }
        }

        let mut parser = Parser::with_positions(tokens, token_lines, token_cols);
        let stmts = match parser.parse_program() {
            Ok(s) => s,
            Err(e) => {
                let diag = make_diagnostic(&e.to_string(), e.line, e.col, 1, "error");
                self.notify("textDocument/publishDiagnostics",
                    serde_json::json!({"uri": uri, "diagnostics": [diag]}));
                return;
            }
        };

        let mut symbols = Vec::new();
        for stmt in &stmts {
            collect_symbols(stmt, &mut symbols);
        }

        let mut analyzer = SemanticAnalyzer::new();
        analyzer.analyze(&stmts);

        let mut diagnostics = Vec::new();
        for err in &analyzer.errors {
            diagnostics.push(make_diagnostic(&err.message, err.line, err.col, 1, "error"));
        }
        self.notify("textDocument/publishDiagnostics",
            serde_json::json!({"uri": uri, "diagnostics": diagnostics}));

        if let Some(d) = self.documents.get_mut(uri) {
            d.stmts = stmts;
            d.symbols = symbols;
            d.analyzer = Some(analyzer);
        }
    }

    fn find_definition(&self, uri: &str, line: u64, col: u64) -> Option<serde_json::Value> {
        let doc = self.documents.get(uri)?;
        let (name, _kind) = self.find_identifier_at(&doc.stmts, line as usize, col as usize)?;
        for (doc_uri, d) in &self.documents {
            for sym in &d.symbols {
                if sym.name == name {
                    return Some(serde_json::json!({
                        "uri": doc_uri,
                        "range": {
                            "start": { "line": sym.line - 1, "character": sym.col - 1 },
                            "end": { "line": (sym.line as u64) - 1, "character": (sym.col as u64) + (name.len() as u64) - 1 }
                        }
                    }));
                }
            }
        }
        None
    }

    fn find_references(&self, uri: &str, line: u64, col: u64) -> Vec<serde_json::Value> {
        let doc = match self.documents.get(uri) { Some(d) => d, None => return vec![] };
        let (name, _kind) = match self.find_identifier_at(&doc.stmts, line as usize, col as usize) { Some(n) => n, None => return vec![] };

        let mut refs = Vec::new();
        for (doc_uri, d) in &self.documents {
            find_id_refs(&d.stmts, &name, doc_uri, &mut refs);
            for sym in &d.symbols {
                if sym.name == name {
                    refs.push(make_loc(doc_uri, sym.line, sym.col, name.len()));
                }
            }
        }
        refs
    }

    fn find_identifier_at(&self, stmts: &[Stmt], line: usize, col: usize) -> Option<(String, String)> {
        for stmt in stmts {
            if let Some(r) = find_name_in_stmt(stmt, line, col) { return Some(r); }
        }
        None
    }

    fn find_completions(&self, uri: &str, _line: u64, _col: u64) -> Vec<serde_json::Value> {
        let doc = match self.documents.get(uri) { Some(d) => d, None => return vec![] };
        let mut items = Vec::new();
        for kw in &["let","const","if","else","while","for","function",
                     "return","class","struct","enum","match","import",
                     "true","false","int","float","string","bool","void"] {
            items.push(serde_json::json!({"label": kw, "kind": 14, "detail": "keyword"}));
        }
        for sym in &doc.symbols {
            let k = match sym.kind.as_str() { "function" => 3, "class" => 5, "struct" => 7, "enum" => 10, _ => 6 };
            items.push(serde_json::json!({"label": sym.name, "kind": k, "detail": sym.kind}));
        }
        items
    }

    fn rename_symbol(&mut self, uri: &str, line: u64, col: u64, new_name: &str) -> Option<serde_json::Value> {
        let doc = self.documents.get(uri)?;
        let (name, _kind) = self.find_identifier_at(&doc.stmts, line as usize, col as usize)?;
        let mut changes = serde_json::Map::new();
        for (doc_uri, d) in &self.documents {
            let mut edits = Vec::new();
            let mut refs = Vec::new();
            find_id_refs(&d.stmts, &name, doc_uri, &mut refs);
            for sym in &d.symbols {
                if sym.name == name {
                    refs.push(make_loc(doc_uri, sym.line, sym.col, name.len()));
                }
            }
            for r in &refs {
                if let Some(range) = r.get("range") {
                    edits.push(serde_json::json!({"range": range, "newText": new_name}));
                }
            }
            if !edits.is_empty() { changes.insert(doc_uri.clone(), serde_json::json!(edits)); }
        }
        if changes.is_empty() { return None; }
        Some(serde_json::json!({"changes": changes}))
    }

    fn resolve_hover(&self, uri: &str, line: u64, col: u64) -> Option<serde_json::Value> {
        let doc = self.documents.get(uri)?;
        let (name, kind) = self.find_identifier_at(&doc.stmts, line as usize, col as usize)?;
        Some(serde_json::json!({
            "contents": { "kind": "markdown", "value": format!("**{}** — {}", name, kind) },
            "range": {
                "start": { "line": line, "character": col },
                "end": { "line": line, "character": col + name.len() as u64 }
            }
        }))
    }

    fn document_symbols(&self, uri: &str) -> Vec<serde_json::Value> {
        let doc = match self.documents.get(uri) { Some(d) => d, None => return vec![] };
        doc.symbols.iter().map(|sym| {
            let k = match sym.kind.as_str() { "function" => 3, "class" => 5, "struct" => 7, "enum" => 10, _ => 6 };
            serde_json::json!({
                "name": sym.name, "kind": k,
                "location": {
                    "uri": uri,
                    "range": {
                        "start": { "line": sym.line - 1, "character": sym.col - 1 },
                        "end": { "line": sym.line - 1, "character": sym.col + sym.name.len() - 1 }
                    }
                }
            })
        }).collect()
    }

    // ── Transport ───────────────────────────────────────────────

    fn send(&self, msg: &LspMessage) {
        let json = serde_json::to_string(msg).unwrap_or_default();
        let header = format!("Content-Length: {}\r\n\r\n", json.len());
        let mut out = io::stdout().lock();
        let _ = out.write_all(header.as_bytes());
        let _ = out.write_all(json.as_bytes());
        let _ = out.flush();
    }

    fn notify(&self, method: &str, params: serde_json::Value) {
        self.send(&LspMessage { id: None, method: Some(method.to_string()), params: Some(params), result: None, error: None });
    }

    fn respond(&self, id: u64, result: serde_json::Value) {
        self.send(&LspMessage { id: Some(id), method: None, params: None, result: Some(result), error: None });
    }

    fn error(&self, id: u64, code: i64, msg: &str) {
        self.send(&LspMessage { id: Some(id), method: None, params: None, result: None, error: Some(LspError { code, message: msg.to_string() }) });
    }

    fn handle_request(&mut self, id: u64, method: &str, params: Option<&serde_json::Value>) {
        match method {
            "initialize" => self.respond(id, serde_json::json!({
                "capabilities": self.capabilities,
                "serverInfo": { "name": "ajeeb-lsp", "version": "0.1.0" }
            })),
            "shutdown" => self.respond(id, serde_json::Value::Null),
            "textDocument/hover" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                let r = self.resolve_hover(uri, line, col).unwrap_or(serde_json::Value::Null);
                self.respond(id, r);
            }
            "textDocument/definition" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                let r = self.find_definition(uri, line, col).unwrap_or(serde_json::Value::Null);
                self.respond(id, r);
            }
            "textDocument/references" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                self.respond(id, serde_json::json!(self.find_references(uri, line, col)));
            }
            "textDocument/prepareRename" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                let r = self.documents.get(uri).and_then(|d| self.find_identifier_at(&d.stmts, line as usize, col as usize));
                match r {
                    Some((name, _)) => self.respond(id, serde_json::json!({
                        "range": { "start": { "line": line, "character": col }, "end": { "line": line, "character": col + name.len() as u64 } },
                        "placeholder": name
                    })),
                    None => self.error(id, -32603, "Cannot rename: identifier not found"),
                }
            }
            "textDocument/rename" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                let new_name = p["newName"].as_str().unwrap_or("");
                match self.rename_symbol(uri, line, col, new_name) {
                    Some(r) => self.respond(id, r),
                    None => self.error(id, -32603, "Cannot rename"),
                }
            }
            "textDocument/completion" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                let line = p["position"]["line"].as_u64().unwrap_or(0);
                let col = p["position"]["character"].as_u64().unwrap_or(0);
                self.respond(id, serde_json::json!({"isIncomplete": false, "items": self.find_completions(uri, line, col)}));
            }
            "textDocument/documentSymbol" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("");
                self.respond(id, serde_json::json!(self.document_symbols(uri)));
            }
            "workspace/symbol" => {
                let query = params.and_then(|p| p["query"].as_str()).unwrap_or("").to_lowercase();
                let mut results = Vec::new();
                for (uri, doc) in &self.documents {
                    for sym in &doc.symbols {
                        if sym.name.to_lowercase().contains(&query) {
                            let k = match sym.kind.as_str() { "function" => 3, "class" => 5, "struct" => 7, "enum" => 10, _ => 6 };
                            results.push(serde_json::json!({
                                "name": sym.name, "kind": k,
                                "location": {
                                    "uri": uri,
                                    "range": {
                                        "start": { "line": sym.line - 1, "character": sym.col - 1 },
                                        "end": { "line": sym.line - 1, "character": sym.col + sym.name.len() - 1 }
                                    }
                                }
                            }));
                        }
                    }
                }
                self.respond(id, serde_json::json!(results));
            }
            _ => { tracing::warn!("Unexpected method: {}", method); self.respond(id, serde_json::Value::Null); }
        }
    }

    fn handle_notification(&mut self, method: &str, params: Option<&serde_json::Value>) {
        match method {
            "initialized" => tracing::info!("Client initialized"),
            "exit" => std::process::exit(0),
            "textDocument/didOpen" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let doc = &p["textDocument"];
                let uri = doc["uri"].as_str().unwrap_or("").to_string();
                let text = doc["text"].as_str().unwrap_or("").to_string();
                let version = doc["version"].as_i64().unwrap_or(1);
                self.documents.insert(uri.clone(), Document { uri: uri.clone(), text, version, stmts: vec![], symbols: vec![], analyzer: None });
                self.analyze_document(&uri);
            }
            "textDocument/didChange" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                let uri = p["textDocument"]["uri"].as_str().unwrap_or("").to_string();
                if let Some(change) = p["contentChanges"].as_array().and_then(|a| a.first()) {
                    let text = change["text"].as_str().unwrap_or("");
                    if let Some(d) = self.documents.get_mut(&uri) { d.text = text.to_string(); d.version = p["textDocument"]["version"].as_i64().unwrap_or(d.version + 1); }
                    self.analyze_document(&uri);
                }
            }
            "textDocument/didSave" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                self.analyze_document(p["textDocument"]["uri"].as_str().unwrap_or(""));
            }
            "textDocument/didClose" => {
                let p = params.unwrap_or(&serde_json::Value::Null);
                self.documents.remove(p["textDocument"]["uri"].as_str().unwrap_or(""));
            }
            _ => tracing::warn!("Unknown notification: {}", method),
        }
    }

    fn run(&mut self) {
        let stdin = io::stdin();
        let mut reader = stdin.lock();
        let mut content_length: Option<usize> = None;
        let mut buf = String::new();
        loop {
            buf.clear();
            match reader.read_line(&mut buf) {
                Ok(0) => break,
                Ok(_) => {
                    let line = buf.trim_end_matches("\r\n").trim_end_matches('\n');
                    if line.is_empty() {
                        if let Some(len) = content_length.take() {
                            let mut body = vec![0u8; len];
                            if read_exact(&mut reader, &mut body).is_err() { continue; }
                            let text = String::from_utf8_lossy(&body);
                            self.handle_message(&text);
                        }
                    } else if line.starts_with("Content-Length:") {
                        content_length = line.trim_start_matches("Content-Length:").trim().parse::<usize>().ok();
                    }
                }
                Err(e) => { tracing::error!("Read error: {}", e); break; }
            }
        }
    }

    fn handle_message(&mut self, body: &str) {
        let msg: LspMessage = match serde_json::from_str(body) { Ok(m) => m, Err(e) => { tracing::error!("Parse error: {} — {}", e, body.chars().take(200).collect::<String>()); return; }};
        if let Some(id) = msg.id {
            if let Some(method) = msg.method { self.handle_request(id, &method, msg.params.as_ref()); }
        } else if let Some(method) = msg.method {
            self.handle_notification(&method, msg.params.as_ref());
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn make_diagnostic(msg: &str, line: usize, col: usize, len: usize, severity: &str) -> serde_json::Value {
    let sev = if severity == "error" { 1 } else { 2 };
    serde_json::json!({
        "range": {
            "start": { "line": line.saturating_sub(1), "character": col.saturating_sub(1) },
            "end": { "line": line.saturating_sub(1), "character": col.saturating_sub(1) + len }
        },
        "severity": sev, "source": "ajeeb", "message": msg
    })
}

fn read_exact<R: io::Read>(reader: &mut R, buf: &mut [u8]) -> io::Result<()> {
    let mut offset = 0;
    while offset < buf.len() {
        match reader.read(&mut buf[offset..]) {
            Ok(0) => return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "eof")),
            Ok(n) => offset += n,
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

fn make_loc(uri: &str, line: usize, col: usize, len: usize) -> serde_json::Value {
    serde_json::json!({
        "uri": uri,
        "range": {
            "start": { "line": line.saturating_sub(1), "character": col.saturating_sub(1) },
            "end": { "line": line.saturating_sub(1), "character": col.saturating_sub(1) + len }
        }
    })
}

fn collect_symbols(stmt: &Stmt, symbols: &mut Vec<SymbolInfo>) {
    let (name, kind, line, col) = match stmt {
        Stmt::FnDef { name, line, col, .. } => (name.clone(), "function", *line, *col),
        Stmt::Class { name, line, col, .. } => (name.clone(), "class", *line, *col),
        Stmt::StructDef { name, line, col, .. } => (name.clone(), "struct", *line, *col),
        Stmt::EnumDef { name, line, col, .. } => (name.clone(), "enum", *line, *col),
        Stmt::Set { name, line, col, .. } => (name.clone(), "variable", *line, *col),
        Stmt::Const { name, line, col, .. } => (name.clone(), "constant", *line, *col),
        Stmt::TraitDef { name, line, col, .. } => (name.clone(), "trait", *line, *col),
        _ => return,
    };
    symbols.push(SymbolInfo { name, kind: kind.to_string(), line, col });
}

fn find_name_in_stmt(stmt: &Stmt, line: usize, col: usize) -> Option<(String, String)> {
    match stmt {
        Stmt::Set { name, line: l, col: c, value, .. } | Stmt::Const { name, line: l, col: c, value, .. } => {
            if *l == line && *c <= col && col <= *c + name.len() { return Some((name.clone(), "variable".to_string())); }
            if let Some(r) = find_name_in_expr(value, line, col) { return Some(r); }
        }
        Stmt::FnDef { name, line: l, col: c, body, .. } => {
            if *l == line && *c <= col && col <= *c + name.len() { return Some((name.clone(), "function".to_string())); }
            for s in body { if let Some(r) = find_name_in_stmt(s, line, col) { return Some(r); } }
        }
        Stmt::If { condition, then_block, else_block, .. } => {
            if let Some(r) = find_name_in_expr(condition, line, col).or_else(|| find_name_in_stmts(then_block, line, col)) { return Some(r); }
            if let Some(eb) = else_block { if let Some(r) = find_name_in_stmts(eb, line, col) { return Some(r); } }
        }
        Stmt::While { condition, body, .. } | Stmt::ForLoop { condition, body, .. } => {
            if let Some(r) = find_name_in_expr(condition, line, col).or_else(|| find_name_in_stmts(body, line, col)) { return Some(r); }
        }
        Stmt::Return { value: Some(v), .. } => { if let Some(r) = find_name_in_expr(v, line, col) { return Some(r); } }
        Stmt::Expr(e, _, _) => { if let Some(r) = find_name_in_expr(e, line, col) { return Some(r); } }
        _ => {}
    }
    None
}

fn find_name_in_stmts(stmts: &[Stmt], line: usize, col: usize) -> Option<(String, String)> {
    for s in stmts { if let Some(r) = find_name_in_stmt(s, line, col) { return Some(r); } }
    None
}

fn find_name_in_expr(expr: &Expr, line: usize, col: usize) -> Option<(String, String)> {
    match expr {
        Expr::Ident(name, l, c) => {
            if *l == line && *c <= col && col <= *c + name.len() { return Some((name.clone(), "variable".to_string())); }
        }
        Expr::Binary { left, right, .. } => {
            if let Some(r) = find_name_in_expr(left, line, col).or_else(|| find_name_in_expr(right, line, col)) { return Some(r); }
        }
        Expr::FnCall { name, args, .. } | Expr::GenericCall { name, args, .. } => {
            for arg in args { if let Some(r) = find_name_in_expr(arg, line, col) { return Some(r); } }
        }
        Expr::MethodCall { obj, args, .. } => {
            if let Some(r) = find_name_in_expr(obj, line, col).or_else(|| find_name_in_stmts_exprs(args, line, col)) { return Some(r); }
        }
        Expr::UnaryMinus(e, _, _) | Expr::UnaryNot(e, _, _) => { if let Some(r) = find_name_in_expr(e, line, col) { return Some(r); } }
        Expr::Index { obj, index, .. } => {
            if let Some(r) = find_name_in_expr(obj, line, col).or_else(|| find_name_in_expr(index, line, col)) { return Some(r); }
        }
        Expr::Field { obj, .. } => { if let Some(r) = find_name_in_expr(obj, line, col) { return Some(r); } }
        Expr::ArrayLit(items, _, _) => { for item in items { if let Some(r) = find_name_in_expr(item, line, col) { return Some(r); } } }
        Expr::StructLit { fields, .. } => { for (_, f) in fields { if let Some(r) = find_name_in_expr(f, line, col) { return Some(r); } } }
        Expr::Assign { value, .. } => { if let Some(r) = find_name_in_expr(value, line, col) { return Some(r); } }
        Expr::Match { value, arms, .. } => {
            if let Some(r) = find_name_in_expr(value, line, col) { return Some(r); }
            for arm in arms { if let Some(r) = find_name_in_expr(&arm.body, line, col) { return Some(r); } }
        }
        _ => {}
    }
    None
}

fn find_name_in_stmts_exprs(exprs: &[Expr], line: usize, col: usize) -> Option<(String, String)> {
    for e in exprs { if let Some(r) = find_name_in_expr(e, line, col) { return Some(r); } }
    None
}

fn find_id_refs(stmts: &[Stmt], name: &str, uri: &str, refs: &mut Vec<serde_json::Value>) {
    for s in stmts { find_ref_in_stmt(s, name, uri, refs); }
}

fn find_ref_in_stmt(stmt: &Stmt, name: &str, uri: &str, refs: &mut Vec<serde_json::Value>) {
    match stmt {
        Stmt::Set { name: n, line, col, value, .. } | Stmt::Const { name: n, line, col, value, .. } => {
            if n == name { refs.push(make_loc(uri, *line, *col, name.len())); }
            find_ref_in_expr(value, name, uri, refs);
        }
        Stmt::FnDef { name: n, line, col, body, .. } => {
            if n == name { refs.push(make_loc(uri, *line, *col, name.len())); }
            for s in body { find_ref_in_stmt(s, name, uri, refs); }
        }
        Stmt::If { condition, then_block, else_block, .. } => {
            find_ref_in_expr(condition, name, uri, refs);
            for s in then_block { find_ref_in_stmt(s, name, uri, refs); }
            if let Some(eb) = else_block { for s in eb { find_ref_in_stmt(s, name, uri, refs); } }
        }
        Stmt::While { condition, body, .. } | Stmt::ForLoop { condition, body, .. } => {
            find_ref_in_expr(condition, name, uri, refs);
            for s in body { find_ref_in_stmt(s, name, uri, refs); }
        }
        Stmt::Return { value: Some(v), .. } => find_ref_in_expr(v, name, uri, refs),
        Stmt::Expr(e, _, _) => find_ref_in_expr(e, name, uri, refs),
        _ => {}
    }
}

fn find_ref_in_expr(expr: &Expr, name: &str, uri: &str, refs: &mut Vec<serde_json::Value>) {
    match expr {
        Expr::Ident(n, l, c) if n == name => refs.push(make_loc(uri, *l, *c, name.len())),
        Expr::Binary { left, right, .. } => { find_ref_in_expr(left, name, uri, refs); find_ref_in_expr(right, name, uri, refs); }
        Expr::FnCall { name: n, args, .. } | Expr::GenericCall { name: n, args, .. } => {
            for arg in args { find_ref_in_expr(arg, name, uri, refs); }
        }
        Expr::MethodCall { obj, args, .. } => { find_ref_in_expr(obj, name, uri, refs); for arg in args { find_ref_in_expr(arg, name, uri, refs); } }
        Expr::UnaryMinus(e, _, _) | Expr::UnaryNot(e, _, _) => find_ref_in_expr(e, name, uri, refs),
        Expr::Index { obj, index, .. } => { find_ref_in_expr(obj, name, uri, refs); find_ref_in_expr(index, name, uri, refs); }
        Expr::Field { obj, .. } => find_ref_in_expr(obj, name, uri, refs),
        Expr::ArrayLit(items, _, _) => { for item in items { find_ref_in_expr(item, name, uri, refs); } }
        Expr::StructLit { fields, .. } => { for (_, f) in fields { find_ref_in_expr(f, name, uri, refs); } }
        Expr::Assign { value, .. } => find_ref_in_expr(value, name, uri, refs),
        Expr::Match { value, arms, .. } => {
            find_ref_in_expr(value, name, uri, refs);
            for arm in arms { find_ref_in_expr(&arm.body, name, uri, refs); }
        }
        _ => {}
    }
}

// ── Main ───────────────────────────────────────────────────────────

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Ajeeb LSP server starting");
    let mut server = LspServer::new();
    server.run();
}
