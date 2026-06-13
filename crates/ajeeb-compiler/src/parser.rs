use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    token_lines: Vec<usize>,
    token_cols: Vec<usize>,
    pos: usize,
    var_types: HashMap<String, TypeAnnot>,
    current_class: Option<String>,
    generic_type_params: Vec<String>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        let token_lines = vec![0; tokens.len()];
        let token_cols = vec![0; tokens.len()];
        Parser {
            tokens,
            token_lines,
            token_cols,
            pos: 0,
            var_types: HashMap::new(),
            current_class: None,
            generic_type_params: Vec::new(),
        }
    }

    pub fn with_positions(tokens: Vec<Token>, lines: Vec<usize>, cols: Vec<usize>) -> Self {
        Parser {
            tokens,
            token_lines: lines,
            token_cols: cols,
            pos: 0,
            var_types: HashMap::new(),
            current_class: None,
            generic_type_params: Vec::new(),
        }
    }

    fn line(&self) -> usize {
        self.token_lines
            .get(self.pos.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }

    fn col(&self) -> usize {
        self.token_cols
            .get(self.pos.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }

    fn err_at(&self, msg: &str, line: usize, col: usize) -> CompileError {
        CompileError::new(line, col, msg.to_string())
    }

    fn err(&self, msg: impl Into<String>) -> CompileError {
        CompileError::new(self.line(), self.col(), msg.into())
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, CompileError> {
        let t = self.advance();
        if std::mem::discriminant(&t) != std::mem::discriminant(expected) {
            return Err(self.err(format!(
                "'{}' expected but kuch aur mila. Check karo.",
                self.token_debug(expected)
            )));
        }
        match expected {
            Token::Identifier(_) => {
                if let Token::Identifier(_) = &t {
                    Ok(t)
                } else {
                    Err(self.err("Identifier expected tha."))
                }
            }
            _ => Ok(t),
        }
    }

    fn expr_pos(e: &Expr) -> (usize, usize) {
        match e {
            Expr::Number(_, l, c)
            | Expr::FloatLit(_, l, c)
            | Expr::StringLit(_, l, c)
            | Expr::Bool(_, l, c)
            | Expr::Ident(_, l, c)
            | Expr::ArrayLit(_, l, c)
            | Expr::UnaryMinus(_, l, c)
            | Expr::UnaryNot(_, l, c)
            | Expr::Group(_, l, c) => (*l, *c),
            Expr::Binary { line, col, .. }
            | Expr::Assign { line, col, .. }
            | Expr::IndexAssign { line, col, .. }
            | Expr::FnCall { line, col, .. }
            | Expr::MethodCall { line, col, .. }
            | Expr::New { line, col, .. }
            | Expr::Index { line, col, .. }
            | Expr::Field { line, col, .. }
            | Expr::FieldAssign { line, col, .. }
            | Expr::StructLit { line, col, .. }
            | Expr::EnumRef { line, col, .. }
            | Expr::EnumCtor { line, col, .. }
            | Expr::Match { line, col, .. }
            | Expr::GenericCall { line, col, .. } => (*line, *col),
        }
    }

    fn token_debug(&self, t: &Token) -> &'static str {
        match t {
            Token::Let => "let",
            Token::Const => "const",
            Token::If => "if",
            Token::Else => "else",
            Token::While => "while",
            Token::Function => "function",
            Token::Return => "return",
            Token::Int => "int",
            Token::Float => "float",
            Token::String => "string",
            Token::Bool => "bool",
            Token::Void => "void",
            Token::Semicolon => ";",
            Token::Colon => ":",
            Token::DoubleColon => "::",
            Token::Comma => ",",
            Token::LParen => "(",
            Token::RParen => ")",
            Token::LBrace => "{",
            Token::RBrace => "}",
            Token::Assign => "=",
            Token::Eq => "==",
            Token::Neq => "!=",
            Token::Lt => "<",
            Token::Gt => ">",
            Token::Le => "<=",
            Token::Ge => ">=",
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Arrow => "->",
            Token::Dot => ".",
            Token::LBracket => "[",
            Token::RBracket => "]",
            Token::Class => "class",
            Token::SelfKwd => "self",
            Token::New => "new",
            Token::For => "for",
            Token::Break => "break",
            Token::Continue => "continue",
            Token::Import => "import",
            Token::Pub => "pub",
            Token::And => "&&",
            Token::Or => "||",
            Token::Not => "!",
            Token::Struct => "struct",
            Token::Enum => "enum",
            Token::Match => "match",
            Token::FatArrow => "=>",
            Token::Underscore => "_",
            Token::Trait => "trait",
            Token::Impl => "impl",
            Token::Identifier(_) => "identifier",
            Token::Number(_) => "number",
            Token::FloatLiteral(_) => "float",
            Token::StringLiteral(_) => "string literal",
            Token::True | Token::False => "boolean",
            _ => "unknown",
        }
    }

    fn parse_type(&mut self) -> Result<Option<TypeAnnot>, CompileError> {
        if self.peek() == &Token::Colon {
            self.advance();
            let ty = self.parse_type_postfix()?;
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    /// Parse a single type name (without `:` prefix), then handle postfix `[]` and `[Args]`.
    /// Used both from parse_type and from generic arg parsing.
    fn parse_type_postfix(&mut self) -> Result<TypeAnnot, CompileError> {
        let mut base = self.parse_single_type()?;
        loop {
            if self.peek() == &Token::LBracket {
                let next = self.peek_next();
                if next == Some(&Token::RBracket) {
                    // Array shorthand: TypeName[]
                    self.advance();
                    self.advance();
                    base = TypeAnnot::Array(Box::new(base));
                } else {
                    // Generic args: TypeName[Arg1, Arg2, ...]
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                        args.push(self.parse_type_postfix()?);
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RBracket)?;
                    base = TypeAnnot::Parameterized {
                        base: Box::new(base),
                        args,
                    };
                }
            } else {
                break;
            }
        }
        Ok(base)
    }

    fn parse_single_type(&mut self) -> Result<TypeAnnot, CompileError> {
        match self.advance() {
            Token::Int => Ok(TypeAnnot::Int),
            Token::Float => Ok(TypeAnnot::Float),
            Token::String => Ok(TypeAnnot::String),
            Token::Bool => Ok(TypeAnnot::Bool),
            Token::Void => Ok(TypeAnnot::Void),
            Token::Identifier(name) => {
                if self.generic_type_params.contains(&name) {
                    Ok(TypeAnnot::Generic(name))
                } else {
                    match name.as_str() {
                        "Int" => Ok(TypeAnnot::Int),
                        "Float" => Ok(TypeAnnot::Float),
                        "String" => Ok(TypeAnnot::String),
                        "Bool" => Ok(TypeAnnot::Bool),
                        "Void" => Ok(TypeAnnot::Void),
                        _ => Ok(TypeAnnot::Class(name)),
                    }
                }
            }
            other => Err(self.err(format!(
                "Unknown type {:?}. Allowed: int, float, string, bool, void, class names.",
                other
            ))),
        }
    }

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::Eof {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_pub(&mut self) -> bool {
        if self.peek() == &Token::Pub {
            self.advance();
            true
        } else {
            false
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        match self.peek() {
            Token::Import => self.parse_import(),
            _ => {
                let pub_ = self.parse_pub();
                match self.peek() {
                    Token::Let => self.parse_let_decl(pub_),
                    Token::Const => self.parse_const_decl(pub_),
                    Token::If => self.parse_if_stmt(),
                    Token::While => self.parse_while_stmt(),
                    Token::For => self.parse_for_stmt(),
                    Token::Function => self.parse_fn_def(pub_),
                    Token::Return => self.parse_return_stmt(),
                    Token::Break => {
                        self.advance();
                        let line = self.line();
                        let col = self.col();
                        self.expect(&Token::Semicolon)?;
                        Ok(Stmt::Break { line, col })
                    }
                    Token::Continue => {
                        self.advance();
                        let line = self.line();
                        let col = self.col();
                        self.expect(&Token::Semicolon)?;
                        Ok(Stmt::Continue { line, col })
                    }
                    Token::Class => self.parse_class_def(pub_),
                    Token::Struct => self.parse_struct_def(pub_),
                    Token::Enum => self.parse_enum_def(pub_),
                    Token::Match => {
                        let expr = self.parse_match_expr()?;
                        Ok(Stmt::Expr(expr, self.line(), self.col()))
                    }
                    Token::Trait => self.parse_trait_def(pub_),
                    Token::Impl => self.parse_impl_block(),
                    Token::RBrace => Err(self.err("Extra '}' mil gaya.")),
                    _ => self.parse_expr_stmt(),
                }
            }
        }
    }

    fn parse_import(&mut self) -> Result<Stmt, CompileError> {
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
        Ok(Stmt::Import(ImportDecl { path, alias, line, col }))
    }

    fn parse_let_decl(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'let' ke baad variable ka naam chahiye.")),
        };
        let type_ann = self.parse_type()?;
        if let Some(ref t) = type_ann {
            self.var_types.insert(name.clone(), t.clone());
        }
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Let { name, type_ann, value, pub_, line, col })
    }

    fn parse_const_decl(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
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

    fn parse_if_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let then_block = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        let mut else_block = None;
        if self.peek() == &Token::Else {
            self.advance();
            if self.peek() == &Token::If {
                let elif = self.parse_if_stmt()?;
                else_block = Some(vec![elif]);
            } else {
                self.expect(&Token::LBrace)?;
                let block = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                else_block = Some(block);
            }
        }
        Ok(Stmt::If {
            condition,
            then_block,
            else_block,
            line,
            col,
        })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::While { condition, body, line, col })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        self.expect(&Token::LParen)?;
        let init = if self.peek() == &Token::Let {
            self.parse_let_decl(false)?
        } else if self.peek() == &Token::Semicolon {
            self.advance();
            Stmt::Expr(Expr::Number(0, 0, 0), 0, 0)
        } else {
            self.parse_expr_stmt()?
        };
        let condition = if self.peek() == &Token::Semicolon {
            Expr::Number(1, 0, 0)
        } else {
            let e = self.parse_expression()?;
            self.expect(&Token::Semicolon)?;
            e
        };
        let update = if self.peek() == &Token::RParen {
            Stmt::Expr(Expr::Number(0, 0, 0), 0, 0)
        } else {
            let e = self.parse_expression()?;
            Stmt::Expr(e, line, col)
        };
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::ForLoop {
            init: Box::new(init),
            condition,
            update: Box::new(update),
            body,
            line,
            col,
        })
    }

    fn parse_fn_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'function' ke baad function ka naam chahiye.")),
        };
        // Parse optional generic type params: fn name[T, U](...
        let mut type_params = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => {
                        type_params.push(tp);
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
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = self.parse_param_name()?;
            let ptype = match self.parse_type()? {
                Some(t) => t,
                None => {
                    return Err(self.err("Parameter ka type batana zaroori hai (jaise x: int)."))
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
        }
        Ok(Stmt::FnDef {
            name,
            type_params,
            params,
            return_type,
            body,
            pub_,
            line,
            col,
        })
    }

    fn parse_class_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
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
                self.expect(&Token::Semicolon)?;
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

    fn parse_struct_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'struct' ke baad naam chahiye.")),
        };
        // Parse optional generic type params: struct Name[T]
        let mut type_params = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => type_params.push(tp),
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
            self.expect(&Token::Semicolon)?;
            fields.push(StructField { name: fname, type_ann: ftype });
        }
        self.expect(&Token::RBrace)?;
        for tp in &type_params {
            if let Some(pos) = self.generic_type_params.iter().position(|x| x == tp) {
                self.generic_type_params.remove(pos);
            }
        }
        Ok(Stmt::StructDef { name, type_params, fields, pub_, line, col })
    }

    fn parse_enum_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'enum' ke baad naam chahiye.")),
        };
        // Parse optional generic type params: enum Option[T]
        let mut type_params = Vec::new();
        if self.peek() == &Token::LBracket && self.peek_next().map_or(false, |t| matches!(t, Token::Identifier(_) | Token::RBracket)) {
            self.advance();
            while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
                match self.advance() {
                    Token::Identifier(tp) => type_params.push(tp),
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
        Ok(Stmt::EnumDef { name, type_params, variants, pub_, line, col })
    }

    fn parse_param_name(&mut self) -> Result<String, CompileError> {
        match self.advance() {
            Token::Identifier(n) => Ok(n),
            Token::SelfKwd => Ok("self".to_string()),
            _ => Err(self.err("Parameter ka naam chahiye.")),
        }
    }

    fn parse_trait_def(&mut self, pub_: bool) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'trait' ke baad naam chahiye.")),
        };
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
                    let ptype = match self.parse_type()? {
                        Some(t) => t,
                        None => return Err(self.err("Parameter ka type batana zaroori hai.")),
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
        Ok(Stmt::TraitDef { name, methods, pub_, line, col })
    }

    fn parse_impl_block(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let trait_name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'impl' ke baad trait ka naam chahiye.")),
        };
        self.expect(&Token::For)?;
        let type_name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("Trait impl ke liye type ka naam chahiye.")),
        };
        self.expect(&Token::LBrace)?;
        let mut methods = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            methods.push(self.parse_fn_def(false)?);
        }
        self.expect(&Token::RBrace)?;
        Ok(Stmt::ImplBlock { trait_name, type_name, methods, line, col })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        if self.peek() == &Token::Semicolon {
            self.advance();
            Ok(Stmt::Return { value: None, line, col })
        } else {
            let value = self.parse_expression()?;
            self.expect(&Token::Semicolon)?;
            Ok(Stmt::Return { value: Some(value), line, col })
        }
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, CompileError> {
        let line = self.line();
        let col = self.col();
        let expr = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Expr(expr, line, col))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_expression(&mut self) -> Result<Expr, CompileError> {
        if self.peek() == &Token::Match {
            return self.parse_match_expr();
        }
        self.parse_assignment()
    }

    fn parse_match_expr(&mut self) -> Result<Expr, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let value = self.parse_expression()?;
        self.expect(&Token::LBrace)?;
        let mut arms = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let pattern = self.parse_pattern()?;
            self.expect(&Token::FatArrow)?;
            let (body, body_block) = if self.peek() == &Token::LBrace {
                self.advance();
                let stmts = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                let last_expr = stmts.iter().rev().find_map(|s| {
                    if let Stmt::Expr(e, ..) = s { Some(e.clone()) } else { None }
                }).unwrap_or(Expr::Number(0, 0, 0));
                (last_expr, Some(stmts))
            } else {
                (self.parse_expression()?, None)
            };
            arms.push(MatchArm { pattern, body, body_block });
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::Match {
            value: Box::new(value),
            arms,
            line,
            col,
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, CompileError> {
        match self.peek() {
            Token::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Identifier(name) if name == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            Token::Number(n) => {
                let v = *n;
                self.advance();
                Ok(Pattern::Int(v))
            }
            Token::StringLiteral(s) => {
                let v = s.clone();
                self.advance();
                Ok(Pattern::String(v))
            }
            Token::Identifier(first) => {
                let first = first.clone();
                self.advance();
                if self.peek() == &Token::DoubleColon {
                    self.advance();
                    let variant = match self.advance() {
                        Token::Identifier(v) => v,
                        _ => return Err(self.err("Enum variant name chahiye after ::")),
                    };
                    let mut bindings = Vec::new();
                    if self.peek() == &Token::LParen {
                        self.advance();
                        while self.peek() != &Token::RParen {
                            match self.advance() {
                                Token::Identifier(b) => bindings.push(b),
                                _ => return Err(self.err("Binding name chahiye in pattern")),
                            }
                            if self.peek() == &Token::Comma {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RParen)?;
                    }
                    Ok(Pattern::EnumVariant {
                        enum_name: first,
                        variant,
                        bindings,
                    })
                } else {
                    // Not :: — treat as identifier binding pattern
                    // For simplicity, just parse as wildcard + use identifier
                    // Actually, just push as named binding
                    Ok(Pattern::Wildcard)
                }
            }
            _ => Err(self.err("Invalid pattern. Use _, enum::Variant, or literal.")),
        }
    }

    fn parse_assignment(&mut self) -> Result<Expr, CompileError> {
        let expr = self.parse_or()?;
        if self.peek() == &Token::Assign {
            self.advance();
            let line = self.line();
            let col = self.col();
            match expr {
                Expr::Ident(name, ..) => {
                    let value = self.parse_assignment()?;
                    Ok(Expr::Assign {
                        name,
                        value: Box::new(value),
                        line,
                        col,
                    })
                }
                Expr::Field { obj, field, .. } => {
                    let value = self.parse_assignment()?;
                    Ok(Expr::FieldAssign {
                        obj,
                        field,
                        value: Box::new(value),
                        line,
                        col,
                    })
                }
                Expr::Index { obj, index, .. } => {
                    let value = self.parse_assignment()?;
                    Ok(Expr::IndexAssign {
                        obj,
                        index,
                        value: Box::new(value),
                        line,
                        col,
                    })
                }
                _ => {
                    Err(self.err("Assignment left side must be variable, field, or index."))
                }
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_and()?;
        while self.peek() == &Token::Or {
            self.advance();
            let line = self.line();
            let col = self.col();
            let right = self.parse_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::Or,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_equality()?;
        while self.peek() == &Token::And {
            self.advance();
            let line = self.line();
            let col = self.col();
            let right = self.parse_equality()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinOp::And,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_comparison()?;
        while self.peek() == &Token::Eq || self.peek() == &Token::Neq {
            let op = match self.advance() {
                Token::Eq => BinOp::Eq,
                _ => BinOp::Neq,
            };
            let line = self.line();
            let col = self.col();
            let right = self.parse_comparison()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_term()?;
        while self.peek() == &Token::Lt
            || self.peek() == &Token::Gt
            || self.peek() == &Token::Le
            || self.peek() == &Token::Ge
        {
            let op = match self.advance() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                _ => BinOp::Ge,
            };
            let line = self.line();
            let col = self.col();
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_factor()?;
        while self.peek() == &Token::Plus || self.peek() == &Token::Minus {
            let op = match self.advance() {
                Token::Plus => BinOp::Add,
                _ => BinOp::Sub,
            };
            let line = self.line();
            let col = self.col();
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_unary()?;
        while self.peek() == &Token::Star || self.peek() == &Token::Slash {
            let op = match self.advance() {
                Token::Star => BinOp::Mul,
                _ => BinOp::Div,
            };
            let line = self.line();
            let col = self.col();
            let right = self.parse_unary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                line,
                col,
            };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if self.peek() == &Token::Minus {
            self.advance();
            let line = self.line();
            let col = self.col();
            let expr = self.parse_unary()?;
            Ok(Expr::UnaryMinus(Box::new(expr), line, col))
        } else if self.peek() == &Token::Not {
            self.advance();
            let line = self.line();
            let col = self.col();
            let expr = self.parse_unary()?;
            Ok(Expr::UnaryNot(Box::new(expr), line, col))
        } else {
            self.parse_primary()
        }
    }

    fn parse_struct_lit(&mut self, name: String, line: usize, col: usize) -> Result<Expr, CompileError> {
        self.advance(); // consume {
        let mut fields = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            let fname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Struct literal me field name chahiye.")),
            };
            self.expect(&Token::Colon)?;
            let fvalue = self.parse_expression()?;
            fields.push((fname, fvalue));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::StructLit {
            struct_name: name,
            fields,
            line,
            col,
        })
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let mut expr = match self.peek() {
            Token::Number(n) => {
                let v = *n;
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::Number(v, line, col)
            }
            Token::FloatLiteral(f) => {
                let v = *f;
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::FloatLit(v, line, col)
            }
            Token::StringLiteral(s) => {
                let v = s.clone();
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::StringLit(v, line, col)
            }
            Token::True => {
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::Bool(true, line, col)
            }
            Token::False => {
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::Bool(false, line, col)
            }
            Token::SelfKwd => {
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::Ident("self".to_string(), line, col)
            }
            Token::New => {
                self.advance();
                let line = self.line();
                let col = self.col();
                let class_name = match self.advance() {
                    Token::Identifier(n) => n,
                    _ => return Err(self.err("'new' ke baad class ka naam chahiye.")),
                };
                Expr::New { class_name, line, col }
            }
            Token::LBracket => {
                self.advance();
                let line = self.line();
                let col = self.col();
                let mut elems = Vec::new();
                while self.peek() != &Token::RBracket {
                    elems.push(self.parse_expression()?);
                    if self.peek() == &Token::Comma {
                        self.advance();
                    }
                }
                self.expect(&Token::RBracket)?;
                Expr::ArrayLit(elems, line, col)
            }
            Token::LParen => {
                self.advance();
                let line = self.line();
                let col = self.col();
                let e = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Expr::Group(Box::new(e), line, col)
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                let line = self.line();
                let col = self.col();
                // Check for EnumRef or StructLit
                if self.peek() == &Token::DoubleColon {
                    self.advance();
                    let variant = match self.advance() {
                        Token::Identifier(v) => v,
                        _ => return Err(self.err("Enum variant name expected after ::")),
                    };
                    if self.peek() == &Token::LParen {
                        let args = self.parse_call_args()?;
                        Expr::EnumCtor {
                            enum_name: name,
                            variant,
                            args,
                            line,
                            col,
                        }
                    } else {
                        Expr::EnumRef {
                            enum_name: name,
                            variant,
                            line,
                            col,
                        }
                    }
                } else if self.peek() == &Token::LBrace && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    return self.parse_struct_lit(name, line, col);
                } else if self.peek() == &Token::LBracket && self.peek_type_args() {
                    // Generic function call: fnName[TypeArgs](args)
                    let type_args = self.parse_type_arg_list()?;
                    if self.peek() == &Token::LParen {
                        let args = self.parse_call_args()?;
                        Expr::GenericCall { name, type_args, args, line, col }
                    } else {
                        return Err(self.err("Generic function call requires arguments: fn[T](...)."));
                    }
                } else {
                    Expr::Ident(name, line, col)
                }
            }
            Token::Underscore => {
                self.advance();
                let line = self.line();
                let col = self.col();
                // _ is a valid expression (used in patterns, but as expression just treat as ident)
                Expr::Ident("_".to_string(), line, col)
            }
            _ => {
                return Err(self.err(format!(
                    "Unexpected token. Expecting expression, got {:?}.",
                    self.peek()
                )))
            }
        };
        // Postfix operators: calls, field access, index
        loop {
            match self.peek() {
                Token::LParen => {
                    let (line, col) = Self::expr_pos(&expr);
                    let name = match &expr {
                        Expr::Ident(n, ..) => n.clone(),
                        Expr::MethodCall { obj, method, .. } => {
                            let args = self.parse_call_args()?;
                            expr = Expr::MethodCall {
                                obj: obj.clone(),
                                method: method.clone(),
                                args,
                                line,
                                col,
                            };
                            continue;
                        }
                        _ => return Err(self.err("Only identifiers and methods can be called.")),
                    };
                    let args = self.parse_call_args()?;
                    expr = Expr::FnCall { name, args, line, col };
                }
                Token::Dot => {
                    let (line, col) = Self::expr_pos(&expr);
                    self.advance();
                    let field = match self.advance() {
                        Token::Identifier(n) => n,
                        _ => return Err(self.err("'.' ke baad field/method ka naam chahiye.")),
                    };
                    if self.peek() == &Token::LParen {
                        let args = self.parse_call_args()?;
                        expr = Expr::MethodCall {
                            obj: Box::new(expr),
                            method: field,
                            args,
                            line,
                            col,
                        };
                    } else {
                        expr = Expr::Field {
                            obj: Box::new(expr),
                            field,
                            line,
                            col,
                        };
                    }
                }
                Token::LBracket => {
                    let (line, col) = Self::expr_pos(&expr);
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket)?;
                    if self.peek() == &Token::Assign {
                        self.advance();
                        let value = self.parse_expression()?;
                        expr = Expr::IndexAssign {
                            obj: Box::new(expr),
                            index: Box::new(index),
                            value: Box::new(value),
                            line,
                            col,
                        };
                    } else {
                        expr = Expr::Index {
                            obj: Box::new(expr),
                            index: Box::new(index),
                            line,
                            col,
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    /// Peek ahead to see if the next `[...]` contains type arguments (not an index expression).
    /// Looks for uppercase identifiers, type keywords, or known generic params inside the brackets.
    fn peek_type_args(&self) -> bool {
        let mut p = self.pos + 1; // skip past '['
        let mut has_content = false;
        loop {
            match self.tokens.get(p) {
                Some(Token::RBracket) => {
                    return has_content; // closing bracket: true if we saw type-like content
                }
                Some(Token::Identifier(s)) => {
                    // Uppercase start = type name; also check known generic params
                    if s.chars().next().map_or(false, |c| c.is_uppercase())
                        || self.generic_type_params.contains(s)
                        || matches!(s.as_str(), "Int" | "Float" | "String" | "Bool" | "Void")
                    {
                        has_content = true;
                        p += 1;
                    } else {
                        return false;
                    }
                }
                Some(Token::Int | Token::Float | Token::String | Token::Bool | Token::Void) => {
                    has_content = true;
                    p += 1;
                }
                Some(Token::Comma) => {
                    p += 1;
                }
                Some(Token::LBracket) => {
                    // Nested brackets for nested generics (e.g., List[Map[K, V]])
                    has_content = true;
                    p += 1;
                    // Skip to matching ]
                    let mut depth = 1;
                    while let Some(tok) = self.tokens.get(p) {
                        if tok == &Token::LBracket { depth += 1; }
                        else if tok == &Token::RBracket { depth -= 1; if depth == 0 { break; } }
                        p += 1;
                    }
                    p += 1;
                }
                _ => return false,
            }
        }
    }

    /// Parse `[TypeArg1, TypeArg2, ...]` and return the list of type arguments.
    fn parse_type_arg_list(&mut self) -> Result<Vec<TypeAnnot>, CompileError> {
        self.advance(); // skip '['
        let mut args = Vec::new();
        while self.peek() != &Token::RBracket && self.peek() != &Token::Eof {
            args.push(self.parse_type_postfix()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RBracket)?;
        Ok(args)
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expr>, CompileError> {
        self.advance();
        let mut args = Vec::new();
        while self.peek() != &Token::RParen {
            args.push(self.parse_expression()?);
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }
}
