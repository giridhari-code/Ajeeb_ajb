use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

impl Parser {
    pub(super) fn parse_pattern(&mut self) -> Result<Pattern, CompileError> {
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
                    Ok(Pattern::Wildcard)
                }
            }
            _ => Err(self.err("Invalid pattern. Use _, enum::Variant, or literal.")),
        }
    }
}
