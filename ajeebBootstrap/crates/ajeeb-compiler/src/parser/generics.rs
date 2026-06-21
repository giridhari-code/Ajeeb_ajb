use super::Parser;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

impl Parser {
    pub(super) fn peek_type_args(&self) -> bool {
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
    pub(super) fn parse_type_arg_list(&mut self) -> Result<Vec<TypeAnnot>, CompileError> {
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

}
