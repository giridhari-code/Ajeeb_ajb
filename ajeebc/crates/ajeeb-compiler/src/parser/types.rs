use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

impl Parser {
    pub(super) fn parse_type(&mut self) -> Result<Option<TypeAnnot>, CompileError> {
        if self.peek() == &Token::Colon {
            self.advance();
            let ty = self.parse_type_postfix()?;
            Ok(Some(ty))
        } else if self.peek() == &Token::Arrow {
            self.advance();
            let ty = self.parse_type_postfix()?;
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    pub(super) fn parse_type_postfix(&mut self) -> Result<TypeAnnot, CompileError> {
        let mut base = self.parse_single_type()?;
        loop {
            if self.peek() == &Token::LBracket {
                let next = self.peek_next();
                if next == Some(&Token::RBracket) {
                    self.advance();
                    self.advance();
                    base = TypeAnnot::Array(Box::new(base));
                } else {
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

    pub(super) fn parse_single_type(&mut self) -> Result<TypeAnnot, CompileError> {
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
}
