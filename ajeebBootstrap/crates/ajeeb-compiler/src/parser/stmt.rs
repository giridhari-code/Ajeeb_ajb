use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

impl Parser {
    pub(super) fn parse_statement(&mut self) -> Result<Stmt, CompileError> {
        match self.peek() {
            Token::Import => self.parse_import(),
            _ => {
                let pub_ = self.parse_pub();
                match self.peek() {
                    Token::Set => self.parse_set_decl(pub_),
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
                    Token::AtImport => Err(self.err("'@import' sirf file ke shuru me ho sakta hai. Pehle declare karo, phir @import karo.")),
                    _ => self.parse_expr_stmt(),
                }
            }
        }
    }


    pub(super) fn parse_if_stmt(&mut self) -> Result<Stmt, CompileError> {
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


    pub(super) fn parse_while_stmt(&mut self) -> Result<Stmt, CompileError> {
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


    pub(super) fn parse_for_stmt(&mut self) -> Result<Stmt, CompileError> {
        self.advance();
        let line = self.line();
        let col = self.col();
        self.expect(&Token::LParen)?;
        let init = if self.peek() == &Token::Set {
            self.parse_set_decl(false)?
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


    pub(super) fn parse_return_stmt(&mut self) -> Result<Stmt, CompileError> {
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


    pub(super) fn parse_expr_stmt(&mut self) -> Result<Stmt, CompileError> {
        let line = self.line();
        let col = self.col();
        let expr = self.parse_expression()?;
        self.expect(&Token::Semicolon)?;
        Ok(Stmt::Expr(expr, line, col))
    }


    pub(super) fn parse_block(&mut self) -> Result<Vec<Stmt>, CompileError> {
        let mut stmts = Vec::new();
        while self.peek() != &Token::RBrace && self.peek() != &Token::Eof {
            stmts.push(self.parse_statement()?);
        }
        Ok(stmts)
    }


}
