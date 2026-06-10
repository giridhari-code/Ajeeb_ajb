use crate::error::CompileError;
use crate::token::Token;

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
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
        let mut s = String::new();
        loop {
            match self.advance() {
                None => return Err(CompileError::new(start_line, start_col, "String khatam nahi hui! Closing quote (\") chahiye.".to_string())),
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

    fn read_number(&mut self, first: char) -> Result<Token, CompileError> {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() { s.push(c); self.advance(); } else { break; }
        }
        let val: i64 = s.parse().map_err(|_| {
            CompileError::new(self.line, self.col, "Number bahut bada hai! i64 me fit nahi ho raha.".to_string())
        })?;
        Ok(Token::Number(val))
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
            "class" => Token::Class, "self" => Token::SelfKwd, "new" => Token::New,
            _ => Token::Identifier(s),
        }
    }

    pub fn next_token(&mut self) -> Result<Token, CompileError> {
        loop {
            self.skip_whitespace();
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
                    else { Ok(Token::Not) }
                }
                '&' => {
                    if self.peek() == Some('&') { self.advance(); Ok(Token::And) }
                    else { Err(CompileError::new(start_line, start_col, "Akela '&' kaam nahi karega. '&&' use karo.".to_string())) }
                }
                '|' => {
                    if self.peek() == Some('|') { self.advance(); Ok(Token::Or) }
                    else { Err(CompileError::new(start_line, start_col, "Akela '|' kaam nahi karega. '||' use karo.".to_string())) }
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
                c if c.is_ascii_digit() => self.read_number(c),
                c if c.is_alphabetic() || c == '_' => Ok(self.read_identifier(c)),
                _ => Err(CompileError::new(start_line, start_col, format!("Unexpected character '{}'. Yeh kya hai bhai?", c))),
            }
        }
    }
}
