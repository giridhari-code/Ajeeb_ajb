pub mod types;
pub mod expr;
pub mod stmt;
pub mod decls;
pub mod generics;
pub mod patterns;

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
    generic_type_bounds: HashMap<String, Vec<String>>,
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
            generic_type_bounds: HashMap::new(),
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
            generic_type_bounds: HashMap::new(),
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
            | Expr::AssociatedFnCall { line, col, .. }
             | Expr::Match { line, col, .. }
             | Expr::GenericCall { line, col, .. }
             | Expr::Lambda { line, col, .. }
             | Expr::ClosureCall { line, col, .. } => (*line, *col),
         }
     }

    fn token_debug(&self, t: &Token) -> &'static str {
        match t {
            Token::Set => "set",
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

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() == &Token::AtImport {
            stmts.push(self.parse_at_import()?);
        }
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
}
