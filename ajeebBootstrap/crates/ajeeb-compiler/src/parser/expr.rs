use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

impl Parser {
    pub(super) fn parse_expression(&mut self) -> Result<Expr, CompileError> {
        if self.peek() == &Token::Match {
            return self.parse_match_expr();
        }
        self.parse_assignment()
    }

    pub(super) fn parse_match_expr(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_assignment(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_or(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_and(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_equality(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_comparison(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_term(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_factor(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_unary(&mut self) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_struct_lit(&mut self, name: String, line: usize, col: usize) -> Result<Expr, CompileError> {
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

    pub(super) fn parse_primary(&mut self) -> Result<Expr, CompileError> {
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
                if self.is_lambda_start() {
                    return self.parse_lambda();
                }
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
                // Check for EnumRef, StructLit, or AssociatedFnCall
                if self.peek() == &Token::DoubleColon {
                    self.advance();
                    let variant = match self.advance() {
                        Token::Identifier(v) => v,
                        Token::New => "new".to_string(),
                        _ => return Err(self.err("Enum variant name expected after ::")),
                    };
                    if self.peek() == &Token::LParen {
                        // Heuristic: lowercase identifier = associated fn call, uppercase = enum ctor
                        let is_assoc = variant.chars().next().map_or(false, |c| c.is_lowercase());
                        let args = self.parse_call_args()?;
                        if is_assoc {
                            Expr::AssociatedFnCall {
                                type_name: name,
                                method: variant,
                                args,
                                line,
                                col,
                            }
                        } else {
                            Expr::EnumCtor {
                                enum_name: name,
                                variant,
                                args,
                                line,
                                col,
                            }
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
                    let type_args = self.parse_type_arg_list()?;
                    if self.peek() == &Token::LParen {
                        // Generic function call: fnName[TypeArgs](args)
                        let args = self.parse_call_args()?;
                        Expr::GenericCall { name, type_args, args, line, col }
                    } else if self.peek() == &Token::Dot && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                        // Generic enum access: Option[Int].Some(10) or Option[Int].None
                        self.advance(); // consume .
                        let variant = match self.advance() {
                            Token::Identifier(v) => v,
                            _ => return Err(self.err("Enum variant name expected after .")),
                        };
                        // Build full enum name with type args
                        let mut full_name = name.clone();
                        full_name.push('[');
                        for (i, ta) in type_args.iter().enumerate() {
                            if i > 0 { full_name.push(','); }
                            match ta {
                                TypeAnnot::Class(s) => full_name.push_str(s),
                                TypeAnnot::Generic(s) => full_name.push_str(s),
                                TypeAnnot::Int => full_name.push_str("Int"),
                                TypeAnnot::Float => full_name.push_str("Float"),
                                TypeAnnot::String => full_name.push_str("String"),
                                TypeAnnot::Bool => full_name.push_str("Bool"),
                                _ => full_name.push_str("?"),
                            }
                        }
                        full_name.push(']');
                        if self.peek() == &Token::LParen {
                            let args = self.parse_call_args()?;
                            Expr::EnumCtor {
                                enum_name: full_name,
                                variant,
                                args,
                                line,
                                col,
                            }
                        } else {
                            Expr::EnumRef {
                                enum_name: full_name,
                                variant,
                                line,
                                col,
                            }
                        }
                    } else if self.peek() == &Token::DoubleColon && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                        // Generic associated function call: Box[Int]::new(42)
                        self.advance(); // consume ::
                        let method = match self.advance() {
                            Token::Identifier(v) => v,
                            Token::New => "new".to_string(),
                            _ => return Err(self.err("Method name expected after ::")),
                        };
                        // Build full type name with type args
                        let mut full_name = name.clone();
                        full_name.push('[');
                        for (i, ta) in type_args.iter().enumerate() {
                            if i > 0 { full_name.push(','); }
                            match ta {
                                TypeAnnot::Class(s) => full_name.push_str(s),
                                TypeAnnot::Generic(s) => full_name.push_str(s),
                                TypeAnnot::Int => full_name.push_str("Int"),
                                TypeAnnot::Float => full_name.push_str("Float"),
                                TypeAnnot::String => full_name.push_str("String"),
                                TypeAnnot::Bool => full_name.push_str("Bool"),
                                _ => full_name.push_str("?"),
                            }
                        }
                        full_name.push(']');
                        let args = self.parse_call_args()?;
                        Expr::AssociatedFnCall {
                            type_name: full_name,
                            method,
                            args,
                            line,
                            col,
                        }
                    } else if self.peek() == &Token::LBrace && name.chars().next().map_or(false, |c| c.is_uppercase()) {
                        // Generic struct literal: Box[Int] { value: 42 }
                        let mut fields = Vec::new();
                        self.advance(); // consume {
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
                        let mut full_name = name.clone();
                        full_name.push('[');
                        for (i, ta) in type_args.iter().enumerate() {
                            if i > 0 { full_name.push(','); }
                            match ta {
                                TypeAnnot::Class(s) => full_name.push_str(s),
                                TypeAnnot::Generic(s) => full_name.push_str(s),
                                TypeAnnot::Int => full_name.push_str("Int"),
                                TypeAnnot::Float => full_name.push_str("Float"),
                                TypeAnnot::String => full_name.push_str("String"),
                                TypeAnnot::Bool => full_name.push_str("Bool"),
                                _ => full_name.push_str("?"),
                            }
                        }
                        full_name.push(']');
                        Expr::StructLit {
                            struct_name: full_name,
                            fields,
                            line,
                            col,
                        }
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
                    match &expr {
                        Expr::Ident(name, ..) => {
                            let args = self.parse_call_args()?;
                            expr = Expr::FnCall { name: name.clone(), args, line, col };
                        }
                        Expr::MethodCall { obj, method, .. } => {
                            let args = self.parse_call_args()?;
                            expr = Expr::MethodCall {
                                obj: obj.clone(),
                                method: method.clone(),
                                args,
                                line,
                                col,
                            };
                        }
                        _ => {
                            let args = self.parse_call_args()?;
                            expr = Expr::ClosureCall {
                                callee: Box::new(expr.clone()),
                                args,
                                line,
                                col,
                            };
                        }
                    }
                }
                Token::Dot => {
                    let (line, col) = Self::expr_pos(&expr);
                    self.advance();
                    let field = match self.advance() {
                        Token::Identifier(n) => n,
                        _ => return Err(self.err("'.' ke baad field/method ka naam chahiye.")),
                    };
                    // Check for enum access: UppercaseIdent.UppercaseVariant
                    if let Expr::Ident(ref obj_name, ..) = expr {
                        if obj_name.chars().next().map_or(false, |c| c.is_uppercase())
                            && field.chars().next().map_or(false, |c| c.is_uppercase())
                        {
                            if self.peek() == &Token::LParen {
                                let args = self.parse_call_args()?;
                                expr = Expr::EnumCtor {
                                    enum_name: obj_name.clone(),
                                    variant: field,
                                    args,
                                    line,
                                    col,
                                };
                            } else {
                                expr = Expr::EnumRef {
                                    enum_name: obj_name.clone(),
                                    variant: field,
                                    line,
                                    col,
                                };
                            }
                            continue;
                        }
                    }
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
                    // Check for generic struct literal: TypeName[TypeArgs] { ... }
                    if let Expr::Ident(ref name, ..) = expr {
                        if name.chars().next().map_or(false, |c| c.is_uppercase())
                            && self.peek_type_args()
                        {
                            let type_args = self.parse_type_arg_list()?;
                            if self.peek() == &Token::LBrace {
                                // Generic struct literal: Box<Int> { value: 42 }
                                let mut fields = Vec::new();
                                self.advance(); // consume {
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
                                // Return a struct lit with the generic type info stored in the name
                                // The semantic analyzer will use the type annotation for resolution
                                let mut full_name = name.clone();
                                full_name.push('[');
                                for (i, ta) in type_args.iter().enumerate() {
                                    if i > 0 { full_name.push(','); }
                                    match ta {
                                        TypeAnnot::Class(s) => full_name.push_str(s),
                                        TypeAnnot::Generic(s) => full_name.push_str(s),
                                        TypeAnnot::Int => full_name.push_str("Int"),
                                        TypeAnnot::Float => full_name.push_str("Float"),
                                        TypeAnnot::String => full_name.push_str("String"),
                                        TypeAnnot::Bool => full_name.push_str("Bool"),
                                        _ => full_name.push_str("?"),
                                    }
                                }
                                full_name.push(']');
                                expr = Expr::StructLit {
                                    struct_name: full_name,
                                    fields,
                                    line,
                                    col,
                                };
                            } else {
                                return Err(self.err("Generic type arguments must be followed by '{' for struct literal or '(' for constructor."));
                            }
                            continue;
                        }
                    }
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

    pub(super) fn parse_call_args(&mut self) -> Result<Vec<Expr>, CompileError> {
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

    /// Check if current `(` starts a lambda parameter list (not a grouped expression).
    /// Looks for: `ident :` or `ident ,` or `)` followed by `=>`
    fn is_lambda_start(&self) -> bool {
        let saved = self.pos;
        let mut pos = saved;
        let tokens = &self.tokens;

        // Must start with `(`
        if tokens.get(pos).map_or(true, |t| t != &Token::LParen) { return false; }
        pos += 1;

        // Empty params: `() =>`
        if tokens.get(pos) == Some(&Token::RParen) {
            // Check if followed by `: type` or `=>`
            pos += 1;
            // Skip optional `: type`
            if tokens.get(pos) == Some(&Token::Colon) || tokens.get(pos) == Some(&Token::Arrow) {
                pos += 1;
                // Skip the return type (one token for simple types, more for complex)
                if tokens.get(pos).map_or(false, |t| {
                    matches!(t, Token::Int | Token::Float | Token::String | Token::Bool | Token::Void | Token::Identifier(_))
                }) {
                    pos += 1;
                }
            }
            return tokens.get(pos) == Some(&Token::FatArrow);
        }

        // Check for `ident : type, ident :` pattern
        let mut depth = 1;
        while depth > 0 {
            match tokens.get(pos) {
                Some(Token::LParen) => { depth += 1; pos += 1; }
                Some(Token::RParen) => {
                    depth -= 1;
                    if depth == 0 {
                        pos += 1;
                        // After `)` check for `=>`
                        if tokens.get(pos) == Some(&Token::Colon) || tokens.get(pos) == Some(&Token::Arrow) {
                            pos += 1;
                            if tokens.get(pos).map_or(false, |t| {
                                matches!(t, Token::Int | Token::Float | Token::String | Token::Bool | Token::Void | Token::Identifier(_))
                            }) {
                                pos += 1;
                            }
                        }
                        return tokens.get(pos) == Some(&Token::FatArrow);
                    }
                    pos += 1;
                }
                Some(Token::Identifier(_)) => {
                    pos += 1;
                    // After identifier -> must be `:` or `,` or `)`
                    match tokens.get(pos) {
                        Some(Token::Colon) | Some(Token::Comma) => {
                            pos += 1;
                            // Skip type annotation after `:`
                            if tokens.get(pos.saturating_sub(1)) == Some(&Token::Colon) {
                                // Skip one type token
                                if tokens.get(pos).map_or(false, |t| {
                                    matches!(t, Token::Int | Token::Float | Token::String | Token::Bool | Token::Void | Token::Identifier(_))
                                }) {
                                    pos += 1;
                                    // Handle Array type: `[]` after type
                                    if tokens.get(pos) == Some(&Token::LBracket) && tokens.get(pos + 1) == Some(&Token::RBracket) {
                                        pos += 2;
                                    }
                                }
                            }
                        }
                        Some(Token::RParen) => {
                            depth -= 1;
                            pos += 1;
                            if depth == 0 {
                                // After `)` check for `=>`
                                if tokens.get(pos) == Some(&Token::Colon) || tokens.get(pos) == Some(&Token::Arrow) {
                                    pos += 1;
                                    if tokens.get(pos).map_or(false, |t| {
                                        matches!(t, Token::Int | Token::Float | Token::String | Token::Bool | Token::Void | Token::Identifier(_))
                                    }) {
                                        pos += 1;
                                    }
                                }
                                return tokens.get(pos) == Some(&Token::FatArrow);
                            }
                        }
                        _ => return false,
                    }
                }
                // Comma is valid between params — just advance
                Some(Token::Comma) => { pos += 1; }
                _ => return false,
            }
        }
        false
    }

    fn parse_lambda(&mut self) -> Result<Expr, CompileError> {
        let line = self.line();
        let col = self.col();
        self.advance(); // consume (
        let mut params = Vec::new();
        while self.peek() != &Token::RParen {
            let pname = match self.advance() {
                Token::Identifier(n) => n,
                _ => return Err(self.err("Lambda parameter name chahiye.")),
            };
            let ptype = match self.parse_type()? {
                Some(t) => t,
                None => return Err(self.err("Lambda parameter ka type batana zaroori hai.")),
            };
            params.push((pname, ptype));
            if self.peek() == &Token::Comma {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        let return_type = match self.parse_type()? {
            Some(t) => Some(t),
            None => None,
        };
        self.expect(&Token::FatArrow)?;
        let body = if self.peek() == &Token::LBrace {
            self.advance();
            let stmts = self.parse_block()?;
            self.expect(&Token::RBrace)?;
            LambdaBody::Block(stmts)
        } else {
            LambdaBody::Expr(Box::new(self.parse_expression()?))
        };
        Ok(Expr::Lambda {
            params,
            return_type,
            body,
            line,
            col,
        })
    }
}
