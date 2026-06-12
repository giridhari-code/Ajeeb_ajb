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
            | Expr::StringLit(_, l, c)
            | Expr::Bool(_, l, c)
            | Expr::Ident(_, l, c)
            | Expr::ArrayLit(_, l, c)
            | Expr::UnaryNot(_, l, c)
            | Expr::Group(_, l, c) => (*l, *c),
            Expr::Binary { line, col, .. }
            | Expr::Assign { line, col, .. }
            | Expr::IndexAssign { line, col, .. }
            | Expr::FnCall { line, col, .. }
            | Expr::New { line, col, .. }
            | Expr::Index { line, col, .. }
            | Expr::Field { line, col, .. }
            | Expr::FieldAssign { line, col, .. } => (*line, *col),
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
            Token::String => "string",
            Token::Bool => "bool",
            Token::Void => "void",
            Token::Semicolon => ";",
            Token::Colon => ":",
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
            Token::And => "&&",
            Token::Or => "||",
            Token::Not => "!",
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
                Token::Identifier(name) => TypeAnnot::Class(name),
                other => {
                    return Err(self.err(format!(
                    "Unknown type {:?}. Sirf int, string, bool, void aur class names allowed hain.",
                    other
                )))
                }
            };
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

    pub fn parse_program(&mut self) -> Result<Vec<Stmt>, CompileError> {
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
            Token::For => self.parse_for_stmt(),
            Token::Function => self.parse_fn_def(),
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
            Token::Class => self.parse_class_def(),
            Token::RBrace => Err(self.err("Extra '}' mil gaya. Kahi closing brace zyada hai.")),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_let_decl(&mut self) -> Result<Stmt, CompileError> {
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
        Ok(Stmt::Let { name, type_ann, value, line, col })
    }

    fn parse_const_decl(&mut self) -> Result<Stmt, CompileError> {
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
        Ok(Stmt::Const { name, type_ann, value, line, col })
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
        // init
        let init = if self.peek() == &Token::Let {
            self.parse_let_decl()?
        } else if self.peek() == &Token::Semicolon {
            self.advance();
            Stmt::Expr(Expr::Number(0, 0, 0), 0, 0)
        } else {
            self.parse_expr_stmt()?
        };
        // condition
        let condition = if self.peek() == &Token::Semicolon {
            Expr::Number(1, 0, 0)
        } else {
            let e = self.parse_expression()?;
            self.expect(&Token::Semicolon)?;
            e
        };
        // update
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

    fn parse_fn_def(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        let name = match self.advance() {
            Token::Identifier(n) => n,
            _ => return Err(self.err("'function' ke baad function ka naam chahiye.")),
        };
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Function parameter ka naam chahiye.")),
            };
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
        Ok(Stmt::FnDef {
            name,
            params,
            return_type,
            body,
            line,
            col,
        })
    }

    fn parse_class_def(&mut self) -> Result<Stmt, CompileError> {
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
            if self.peek() == &Token::Function {
                methods.push(self.parse_fn_def()?);
            } else {
                let fname = match self.advance() {
                    Token::Identifier(n) => n,
                    _ => return Err(self.err("Field ka naam chahiye.")),
                };
                if self.parse_type()?.is_none() {
                    return Err(self.err("Field ka type batana zaroori hai."));
                }
                self.expect(&Token::Semicolon)?;
                fields.push(ClassField { name: fname });
            }
        }
        self.current_class = old_class;
        self.expect(&Token::RBrace)?;
        Ok(Stmt::Class {
            name,
            fields,
            methods,
            line,
            col,
        })
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
        self.parse_assignment()
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
                    Err(self.err("Assignment ka left side variable, field, ya index hona chahiye."))
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
            Ok(Expr::Binary {
                left: Box::new(Expr::Number(0, line, col)),
                op: BinOp::Sub,
                right: Box::new(expr),
                line,
                col,
            })
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

    fn parse_primary(&mut self) -> Result<Expr, CompileError> {
        let mut expr = match self.peek() {
            Token::Number(n) => {
                let v = *n;
                self.advance();
                let line = self.line();
                let col = self.col();
                Expr::Number(v, line, col)
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
                Expr::Ident(name, line, col)
            }
            _ => {
                return Err(self.err(format!(
                    "Unexpected token. Expecting expression, mila {:?}.",
                    self.peek()
                )))
            }
        };
        loop {
            match self.peek() {
                Token::LParen => {
                    let (line, col) = Self::expr_pos(&expr);
                    let name = match &expr {
                        Expr::Ident(n, ..) => n.clone(),
                        _ => return Err(self.err("Sirf identifier ko call kar sakte ho.")),
                    };
                    self.advance();
                    let mut args = Vec::new();
                    while self.peek() != &Token::RParen {
                        args.push(self.parse_expression()?);
                        if self.peek() == &Token::Comma {
                            self.advance();
                        }
                    }
                    self.expect(&Token::RParen)?;
                    expr = Expr::FnCall { name, args, line, col };
                }
                Token::Dot => {
                    let (line, col) = Self::expr_pos(&expr);
                    self.advance();
                    let class_name = match &expr {
                        Expr::Ident(n, ..) => {
                            if n == "self" {
                                self.current_class.clone()
                            } else {
                                self.var_types.get(n).and_then(|t| {
                                    if let TypeAnnot::Class(cn) = t {
                                        Some(cn.clone())
                                    } else {
                                        None
                                    }
                                })
                            }
                        }
                        _ => None,
                    };
                    let field = match self.advance() {
                        Token::Identifier(n) => n,
                        _ => return Err(self.err("'.' ke baad field/method ka naam chahiye.")),
                    };
                    if self.peek() == &Token::LParen {
                        self.advance();
                        let mut args = vec![expr.clone()];
                        while self.peek() != &Token::RParen {
                            args.push(self.parse_expression()?);
                            if self.peek() == &Token::Comma {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RParen)?;
                        let name = if let Some(ref cn) = class_name {
                            format!("{}_{}", cn, field)
                        } else {
                            field
                        };
                        expr = Expr::FnCall { name, args, line, col };
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
}
