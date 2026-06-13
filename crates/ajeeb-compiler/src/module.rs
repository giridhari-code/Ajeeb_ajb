use crate::ast::{ImportDecl, Stmt};
use crate::error::CompileError;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct Module {
    pub name: String,
    pub file_path: PathBuf,
    pub imports: Vec<ImportDecl>,
    pub stmts: Vec<Stmt>,
}

pub struct ModuleLoader {
    pub modules: HashMap<String, Module>,
    pub errors: Vec<CompileError>,
    import_paths: Vec<PathBuf>,
}

impl ModuleLoader {
    pub fn new() -> Self {
        ModuleLoader {
            modules: HashMap::new(),
            errors: Vec::new(),
            import_paths: Vec::new(),
        }
    }

    pub fn add_import_path(&mut self, path: PathBuf) {
        self.import_paths.push(path);
    }

    pub fn load_entry(&mut self, file_path: &Path) -> Option<&Module> {
        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("main")
            .to_string();

        self.import_paths.push(
            file_path.parent().unwrap_or(Path::new(".")).to_path_buf(),
        );

        match self.load_module_file(&module_name, file_path) {
            Ok(()) => self.modules.get(&module_name),
            Err(e) => {
                self.errors.push(e);
                None
            }
        }
    }

    fn load_module_file(
        &mut self,
        name: &str,
        file_path: &Path,
    ) -> Result<(), CompileError> {
        if self.modules.contains_key(name) {
            return Ok(());
        }

        let source = std::fs::read_to_string(file_path).map_err(|e| {
            CompileError::new(
                0,
                0,
                format!("Cannot read module '{}' from {}: {}", name, file_path.display(), e),
            )
        })?;

        let mut lexer = Lexer::new(&source);
        let mut tokens = Vec::new();
        let mut token_lines = Vec::new();
        let mut token_cols = Vec::new();

        loop {
            match lexer.next_token_spanned() {
                Ok((Token::Eof, _, _)) => break,
                Ok((tok, line, col)) => {
                    tokens.push(tok);
                    token_lines.push(line);
                    token_cols.push(col);
                }
                Err(e) => return Err(e),
            }
        }

        let mut parser = Parser::with_positions(tokens, token_lines, token_cols);
        let stmts = parser.parse_program()?;

        let mut imports = Vec::new();
        for stmt in &stmts {
            if let Stmt::Import(import) = stmt {
                imports.push(import.clone());
            }
        }

        let module = Module {
            name: name.to_string(),
            file_path: file_path.to_path_buf(),
            imports,
            stmts,
        };

        self.modules.insert(name.to_string(), module);

        // Recursively load dependencies
        let import_list = self.modules[name].imports.clone();
        for import in &import_list {
            self.resolve_import(import)?;
        }

        Ok(())
    }

    fn resolve_import(&mut self, import: &ImportDecl) -> Result<(), CompileError> {
        let module_name = import.alias.clone().unwrap_or_else(|| {
            import.path.last().cloned().unwrap_or_default()
        });

        if self.modules.contains_key(&module_name) {
            return Ok(());
        }

        // Try each import path
        for base in &self.import_paths {
            let rel_path: PathBuf = import.path.iter().collect();
            // Try mod.ajb
            let candidate1 = base.join(&rel_path).join("mod.ajb");
            if candidate1.exists() {
                return self.load_module_file(&module_name, &candidate1);
            }
            // Try module_name.ajb
            let candidate2 = base.join(&rel_path).with_extension("ajb");
            if candidate2.exists() {
                return self.load_module_file(&module_name, &candidate2);
            }
        }

        Err(CompileError::new(
            import.line,
            import.col,
            format!(
                "Module '{}' not found in any import path. Searched paths: {}",
                import.path.join("::"),
                self.import_paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
            ),
        ))
    }

    pub fn collect_all_stmts(&self) -> Vec<Stmt> {
        let mut all_stmts = Vec::new();
        // Topological sort using owned strings
        let order = self.topo_sort_owned();
        for name in &order {
            if let Some(module) = self.modules.get(name) {
                all_stmts.extend(module.stmts.iter().filter(|s| !matches!(s, Stmt::Import(_))).cloned());
            }
        }
        all_stmts
    }

    fn topo_sort_owned(&self) -> Vec<String> {
        let mut ordered = Vec::new();
        let mut visited: HashMap<String, bool> = HashMap::new();
        let names: Vec<String> = self.modules.keys().cloned().collect();
        for name in &names {
            self.topo_visit(name, &mut visited, &mut ordered);
        }
        ordered
    }

    fn topo_visit(
        &self,
        name: &str,
        visited: &mut HashMap<String, bool>,
        ordered: &mut Vec<String>,
    ) {
        if let Some(&in_progress) = visited.get(name) {
            if in_progress {
                return;
            }
            return;
        }
        visited.insert(name.to_string(), true);
        if let Some(module) = self.modules.get(name) {
            for import in &module.imports {
                let dep_name = import.alias.clone().unwrap_or_else(|| {
                    import.path.last().cloned().unwrap_or_default()
                });
                if self.modules.contains_key(&dep_name) {
                    self.topo_visit(&dep_name, visited, ordered);
                }
            }
        }
        visited.insert(name.to_string(), false);
        ordered.push(name.to_string());
    }

    /// Resolve all imports for all registered modules recursively.
    pub fn resolve_imports(&mut self) -> Result<(), CompileError> {
        // Collect all module names; resolve each module's imports
        let names: Vec<String> = self.modules.keys().cloned().collect();
        for name in names {
            self.resolve_module_imports(&name)?;
        }
        Ok(())
    }

    fn resolve_module_imports(&mut self, name: &str) -> Result<(), CompileError> {
        if let Some(module) = self.modules.get(name) {
            let imports = module.imports.clone();
            for import in &imports {
                self.resolve_import(import)?;
            }
        }
        Ok(())
    }
}

// Re-import Token for use in this module
use crate::token::Token;
