use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;
use std::collections::HashMap;

impl Parser {
    pub(super) fn parse_import(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let mut path = Vec::new();
        loop {
            match self.advance() {
                Token::Identifier(name) => path.push(name),
                Token::String => path.push("string".to_string()),
                Token::Int => path.push("int".to_string()),
                Token::Bool => path.push("bool".to_string()),
                Token::Float => path.push("float".to_string()),
                _ => return Err(self.err("Import path me identifier chahiye.")),
            }
            if self.peek() == &Token::DoubleColon {
                self.advance();
            } else {
                break;
            }
        }
        let alias = if self.peek() == &Token::Identifier("as".to_string()) {
            self.advance();
            match self.advance() {
                Token::Identifier(a) => Some(a),
                _ => return Err(self.err("'as' ke baad alias name chahiye.")),
            }
        } else {
            None
        };
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Import(ImportDecl { path, alias, c_import: false, line, col }))
    }

    pub(super) fn parse_at_import(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // consume AtImport
        let line = self.line();
        let col = self.col();
        let mut path = Vec::new();
        let mut c_import = false;
        // Check if next token is a string literal (C library import)
        if let Token::StringLiteral(lib_path) = self.peek().clone() {
            self.advance();
            // Split path into path components for consistency
            for part in lib_path.split('/') {
                if !part.is_empty() {
                    path.push(part.to_string());
                }
            }
            if path.is_empty() {
                path.push(lib_path);
            }
            c_import = true;
        } else {
            loop {
                match self.advance() {
                    Token::Identifier(name) => path.push(name),
                    _ => return Err(self.err("'@import' ke baad path chahiye (e.g. @import std.io ya @import \"lib.so\")")),
                }
                if self.peek() == &Token::Dot {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        let alias = if self.peek() == &Token::Identifier("as".to_string()) {
            self.advance();
            match self.advance() {
                Token::Identifier(a) => Some(a),
                _ => return Err(self.err("'as' ke baad alias name chahiye.")),
            }
        } else {
            None
        };
        if self.peek() == &Token::Semicolon {
            self.advance();
        }
        Ok(Stmt::Import(ImportDecl { path, alias, c_import, line, col }))
    }

    pub(super) fn parse_set_decl(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'set' ke baad variable ka naam chahiye.")),
        };
        let type_ann = self.parse_type()?;
        if let Some(ref t) = type_ann {
            self.var_types.insert(name.clone(), t.clone());
        }
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Set { name, type_ann, value, pub_, line, col })
    }

    pub(super) fn parse_const_decl(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'const' ke baad variable ka naam chahiye.")),
        };
        let type_ann = self.parse_type()?;
        if let Some(ref t) = type_ann {
            self.var_types.insert(name.clone(), t.clone());
        }
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Const { name, type_ann, value, pub_, line, col })
    }

    pub(super) fn parse_fn_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            Token::New => "new".to_string(),
            _ => return Err(self.err("'function' ke baad function ka naam chahiye.")),
        };
        // Parse optional generic type params with trait bounds: fn name[T: Trait, U](...
        let mut type_params = Vec::new();
        let mut type_bounds: HashMap<String, Vec<String>> = HashMap::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            let mut seen_params = std::collections::HashSet::new();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        if !seen_params.insert(tp.clone()) {
                            return Err(self.err(&format!("Duplicate type parameter '{}' in generic params", tp)));
                        }
                        type_params.push(tp.clone());
                        // Parse optional trait bounds: T: Trait1 + Trait2
                        if self.peek() == &Token::Colon {
                            self.advance();
                            let mut bounds = Vec::new();
                            loop {
                                match self.advance() {
                                    Token::Identifier(trait_name) => {
                                        bounds.push(trait_name);
                                    }
                                    _ => return Err(self.err("Trait bound name chahiye after ':'")),
                                }
                                if self.peek() == &Token::Plus {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                            type_bounds.insert(tp, bounds);
                        }
                    }
                    _ => return Err(self.err("Generic type parameter name chahiye.")),
                }
                if self.peek() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(&Token::RBracket)?;
        }
        self.generic_type_params.extend(type_params.clone());
        self.generic_type_bounds.extend(type_bounds.clone());
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = self.parse_param_name()?;
            // In functions/methods, 'self' param type is optional (defaults to Void placeholder)
            let ptype = if pname == "self" && self.peek() != &Token::Colon && self.peek() != &Token::Arrow {
                TypeAnnot::Void
            } else {
                match self.parse_type()? {
                    Some(t) => t,
                    None => {
                        return Err(self.err("Parameter ka type batana zaroori hai (jaise x: int)."))
                    }
                }
            };
            params.push((pname, ptype));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        let return_type = match self.parse_type()? {
            Some(t) => t,
            None => TypeAnnot::Void,
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        // Pop generic type params from scope
        for tp in &type_params {
            if let Some(pos) = self.generic_type_params.iter().position(|x| x == tp) {
                self.generic_type_params.remove(pos);
            }
            self.generic_type_bounds.remove(tp);
        }
        Ok(Stmt::FnDef {
            name,
            type_params,
            type_param_bounds: type_bounds.into_iter().collect(),
            params,
            return_type,
            body,
            pub_,
            line,
            col,
        })
    }

    pub(super) fn parse_class_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'class' ke baad naam chahiye.")),
        };
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let old_class = self.current_class.replace(name.clone());
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let field_pub = self.parse_pub();
            if self.peek() == &Token::Function {
                methods.push(self.parse_fn_def(field_pub)?);
            } else {
                let fname = match self.advance() {
                    Token::Identifier(n) => n,
                    _ => return Err(self.err("Field ka naam chahiye.")),
                };
                let ftype = match self.parse_type()? {
                    Some(t) => t,
                    None => return Err(self.err("Field ka type batana zaroori hai.")),
                };
                // Accept both ',' and ';' as field separators (trailing comma/semicolon allowed)
                if self.peek() == &Token::Comma || self.peek() == &Token::Semicolon {
                    self.advance();
                }
                fields.push(ClassField { name: fname, type_ann: ftype, pub_: field_pub });
            }
        }
        self.current_class = old_class;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::Class {
            name,
            fields,
            methods,
            pub_,
            line,
            col,
        })
    }

    pub(super) fn parse_struct_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'struct' ke baad naam chahiye.")),
        };
        // Parse optional generic type params: struct Name[T] or struct Name[T: Display]
        let mut type_params = Vec::new();
        let mut type_param_bounds: Vec<(String, Vec<String>)> = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            let mut seen_params = std::collections::HashSet::new();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        if !seen_params.insert(tp.clone()) {
                            return Err(self.err(&format!("Duplicate type parameter '{}' in generic params", tp)));
                        }
                        type_params.push(tp.clone());
                        // Parse optional trait bounds: T: Trait1 + Trait2
                        if self.peek() == &Token::Colon {
                            self.advance();
                            let mut bounds = Vec::new();
                            loop {
                                match self.advance() {
                                    Token::Identifier(trait_name) => { bounds.push(trait_name); }
                                    _ => return Err(self.err("Trait bound name chahiye after ':'")),
                                }
                                if self.peek() == &Token::Plus { self.advance(); } else { break; }
                            }
                            if !bounds.is_empty() {
                                type_param_bounds.push((tp, bounds));
                            }
                        }
                    }
                    _ => return Err(self.err("Generic type parameter name chahiye.")),
                }
                if self.peek() == &Token::Comma { self.advance(); }
            }
            self.expect(&Token::RBracket)?;
        }
        self.generic_type_params.extend(type_params.clone());
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let fname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Struct field ka naam chahiye.")),
            };
            let ftype = match self.parse_type()? {
                Some(t) => t,
                None => return Err(self.err("Struct field ka type batana zaroori hai.")),
            };
            // Accept both ',' and ';' as field separators (trailing comma/semicolon allowed)
            if self.peek() == &Token::Comma || self.peek() == &Token::Semicolon {
                self.advance();
            }
            fields.push(StructField { name: fname, type_ann: ftype });
        }
        self.expect(&Token::RBrace)?;
        for tp in &type_params {
            if let Some(pos) = self.generic_type_params.iter().position(|x| x == tp) {
                self.generic_type_params.remove(pos);
            }
        }
        Ok(Stmt::StructDef { name, type_params, type_param_bounds, fields, pub_, line, col })
    }

    pub(super) fn parse_enum_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'enum' ke baad naam chahiye.")),
        };
        // Parse optional generic type params: enum Option[T] or enum Option[T: Display]
        let mut type_params = Vec::new();
        let mut type_param_bounds: Vec<(String, Vec<String>)> = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            let mut seen_params = std::collections::HashSet::new();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        if !seen_params.insert(tp.clone()) {
                            return Err(self.err(&format!("Duplicate type parameter '{}' in generic params", tp)));
                        }
                        type_params.push(tp.clone());
                        // Parse optional trait bounds: T: Trait1 + Trait2
                        if self.peek() == &Token::Colon {
                            self.advance();
                            let mut bounds = Vec::new();
                            loop {
                                match self.advance() {
                                    Token::Identifier(trait_name) => { bounds.push(trait_name); }
                                    _ => return Err(self.err("Trait bound name chahiye after ':'")),
                                }
                                if self.peek() == &Token::Plus { self.advance(); } else { break; }
                            }
                            if !bounds.is_empty() {
                                type_param_bounds.push((tp, bounds));
                            }
                        }
                    }
                    _ => return Err(self.err("Generic type parameter name chahiye.")),
                }
                if self.peek() == &Token::Comma { self.advance(); }
            }
            self.expect(&Token::RBracket)?;
        }
        self.generic_type_params.extend(type_params.clone());
        self.expect(&Token::LBrace)?;
        let mut variants = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let vname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Enum variant ka naam chahiye.")),
            };
            let mut fields = Vec::new();
            if self.peek() == &Token::LParen {
                self.advance();
                while self.peek() != &Token::RParen {
                    let ftype = match self.advance() {
                        Token::Int => TypeAnnot::Int,
                        Token::Float => TypeAnnot::Float,
                        Token::String => TypeAnnot::String,
                        Token::Bool => TypeAnnot::Bool,
                        Token::Void => TypeAnnot::Void,
                        Token::Identifier(tname) => {
                            if self.generic_type_params.contains(&tname) {
                                TypeAnnot::Generic(tname)
                            } else {
                                match tname.as_str() {
                                    "Int" => TypeAnnot::Int,
                                    "Float" => TypeAnnot::Float,
                                    "String" => TypeAnnot::String,
                                    "Bool" => TypeAnnot::Bool,
                                    "Void" => TypeAnnot::Void,
                                    _ => TypeAnnot::Class(tname),
                                }
                            }
                        },
                        _ => return Err(self.err("Enum variant field type chahiye.")),
                    };
                    fields.push(ftype);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RParen)?;
            }
            variants.push(EnumVariantDef { name: vname, fields });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        for tp in &type_params {
            if let Some(pos) = self.generic_type_params.iter().position(|x| x == tp) {
                self.generic_type_params.remove(pos);
            }
        }
        Ok(Stmt::EnumDef { name, type_params, type_param_bounds, variants, pub_, line, col })
    }

    pub(super) fn parse_param_name(&mut self) -> Result<String, CompileError> {
        match self.advance() {
            Token::Identifier(n) => Ok(n),
            Token::SelfKwd => Ok("self".to_string()),
            _ => Err(self.err("Parameter ka naam chahiye.")),
        }
    }

    pub(super) fn parse_trait_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'trait' ke baad naam chahiye.")),
        };
        // Parse optional generic type params: trait Display[T] or trait Map[K, V]
        let mut type_params = Vec::new();
        let mut type_param_bounds: Vec<(String, Vec<String>)> = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            let mut seen_params = std::collections::HashSet::new();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        if !seen_params.insert(tp.clone()) {
                            return Err(self.err(&format!("Duplicate type parameter '{}' in trait", tp)));
                        }
                        type_params.push(tp.clone());
                        // Parse optional trait bounds: T: Trait1 + Trait2
                        if self.peek() == &Token::Colon {
                            self.advance();
                            let mut bounds = Vec::new();
                            loop {
                                match self.advance() {
                                    Token::Identifier(trait_name) => { bounds.push(trait_name); }
                                    _ => return Err(self.err("Trait bound name chahiye after ':'")),
                                }
                                if self.peek() == &Token::Plus { self.advance(); } else { break; }
                            }
                            if !bounds.is_empty() {
                                type_param_bounds.push((tp, bounds));
                            }
                        }
                    }
                    _ => return Err(self.err("Generic type parameter name expected in trait[]")),
                }
                if self.peek() == &Token::Comma {
                    self.advance();
                }
            }
            self.expect(&Token::RBracket)?;
        }
        // Push type params into parser scope so method bodies can reference them
        self.generic_type_params.extend(type_params.clone());
        self.expect(&Token::LBrace)?;
        let mut methods = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.peek() == &Token::Function {
                self.advance();
                let mname = match self.advance() {
                    Token::Identifier(n) => n,
                    _ => return Err(self.err("Trait method ka naam chahiye.")),
                };
                self.expect(&Token::LParen)?;
                let mut params = Vec::new();
                while self.peek() != &Token::RParen {
                    let pname = self.parse_param_name()?;
                    // In trait methods, 'self' param type is optional (defaults to Void placeholder)
                    let ptype = if pname == "self" && self.peek() != &Token::Colon && self.peek() != &Token::Arrow {
                        TypeAnnot::Void
                    } else {
                        match self.parse_type()? {
                            Some(t) => t,
                            None => return Err(self.err("Parameter ka type batana zaroori hai.")),
                        }
                    };
                    params.push((pname, ptype));
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RParen)?;
                let return_type = match self.parse_type()? {
                    Some(t) => t,
                    None => TypeAnnot::Void,
                };
                self.expect(&Token::Semicolon)?;
                methods.push(TraitMethod { name: mname, params, return_type });
            } else {
                return Err(self.err("Trait me sirf function signatures allowed hain."));
            }
        }
        self.expect(&Token::RBrace)?;
        // Pop type params from parser scope
        for tp in &type_params {
            self.generic_type_params.retain(|p| p != tp);
        }
        Ok(Stmt::TraitDef { name, type_params, type_param_bounds, methods, pub_, line, col })
    }

    pub(super) fn parse_impl_block(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        // Parse optional generic type params: impl[T] or impl[T: Bound, U]
        let mut type_params = Vec::new();
        let mut type_param_bounds: Vec<(String, Vec<String>)> = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            let mut seen_params = std::collections::HashSet::new();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        if !seen_params.insert(tp.clone()) {
                            return Err(self.err(&format!("Duplicate type parameter '{}' in impl", tp)));
                        }
                        type_params.push(tp.clone());
                        // Parse optional trait bounds: T: Trait1 + Trait2
                        let mut bounds = Vec::new();
                        if self.peek() == &Token::Colon {
                            self.advance();
                            loop {
                                match self.advance() {
                                    Token::Identifier(b) => bounds.push(b),
                                    _ => return Err(self.err("Trait bound expected")),
                                }
                                if self.peek() == &Token::Plus {
                                    self.advance();
                                } else {
                                    break;
                                }
                            }
                        }
                        if !bounds.is_empty() {
                            type_param_bounds.push((tp, bounds));
                        }
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    _ => return Err(self.err("Type parameter name expected in impl[]")),
                }
            }
            self.expect(&Token::RBracket)?;
            // Push type params into parser scope so method bodies can reference them
            self.generic_type_params.extend(type_params.clone());
            for (tp, bounds) in &type_param_bounds {
                self.generic_type_bounds.insert(tp.clone(), bounds.clone());
            }
        }
        let first_name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'impl' ke baad type ya trait ka naam chahiye.")),
        };
        let trait_or_type_name = first_name.clone();
        // Check if type name is followed by generic args: Box[T] or Trait[T]
        let mut trait_type_args = Vec::new();
        let full_type_name = if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            // Parse generic args and build full name like "Box[T]"
            self.advance(); // consume [
            let mut name = first_name.clone();
            name.push('[');
            let mut first = true;
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                // Skip commas between type args
                if self.peek() == &Token::Comma {
                    self.advance();
                    continue;
                }
                if !first { name.push(','); }
                first = false;
                match self.advance() {
                    Token::Identifier(arg) => { name.push_str(&arg); trait_type_args.push(arg); }
                    Token::Int => { name.push_str("Int"); trait_type_args.push("Int".to_string()); }
                    Token::Float => { name.push_str("Float"); trait_type_args.push("Float".to_string()); }
                    Token::String => { name.push_str("String"); trait_type_args.push("String".to_string()); }
                    Token::Bool => { name.push_str("Bool"); trait_type_args.push("Bool".to_string()); }
                    _ => return Err(self.err("Type argument expected in impl")),
                }
            }
            self.expect(&Token::RBracket)?;
            name.push(']');
            name
        } else {
            first_name
        };
        // Check if next token is 'for' (trait impl) or '{' (inherent impl)
        if self.peek() == &Token::For {
            // Trait impl: impl Trait for Type { ... }
            self.advance();
            let type_name_base = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Trait impl ke liye type ka naam chahiye.")),
            };
            // Parse optional generic type args on the type name: Option[Int]
            let type_name = if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
                self.advance(); // consume [
                let mut name = type_name_base.clone();
                name.push('[');
                let mut first = true;
                while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                    if self.peek() == &Token::Comma {
                        self.advance();
                        continue;
                    }
                    if !first { name.push(','); }
                    first = false;
                    match self.advance() {
                        Token::Identifier(arg) => name.push_str(&arg),
                        Token::Int => name.push_str("Int"),
                        Token::Float => name.push_str("Float"),
                        Token::String => name.push_str("String"),
                        Token::Bool => name.push_str("Bool"),
                        _ => return Err(self.err("Type argument expected")),
                    }
                }
                self.expect(&Token::RBracket)?;
                name.push(']');
                name
            } else {
                type_name_base
            };
            self.expect(&Token::LBrace)?;
            let mut methods = Vec::new();
            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                methods.push(self.parse_fn_def(false)?);
            }
            self.expect(&Token::RBrace)?;
            // Pop type params from parser scope
            for tp in &type_params {
                self.generic_type_params.retain(|p| p != tp);
                self.generic_type_bounds.remove(tp);
            }
            Ok(Stmt::ImplBlock { trait_name: Some(trait_or_type_name), trait_type_args, type_params, type_param_bounds, type_name, methods, line, col })
        } else if self.peek() == &Token::LBrace {
            // Inherent impl: impl Type { ... }
            self.expect(&Token::LBrace)?;
            let mut methods = Vec::new();
            while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
                methods.push(self.parse_fn_def(false)?);
            }
            self.expect(&Token::RBrace)?;
            // Pop type params from parser scope
            for tp in &type_params {
                self.generic_type_params.retain(|p| p != tp);
                self.generic_type_bounds.remove(tp);
            }
            Ok(Stmt::ImplBlock { trait_name: None, trait_type_args: Vec::new(), type_params, type_param_bounds, type_name: full_type_name, methods, line, col })
        } else {
            Err(self.err("'impl' ke baad '{' ya 'for' expected hai."))
        }
    }
}

