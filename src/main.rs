use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, Read, Write};

// ============================================================
//  TOKENS
// ============================================================
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Let, Const, If, Else, While, Function, Return, True, False,
    Int, String, Bool, Void,
    Identifier(String), Number(i64), StringLiteral(String),
    Plus, Minus, Star, Slash, Assign,
    Eq, Neq, Lt, Gt, Le, Ge,
    Semicolon, Colon, Comma, Arrow, Dot,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Class, SelfKwd,
    Eof,
}

// ============================================================
//  LEXER (character-by-character)
// ============================================================
struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(source: &str) -> Self {
        Lexer { chars: source.chars().collect(), pos: 0, line: 1, col: 1 }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' { self.line += 1; self.col = 1; } else { self.col += 1; }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.advance();
            } else { break; }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some('/') {
            self.advance();
            if self.peek() == Some('/') {
                while let Some(c) = self.advance() {
                    if c == '\n' { break; }
                }
            } else if self.peek() == Some('*') {
                self.advance();
                while let Some(c) = self.advance() {
                    if c == '*' && self.peek() == Some('/') { self.advance(); break; }
                }
            }
        }
    }

    fn read_string(&mut self) -> Result<Token, CompileError> {
        let start_line = self.line;
        let start_col = self.col;
        // opening " already consumed by next_token()
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(CompileError::new(start_line, start_col, "String khatam nahi hui! Closing quote (\" ) chahiye.".to_string())),
                Some('"') => break,
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('"') => s.push('"'),
                        Some('\\') => s.push('\\'),
                        _ => return Err(CompileError::new(self.line, self.col, "Galat escape sequence. Sirf \\n, \\t, \\\", \\\\ allowed hain.".to_string())),
                    }
                }
                Some(c) => s.push(c),
            }
        }
        Ok(Token::StringLiteral(s))
    }

    fn read_number(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() { s.push(c); self.advance(); } else { break; }
        }
        Token::Number(s.parse().unwrap())
    }

    fn read_identifier(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' { s.push(c); self.advance(); } else { break; }
        }
        match s.as_str() {
            "let" => Token::Let, "const" => Token::Const,
            "if" => Token::If, "else" => Token::Else,
            "while" => Token::While, "function" => Token::Function,
            "return" => Token::Return,
            "true" => Token::True, "false" => Token::False,
            "int" => Token::Int, "string" => Token::String,
            "bool" => Token::Bool, "void" => Token::Void,
            "class" => Token::Class, "self" => Token::SelfKwd,
            _ => Token::Identifier(s),
        }
    }

    fn next_token(&mut self) -> Result<Token, CompileError> {
        loop {
            self.skip_whitespace();
            // handle comments
            if self.peek() == Some('/') {
                let saved = self.pos;
                self.advance();
                if self.peek() == Some('/') || self.peek() == Some('*') {
                    self.pos = saved;
                    self.skip_comment();
                    continue;
                }
                self.pos = saved;
            }
            break;
        }

        let start_line = self.line;
        let start_col = self.col;

        match self.advance() {
            None => Ok(Token::Eof),
            Some(c) => match c {
                '+' => Ok(Token::Plus),
                '-' => {
                    if self.peek() == Some('>') { self.advance(); Ok(Token::Arrow) }
                    else { Ok(Token::Minus) }
                }
                '*' => Ok(Token::Star),
                '/' => Ok(Token::Slash),
                '=' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::Eq) }
                    else { Ok(Token::Assign) }
                }
                '!' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::Neq) }
                    else { Err(CompileError::new(start_line, start_col, "Akela '!' kaam nahi karega. '!=' ya '==' use karo.".to_string())) }
                }
                '<' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::Le) }
                    else { Ok(Token::Lt) }
                }
                '>' => {
                    if self.peek() == Some('=') { self.advance(); Ok(Token::Ge) }
                    else { Ok(Token::Gt) }
                }
                ';' => Ok(Token::Semicolon),
                ':' => Ok(Token::Colon),
                ',' => Ok(Token::Comma),
                '(' => Ok(Token::LParen),
                ')' => Ok(Token::RParen),
                '{' => Ok(Token::LBrace),
                '}' => Ok(Token::RBrace),
                '[' => Ok(Token::LBracket),
                ']' => Ok(Token::RBracket),
                '.' => Ok(Token::Dot),
                '"' => self.read_string(),
                c if c.is_ascii_digit() => Ok(self.read_number(c)),
                c if c.is_alphabetic() || c == '_' => Ok(self.read_identifier(c)),
                _ => Err(CompileError::new(start_line, start_col, format!("Unexpected character '{}'. Yeh kya hai bhai?", c))),
            }
        }
    }
}

// ============================================================
//  ERROR REPORTING (Hindi style)
// ============================================================
#[derive(Debug, Clone)]
struct CompileError {
    line: usize,
    col: usize,
    message: String,
}

impl CompileError {
    fn new(line: usize, col: usize, message: String) -> Self {
        CompileError { line, col, message }
    }
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "❌ Line {}, Col {}: {}", self.line, self.col, self.message)
    }
}

// ============================================================
//  AST NODES
// ============================================================
#[derive(Debug, Clone)]
pub enum TypeAnnot {
    Int, String, Bool, Void,
    Array(Box<TypeAnnot>),
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: String,
    pub type_ann: TypeAnnot,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub fields: Vec<ClassField>,
    pub methods: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, type_ann: Option<TypeAnnot>, value: Expr },
    Const { name: String, type_ann: Option<TypeAnnot>, value: Expr },
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Expr, body: Vec<Stmt> },
    Return { value: Option<Expr> },
    Expr(Expr),
    FnDef { name: String, params: Vec<(String, TypeAnnot)>, return_type: TypeAnnot, body: Vec<Stmt> },
    Class { name: String, fields: Vec<ClassField>, methods: Vec<Stmt> },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    StringLit(String),
    Bool(bool),
    Ident(String),
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Assign { name: String, value: Box<Expr> },
    IndexAssign { obj: Box<Expr>, index: Box<Expr>, value: Box<Expr> },
    FnCall { name: String, args: Vec<Expr> },
    ArrayLit(Vec<Expr>),
    Index { obj: Box<Expr>, index: Box<Expr> },
    Field { obj: Box<Expr>, field: String },
    Group(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, Lt, Gt, Le, Ge,
}

// ============================================================
//  PARSER (recursive descent)
// ============================================================
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let t = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, CompileError> {
        let t = self.advance();
        if std::mem::discriminant(&t) != std::mem::discriminant(expected) {
            return Err(CompileError::new(0, 0, format!("'{}' expected but kuch aur mila. Check karo.", self.token_debug(expected))));
        }
        // Special handling for tokens with data
        match expected {
            Token::Identifier(_) => {
                if let Token::Identifier(_) = &t { Ok(t) } else { Err(CompileError::new(0, 0, "Identifier expected tha.".to_string())) }
            }
            _ => Ok(t),
        }
    }

    fn token_debug(&self, t: &Token) -> &'static str {
        match t {
            Token::Let => "let", Token::Const => "const", Token::If => "if",
            Token::Else => "else", Token::While => "while", Token::Function => "function",
            Token::Return => "return", Token::Int => "int", Token::String => "string",
            Token::Bool => "bool", Token::Void => "void",
            Token::Semicolon => ";", Token::Colon => ":", Token::Comma => ",",
            Token::LParen => "(", Token::RParen => ")",
            Token::LBrace => "{", Token::RBrace => "}",
            Token::Assign => "=", Token::Eq => "==", Token::Neq => "!=",
            Token::Lt => "<", Token::Gt => ">", Token::Le => "<=", Token::Ge => ">=",
            Token::Plus => "+", Token::Minus => "-", Token::Star => "*", Token::Slash => "/",
            Token::Arrow => "->",
            Token::Dot => ".",
            Token::LBracket => "[",
            Token::RBracket => "]",
            Token::Class => "class",
            Token::SelfKwd => "self",
            Token::Identifier(_) => "identifier",
            Token::Number(_) => "number",
            Token::StringLiteral(_) => "string literal",
            Token::True | Token::False => "boolean",
            _ => "unknown",
        }
    }

    fn parse_type(&mut self) -> Result<Option<TypeAnnot>, CompileError> {
        if self.peek() == &Token::Colon {
            self.advance();
            let base = match self.advance() {
                Token::Int => TypeAnnot::Int,
                Token::String => TypeAnnot::String,
                Token::Bool => TypeAnnot::Bool,
                Token::Void => TypeAnnot::Void,
                other => return Err(CompileError::new(0, 0, format!("Unknown type {:?}. Sirf int, string, bool, void allowed hain.", other))),
            };
            // Check for array type: int[], string[], etc.
            if self.peek() == &Token::LBracket {
                self.advance();
                self.expect(&Token::RBracket)?;
                Ok(Some(TypeAnnot::Array(Box::new(base))))
            } else {
                Ok(Some(base))
            }
        } else {
            Ok(None)
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::Eof {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        match self.peek() {
            Token::Let => self.parse_let_decl(),
            Token::Const => self.parse_const_decl(),
            Token::If => self.parse_if_stmt(),
            Token::While => self.parse_while_stmt(),
            Token::Function => self.parse_fn_def(),
            Token::Return => self.parse_return_stmt(),
            Token::Class => self.parse_class_def(),
            Token::RBrace => Err(CompileError::new(0, 0, "Extra '}' mil gaya. Kahi closing brace zyada hai.".to_string())),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let_decl(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // let
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(CompileError::new(0, 0, "'let' ke baad variable ka naam chahiye.".to_string())),
        };
        let type_ann = self.parse_type()?;
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Let { name, type_ann, value })
    }

    fn parse_const_decl(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // const
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(CompileError::new(0, 0, "'const' ke baad variable ka naam chahiye.".to_string())),
        };
        let type_ann = self.parse_type()?;
        self.expect(&Token::Assign)?;
        let value = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Const { name, type_ann, value })
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // if
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
                // else if
                let elif = self.parse_if_stmt()?;
                else_block = Some(vec![elif]);
            } else {
                self.expect(&Token::LBrace)?;
                let block = self.parse_block()?;
                self.expect(&Token::RBrace)?;
                else_block = Some(block);
            }
        }
        Ok(Stmt::If { condition, then_block, else_block })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // while
        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::While { condition, body })
    }

    fn parse_fn_def(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(CompileError::new(0, 0, "'function' ke baad function ka naam chahiye.".to_string())),
        };
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(CompileError::new(0, 0, "Function parameter ka naam chahiye.".to_string())),
            };
            let ptype = match self.parse_type()? {
                Some(t) => t,
                None => return Err(CompileError::new(0, 0, "Parameter ka type batana zaroori hai (jaise x: int).".to_string())),
            };
            params.push((pname, ptype));
            if self.peek() == &Token::Comma { self.advance(); }
        }
        self.expect(&Token::RParen)?;
        let return_type = match self.parse_type()? {
            Some(t) => t,
            None => TypeAnnot::Void,
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::FnDef { name, params, return_type, body })
    }

    fn parse_class_def(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // class
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(CompileError::new(0, 0, "'class' ke baad naam chahiye.".to_string())),
        };
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            if self.peek() == &Token::Function {
                methods.push(self.parse_fn_def()?);
            } else {
                // field declaration: name : type ;
                let fname = match self.advance() {
                    Token::Identifier(n) => n,
                    _ => return Err(CompileError::new(0, 0, "Field ka naam chahiye.".to_string())),
                };
                let ftype = match self.parse_type()? {
                    Some(t) => t,
                    None => return Err(CompileError::new(0, 0, "Field ka type batana zaroori hai.".to_string())),
                };
                self.expect(&Token::Semicolon)?;
                fields.push(ClassField { name: fname, type_ann: ftype });
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Stmt::Class { name, fields, methods })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance(); // return
        if self.peek() == &Token::Semicolon {
            self.advance();
            Ok(Stmt::Return { value: None })
        } else {
            let value = self.parse_expression()?;
            self.expect(&Token::Semicolon)?;
            Ok(Stmt::Return { value: Some(value) })
        }
    }

    fn parse_expr_stmt(&mut self) -> Result<Stmt, CompileError> {
        let expr = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }

    // Expression parsing with precedence
    fn parse_expression(&mut self) -> Result<Expr, CompileError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, CompileError> {
        let expr = self.parse_equality()?;
        if self.peek() == &Token::Assign {
            self.advance();
            match expr {
                Expr::Ident(name) => {
                    let value = self.parse_assignment()?;
                    Ok(Expr::Assign { name, value: Box::new(value) })
                }
                _ => Err(CompileError::new(0, 0, "Assignment ka left side variable hona chahiye.".to_string())),
            }
        } else {
            Ok(expr)
        }
    }

    fn parse_equality(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_comparison()?;
        while self.peek() == &Token::Eq || self.peek() == &Token::Neq {
            let op = match self.advance() {
                Token::Eq => BinOp::Eq,
                _ => BinOp::Neq,
            };
            let right = self.parse_comparison()?;
            expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.parse_term()?;
        while self.peek() == &Token::Lt || self.peek() == &Token::Gt
            || self.peek() == &Token::Le || self.peek() == &Token::Ge
        {
            let op = match self.advance() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Le => BinOp::Le,
                _ => BinOp::Ge,
            };
            let right = self.parse_term()?;
            expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
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
            let right = self.parse_factor()?;
            expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
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
            let right = self.parse_unary()?;
            expr = Expr::Binary { left: Box::new(expr), op, right: Box::new(right) };
        }
        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, CompileError> {
        if self.peek() == &Token::Minus {
            self.advance();
            let expr = self.parse_unary()?;
            Ok(Expr::Binary {
                left: Box::new(Expr::Number(0)),
                op: BinOp::Sub,
                right: Box::new(expr),
            })
        } else {
            self.parse_primary()
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let mut expr = match self.peek() {
            Token::Number(n) => { let v = n.clone(); self.advance(); Expr::Number(v) }
            Token::StringLiteral(s) => { let v = s.clone(); self.advance(); Expr::StringLit(v) }
            Token::True => { self.advance(); Expr::Bool(true) }
            Token::False => { self.advance(); Expr::Bool(false) }
            Token::SelfKwd => { self.advance(); Expr::Ident("self".to_string()) }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while self.peek() != &Token::RBracket {
                    elems.push(self.parse_expression()?);
                    if self.peek() == &Token::Comma { self.advance(); }
                }
                self.expect(&Token::RBracket)?;
                Expr::ArrayLit(elems)
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Expr::Group(Box::new(e))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Expr::Ident(name)
            }
            _ => return Err(CompileError::new(0, 0, format!("Unexpected token. Expecting expression, mila {:?}.", self.peek()))),
        };
        // Postfix chain: .field, .method(), [index], (args)
        loop {
            match self.peek() {
                Token::LParen => {
                    // function call: ident(args)
                    let name = match &expr {
                        Expr::Ident(n) => n.clone(),
                        _ => return Err(CompileError::new(0, 0, "Sirf identifier ko call kar sakte ho.".to_string())),
                    };
                    self.advance(); // (
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen {
                        args.push(self.parse_expression()?);
                        if self.peek() == &Token::Comma { self.advance(); }
                    }
                    self.expect(&Token::RParen)?;
                    expr = Expr::FnCall { name, args };
                }
                Token::Dot => {
                    self.advance();
                    let field = match self.advance() {
                        Token::Identifier(n) => n,
                        _ => return Err(CompileError::new(0, 0, "'.' ke baad field/method ka naam chahiye.".to_string())),
                    };
                    if self.peek() == &Token::LParen {
                        // method call: obj.method(args)
                        self.advance();
                        let mut args = vec![expr.clone()]; // self is first arg
                        while self.peek() != &Token::RParen {
                            args.push(self.parse_expression()?);
                            if self.peek() == &Token::Comma { self.advance(); }
                        }
                        self.expect(&Token::RParen)?;
                        expr = Expr::FnCall { name: field, args };
                    } else {
                        expr = Expr::Field { obj: Box::new(expr), field };
                    }
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket)?;
                    if self.peek() == &Token::Assign {
                        self.advance();
                        let value = self.parse_expression()?;
                        expr = Expr::IndexAssign { obj: Box::new(expr), index: Box::new(index), value: Box::new(value) };
                    } else {
                        expr = Expr::Index { obj: Box::new(expr), index: Box::new(index) };
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }
}

// ============================================================
//  ASSEMBLY GENERATOR  (aarch64 / ARM64)
// ============================================================
struct AsmGen {
    asm: String,
    data: String,
    label_counter: usize,
    var_map: HashMap<String, i32>,
    fn_map: HashMap<String, FnInfo>,
    class_map: HashMap<String, ClassLayout>,
    _field_access_tmp: HashMap<String, String>,
    current_offset: i32,
    class_field_scope: Option<String>, // when emitting class method, which class
}

struct FnInfo {
    label: String,
}

#[allow(dead_code)]
struct ClassLayout {
    _fields: Vec<String>,
    field_offsets: HashMap<String, i32>,
    _size: i32,
}

impl AsmGen {
    fn new() -> Self {
        AsmGen {
            asm: String::new(),
            data: String::new(),
            label_counter: 0,
            var_map: HashMap::new(),
            fn_map: HashMap::new(),
            class_map: HashMap::new(),
            _field_access_tmp: HashMap::new(),
            current_offset: 0,
            class_field_scope: None,
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!(".L{}_{}", prefix, id)
    }

    fn get_var_offset(&self, name: &str) -> i32 {
        *self.var_map.get(name).unwrap_or_else(|| panic!("Variable '{}' declare nahi hui!", name))
    }

    fn alloc_var(&mut self, name: &str) -> i32 {
        self.current_offset -= 8;
        self.var_map.insert(name.to_string(), self.current_offset);
        self.current_offset
    }

    fn alloc_var_sized(&mut self, name: &str, slots: usize) -> i32 {
        for _ in 0..slots { self.current_offset -= 8; }
        let base = self.current_offset + 8; // first slot
        self.var_map.insert(name.to_string(), self.current_offset + 8 * slots as i32);
        base
    }

    fn generate(&mut self, stmts: &[Stmt]) -> Result<String, CompileError> {
        // First pass: collect class info and function definitions
        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, .. } => {
                    let label = format!("fn_{}", name);
                    self.fn_map.insert(name.clone(), FnInfo { label });
                }
                Stmt::Class { name, fields, methods } => {
                    let mut offsets = HashMap::new();
                    let mut fnames = Vec::new();
                    let mut off: i32 = 0;
                    for f in fields {
                        offsets.insert(f.name.clone(), off);
                        fnames.push(f.name.clone());
                        off += 8;
                    }
                    self.class_map.insert(name.clone(), ClassLayout {
                        _fields: fnames,
                        field_offsets: offsets,
                        _size: off,
                    });
                    // Register methods as functions with class prefix
                    for m in methods {
                        if let Stmt::FnDef { name: mname, .. } = m {
                            let label = format!("fn_{}_{}", name, mname);
                            self.fn_map.insert(format!("{}_{}", name, mname), FnInfo { label });
                        }
                    }
                }
                _ => {}
            }
        }

        self.asm.push_str(".global _start\n");
        self.asm.push_str(".text\n");

        let has_main = stmts.iter().any(|s| matches!(s, Stmt::FnDef { name, .. } if name == "main"));

        if has_main {
            self.asm.push_str("_start:\n");
            self.asm.push_str("    bl fn_main\n");
            self.asm.push_str("    mov x8, #93\n");
            self.asm.push_str("    svc #0\n");
        } else {
            self.asm.push_str("_start:\n");
            self.asm.push_str("    mov x29, sp\n");
        }

        for stmt in stmts {
            self.emit_stmt(stmt)?;
        }

        if !has_main {
            self.asm.push_str("    mov x0, #0\n");
            self.asm.push_str("    mov x8, #93\n");
            self.asm.push_str("    svc #0\n");
        }

        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, body, .. } => {
                    self.emit_fn_def(&name, &params, &body)?;
                }
                Stmt::Class { name, methods, .. } => {
                    // Emit methods as ClassName_methodName
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, body, return_type: _ } = m {
                            let mut all_params = vec![("self".to_string(), TypeAnnot::Int)]; // self pointer
                            all_params.extend(params.iter().cloned());
                            self.class_field_scope = Some(name.clone());
                            // Register params including self
                            let mangled_name = format!("{}_{}", name, mname);
                            self.emit_fn_def(&mangled_name, &all_params, &body)?;
                            self.class_field_scope = None;
                        }
                    }
                }
                _ => {}
            }
        }

        self.emit_data_section();
        let bss = ".section .bss\n.align 4\n__ajeeb_buf: .space 4096\n";
        Ok(format!("{}\n{}{}", self.asm, self.data, bss))
    }

    fn emit_data_section(&mut self) {
        if !self.data.is_empty() {
            self.data = format!(".section .rodata\n{}", self.data);
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let { name, value, .. } | Stmt::Const { name, value, .. } => {
                match value {
                    Expr::ArrayLit(elems) => {
                        // Allocate pointer slot + element slots
                        let base = self.alloc_var_sized(name, elems.len() + 1);
                        let ptr_slot = self.get_var_offset(name); // where pointer is stored
                        for (i, elem) in elems.iter().enumerate() {
                            let elem_addr = base + (i as i32 * 8);
                            self.emit_expr(elem)?;
                            self.asm.push_str(&format!("    str x0, [x29, {}]\n", elem_addr));
                        }
                        // Store pointer to first element
                        self.asm.push_str(&format!("    add x0, x29, #{}\n", base));
                        self.asm.push_str(&format!("    str x0, [x29, {}]\n", ptr_slot));
                    }
                    _ => {
                        self.alloc_var(name);
                        self.emit_expr(value)?;
                        let offset = self.get_var_offset(name);
                        self.asm.push_str(&format!("    str x0, [x29, {}]\n", offset));
                    }
                }
                Ok(())
            }
            Stmt::If { condition, then_block, else_block } => {
                let else_label = self.fresh_label("else");
                let end_label = self.fresh_label("endif");
                self.emit_condition(condition, &else_label)?;
                for s in then_block { self.emit_stmt(s)?; }
                self.asm.push_str(&format!("    b {}\n", end_label));
                self.asm.push_str(&format!("{}:\n", else_label));
                if let Some(eblock) = else_block {
                    for s in eblock { self.emit_stmt(s)?; }
                }
                self.asm.push_str(&format!("{}:\n", end_label));
                Ok(())
            }
            Stmt::While { condition, body } => {
                let begin_label = self.fresh_label("while_begin");
                let end_label = self.fresh_label("while_end");
                self.asm.push_str(&format!("{}:\n", begin_label));
                self.emit_expr(condition)?;
                self.asm.push_str("    cmp x0, #0\n");
                self.asm.push_str(&format!("    b.eq {}\n", end_label));
                for s in body { self.emit_stmt(s)?; }
                self.asm.push_str(&format!("    b {}\n", begin_label));
                self.asm.push_str(&format!("{}:\n", end_label));
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(expr) = value {
                    self.emit_expr(expr)?;
                }
                self.emit_fn_epilogue();
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::FnDef { .. } => Ok(()),
            Stmt::Class { .. } => Ok(()), // methods emitted separately
        }
    }

    fn emit_condition(&mut self, condition: &Expr, false_label: &str) -> Result<(), CompileError> {
        self.emit_expr(condition)?;
        self.asm.push_str("    cmp x0, #0\n");
        self.asm.push_str(&format!("    b.eq {}\n", false_label));
        Ok(())
    }

    fn emit_print(&mut self, expr: &Expr) -> Result<(), CompileError> {
        // print(expr) — write syscall: x0=1(stdout), x1=string, x2=len, x8=64, svc
        // For string literals, we know the length at compile time
        if let Expr::StringLit(s) = expr {
            let lbl = self.fresh_label("str");
            let len = s.len();
            self.data.push_str(&format!("{}: .asciz \"", lbl));
            for c in s.chars() {
                match c {
                    '\n' => self.data.push_str("\\n"),
                    '\t' => self.data.push_str("\\t"),
                    '\\' => self.data.push_str("\\\\"),
                    '"' => self.data.push_str("\"\""),
                    _ => self.data.push(c),
                }
            }
            self.data.push_str("\"\n");
            self.asm.push_str("    mov x0, #1\n");
            self.asm.push_str(&format!("    adrp x1, {}\n", lbl));
            self.asm.push_str(&format!("    add x1, x1, :lo12:{}\n", lbl));
            self.asm.push_str(&format!("    mov x2, #{}\n", len));
            self.asm.push_str("    mov x8, #64\n");
            self.asm.push_str("    svc #0\n");
        } else {
            // For non-string, just emit as-is (will be a pointer)
            self.emit_expr(expr)?;
            self.asm.push_str("    mov x1, x0\n");
            self.asm.push_str("    mov x0, #1\n");
            self.asm.push_str("    mov x2, #8\n"); // print 8 bytes as fallback
            self.asm.push_str("    mov x8, #64\n");
            self.asm.push_str("    svc #0\n");
        }
        Ok(())
    }

    fn emit_println(&mut self, expr: &Expr) -> Result<(), CompileError> {
        self.emit_print(expr)?;
        let nl_lbl = self.fresh_label("nl");
        self.data.push_str(&format!("{}: .asciz \"\\n\"\n", nl_lbl));
        self.asm.push_str("    mov x0, #1\n");
        self.asm.push_str(&format!("    adrp x1, {}\n", nl_lbl));
        self.asm.push_str(&format!("    add x1, x1, :lo12:{}\n", nl_lbl));
        self.asm.push_str("    mov x2, #1\n");
        self.asm.push_str("    mov x8, #64\n");
        self.asm.push_str("    svc #0\n");
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr {
            Expr::Number(n) => {
                self.asm.push_str(&format!("    mov x0, #{}\n", n));
            }
            Expr::StringLit(s) => {
                let lbl = self.fresh_label("str");
                self.data.push_str(&format!("{}: .asciz \"", lbl));
                for c in s.chars() {
                    match c {
                        '\n' => self.data.push_str("\\n"),
                        '\t' => self.data.push_str("\\t"),
                        '\\' => self.data.push_str("\\\\"),
                        '"' => self.data.push_str("\"\""),
                        _ => self.data.push(c),
                    }
                }
                self.data.push_str("\"\n");
                self.asm.push_str(&format!("    adrp x0, {}\n", lbl));
                self.asm.push_str(&format!("    add x0, x0, :lo12:{}\n", lbl));
            }
            Expr::Bool(b) => {
                self.asm.push_str(&format!("    mov x0, #{}\n", if *b { 1 } else { 0 }));
            }
            Expr::Ident(name) => {
                if name == "self" && self.class_field_scope.is_some() {
                    // self pointer — load it from the first parameter slot
                    let offset = self.get_var_offset(name);
                    self.asm.push_str(&format!("    ldr x0, [x29, {}]\n", offset));
                } else {
                    let offset = self.get_var_offset(name);
                    self.asm.push_str(&format!("    ldr x0, [x29, {}]\n", offset));
                }
            }
            Expr::Binary { left, op, right } => {
                self.emit_expr(left)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n");
                self.emit_expr(right)?;
                self.asm.push_str("    mov x1, x0\n");
                self.asm.push_str("    ldr x0, [sp], #16\n");
                match op {
                    BinOp::Add => self.asm.push_str("    add x0, x0, x1\n"),
                    BinOp::Sub => self.asm.push_str("    sub x0, x0, x1\n"),
                    BinOp::Mul => self.asm.push_str("    mul x0, x0, x1\n"),
                    BinOp::Div => self.asm.push_str("    sdiv x0, x0, x1\n"),
                    BinOp::Eq => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, eq\n");
                    }
                    BinOp::Neq => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, ne\n");
                    }
                    BinOp::Lt => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, lt\n");
                    }
                    BinOp::Gt => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, gt\n");
                    }
                    BinOp::Le => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, le\n");
                    }
                    BinOp::Ge => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, ge\n");
                    }
                }
            }
            Expr::Assign { name, value } => {
                self.emit_expr(value)?;
                let offset = self.get_var_offset(name);
                self.asm.push_str(&format!("    str x0, [x29, {}]\n", offset));
            }
            Expr::IndexAssign { obj, index, value } => {
                self.emit_expr(obj)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n"); // push base
                self.emit_expr(index)?;
                self.asm.push_str("    mov x1, x0\n");  // index
                self.asm.push_str("    ldr x0, [sp], #16\n"); // pop base
                self.asm.push_str("    add x0, x0, x1, lsl #3\n"); // base + index*8
                self.asm.push_str("    str x0, [sp, #-16]!\n"); // push addr
                self.emit_expr(value)?;
                self.asm.push_str("    ldr x1, [sp], #16\n"); // pop addr
                self.asm.push_str("    str x0, [x1]\n");
            }
            Expr::FnCall { name, args } => {
                // Handle built-in print/println
                if name == "print" {
                    if args.len() == 1 {
                        return self.emit_print(&args[0]);
                    }
                }
                if name == "println" {
                    if args.len() == 1 {
                        return self.emit_println(&args[0]);
                    }
                }
                // Built-in: len(str)
                if name == "len" && args.len() == 1 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x1, x0\n");
                    self.asm.push_str("    mov x0, #0\n");
                    let sl_lbl = self.fresh_label("strlen_loop");
                    self.asm.push_str(&format!("{}:\n", sl_lbl));
                    let selbl = self.fresh_label("strlen_end");
                    self.asm.push_str("    ldrb w2, [x1, x0]\n");
                    self.asm.push_str(&format!("    cbz w2, {}\n", selbl));
                    self.asm.push_str("    add x0, x0, #1\n");
                    self.asm.push_str(&format!("    b {}\n", sl_lbl));
                    self.asm.push_str(&format!("{}:\n", selbl));
                    return Ok(());
                }
                // Built-in: charCode(str, index)
                if name == "charCode" && args.len() == 2 {
                    self.emit_expr(&args[1])?;  // index
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                    self.emit_expr(&args[0])?;  // str
                    self.asm.push_str("    ldr x1, [sp], #16\n"); // index in x1
                    self.asm.push_str("    add x0, x0, x1\n"); // str + index
                    self.asm.push_str("    ldrb w0, [x0]\n");
                    return Ok(());
                }
                // Built-in: readFile(path)
                if name == "readFile" && args.len() == 1 {
                    self.emit_expr(&args[0])?; // path in x0
                    // openat(AT_FDCWD=-100, path, O_RDONLY=0)
                    self.asm.push_str("    mov x1, x0\n"); // path
                    self.asm.push_str("    mov x0, #-100\n"); // dirfd = AT_FDCWD
                    self.asm.push_str("    mov x2, #0\n"); // O_RDONLY
                    self.asm.push_str("    mov x8, #56\n"); // openat
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x19, x0\n"); // save fd
                    // read(fd, buf, 4096)
                    self.asm.push_str("    adrp x1, __ajeeb_buf\n");
                    self.asm.push_str("    add x1, x1, :lo12:__ajeeb_buf\n");
                    self.asm.push_str("    mov x2, #4096\n");
                    self.asm.push_str("    mov x0, x19\n"); // fd
                    self.asm.push_str("    mov x8, #63\n"); // read
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x20, x0\n"); // save bytes read
                    // null-terminate
                    self.asm.push_str("    adrp x1, __ajeeb_buf\n");
                    self.asm.push_str("    add x1, x1, :lo12:__ajeeb_buf\n");
                    self.asm.push_str("    strb wzr, [x1, x20]\n");
                    // close(fd)
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x8, #57\n"); // close
                    self.asm.push_str("    svc #0\n");
                    // Return buffer address
                    self.asm.push_str("    adrp x0, __ajeeb_buf\n");
                    self.asm.push_str("    add x0, x0, :lo12:__ajeeb_buf\n");
                    return Ok(());
                }
                // Built-in: writeFile(path, content)
                if name == "writeFile" && args.len() == 2 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x19, x0\n"); // save path
                    self.emit_expr(&args[1])?;
                    self.asm.push_str("    mov x20, x0\n"); // save content
                    // strlen
                    self.asm.push_str("    mov x2, #0\n");
                    let wlbl = self.fresh_label("wstrlen");
                    let welbl = self.fresh_label("wstrlen_end");
                    self.asm.push_str(&format!("{}:\n", wlbl));
                    self.asm.push_str("    ldrb w3, [x20, x2]\n");
                    self.asm.push_str(&format!("    cbz w3, {}\n", welbl));
                    self.asm.push_str("    add x2, x2, #1\n");
                    self.asm.push_str(&format!("    b {}\n", wlbl));
                    self.asm.push_str(&format!("{}:\n", welbl));
                    self.asm.push_str("    mov x21, x2\n"); // save len
                    // openat(AT_FDCWD=-100, path, O_WRONLY|O_CREAT|O_TRUNC=577, 0644)
                    self.asm.push_str("    mov x1, x19\n"); // path
                    self.asm.push_str("    mov x0, #-100\n"); // dirfd = AT_FDCWD
                    self.asm.push_str("    mov x2, #577\n"); // flags
                    self.asm.push_str("    mov x3, #420\n"); // mode 0644
                    self.asm.push_str("    mov x8, #56\n"); // openat
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x19, x0\n"); // save fd
                    // write(fd, content, len)
                    self.asm.push_str("    mov x0, x19\n"); // fd
                    self.asm.push_str("    mov x1, x20\n"); // content
                    self.asm.push_str("    mov x2, x21\n"); // len
                    self.asm.push_str("    mov x8, #64\n");
                    self.asm.push_str("    svc #0\n");
                    // close(fd)
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x8, #57\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x0, #0\n");
                    return Ok(());
                }
                let label = {
                    let info = self.fn_map.get(name)
                        .ok_or_else(|| CompileError::new(0, 0, format!("Function '{}' define nahi hui!", name)))?;
                    info.label.clone()
                };
                if args.len() > 8 {
                    return Err(CompileError::new(0, 0, "Zyaada arguments! Sirf 8 arguments allowed hain.".to_string()));
                }
                for arg in args.iter() {
                    self.emit_expr(arg)?;
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                }
                let arg_regs = ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"];
                for (i, _) in args.iter().enumerate().rev() {
                    self.asm.push_str(&format!("    ldr {}, [sp], #16\n", arg_regs[i]));
                }
                self.asm.push_str(&format!("    bl {}\n", label));
            }
            Expr::ArrayLit(elems) => {
                // Result is pointer to first element on stack
                self.asm.push_str("    mov x0, sp\n");
                for elem in elems.iter() {
                    self.emit_expr(elem)?;
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                }
                // x0 points to where first element was stored... but we modified sp
                // Instead, just allocate a slot and return pointer
                self.asm.push_str("    mov x0, sp\n");
            }
            Expr::Index { obj, index } => {
                self.emit_expr(obj)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n"); // push base
                self.emit_expr(index)?;
                self.asm.push_str("    mov x1, x0\n");  // index
                self.asm.push_str("    ldr x0, [sp], #16\n"); // pop base
                self.asm.push_str("    add x0, x0, x1, lsl #3\n"); // base + index*8
                self.asm.push_str("    ldr x0, [x0]\n"); // load value
            }
            Expr::Field { obj, field } => {
                let class_name = self.class_field_scope.as_ref().cloned();
                if let Some(ref cn) = class_name {
                    let offset_val = self.class_map.get(cn)
                        .and_then(|layout| layout.field_offsets.get(field))
                        .cloned();
                    if let Some(off) = offset_val {
                        self.emit_expr(obj)?;
                        self.asm.push_str(&format!("    ldr x0, [x0, #{}]\n", off));
                    } else {
                        return Err(CompileError::new(0, 0, format!("Field '{}' class '{}' me exist nahi karta!", field, cn)));
                    }
                } else {
                    return Err(CompileError::new(0, 0, "Field access sirf class methods me kaam karta hai.".to_string()));
                }
            }
            Expr::Group(inner) => self.emit_expr(inner)?,
        }
        Ok(())
    }

    fn emit_fn_def(&mut self, name: &str, params: &[(String, TypeAnnot)], body: &[Stmt]) -> Result<(), CompileError> {
        let info = self.fn_map.get(name).unwrap();
        self.asm.push_str(&format!("{}:\n", info.label));
        self.asm.push_str("    stp x29, x30, [sp, #-16]!\n");
        self.asm.push_str("    stp x19, x20, [sp, #-16]!\n");
        self.asm.push_str("    str x21, [sp, #-16]!\n");
        self.asm.push_str("    mov x29, sp\n");

        let old_var_map = self.var_map.clone();
        let old_offset = self.current_offset;
        self.var_map.clear();
        self.current_offset = 0;

        let param_regs = ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"];
        for (i, (pname, _)) in params.iter().enumerate() {
            let offset = self.alloc_var(pname);
            self.asm.push_str(&format!("    str {}, [x29, {}]\n", param_regs[i], offset));
        }

        let mut local_var_count = 0;
        for stmt in body {
            self.count_local_vars(stmt, &mut local_var_count);
        }
        let total_stack = params.len() + local_var_count;
        if total_stack > 0 {
            // align to 16 bytes for aarch64 stack alignment requirement
            let size = ((total_stack * 8) + 15) & !15;
            self.asm.push_str(&format!("    sub sp, sp, #{}\n", size));
        }

        for stmt in body {
            self.emit_stmt(stmt)?;
        }

        let has_return = body.iter().any(|s| matches!(s, Stmt::Return { .. }));
        if !has_return {
            self.emit_fn_epilogue();
        }

        self.var_map = old_var_map;
        self.current_offset = old_offset;

        Ok(())
    }

    fn count_local_vars(&mut self, stmt: &Stmt, count: &mut usize) {
        match stmt {
            Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
                if let Expr::ArrayLit(elems) = value {
                    *count += elems.len() + 1; // pointer + elements
                } else {
                    *count += 1;
                }
            }
            Stmt::If { then_block, else_block, .. } => {
                for s in then_block { self.count_local_vars(s, count); }
                if let Some(eb) = else_block {
                    for s in eb { self.count_local_vars(s, count); }
                }
            }
            Stmt::While { body, .. } => {
                for s in body { self.count_local_vars(s, count); }
            }
            _ => {}
        }
    }

    fn emit_fn_epilogue(&mut self) {
        self.asm.push_str("    mov sp, x29\n");
        self.asm.push_str("    ldr x21, [sp], #16\n");
        self.asm.push_str("    ldp x19, x20, [sp], #16\n");
        self.asm.push_str("    ldp x29, x30, [sp], #16\n");
        self.asm.push_str("    ret\n");
    }
}

// ============================================================
//  MAIN
// ============================================================
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Arre Bhai! File ka naam toh do. Example: cargo run test.ajb");
        return Ok(());
    }

    let file_path = &args[1];
    let mut file = File::open(file_path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // 1. LEX
    let mut lexer = Lexer::new(&contents);
    let mut tokens = Vec::new();
    loop {
        match lexer.next_token() {
            Ok(Token::Eof) => break,
            Ok(tok) => tokens.push(tok),
            Err(e) => {
                println!("{}\n😡 Lexing error! Tokenize karte waqt problem aayi.", e);
                return Ok(());
            }
        }
    }

    println!("✓ Lexer: {} tokens mil gaye", tokens.len());

    // 2. PARSE
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse_program() {
        Ok(stmts) => stmts,
        Err(e) => {
            println!("{}\n😤 Parsing error! AST banane me problem aayi.", e);
            return Ok(());
        }
    };

    println!("✓ Parser: {} statements parse ho gaye", ast.len());

    // 3. ASSEMBLY GENERATION
    let mut asm_gen = AsmGen::new();
    match asm_gen.generate(&ast) {
        Ok(asm_code) => {
            println!("\n--- ⚡ Generated Assembly (aarch64) ---");
            println!("{}", asm_code);
            println!("---------------------------------------");

            let mut asm_file = File::create("output.asm")?;
            asm_file.write_all(asm_code.as_bytes())?;
            println!("🎉 Sukriya! 'output.asm' file create ho chuki hai.");
            println!("📝 Assemble karne ke liye: as output.asm -o output.o && ld output.o -o output");
        }
        Err(e) => {
            println!("{}\n🔥 Code generation error! Assembly nahi ban paayi.", e);
        }
    }

    Ok(())
}
