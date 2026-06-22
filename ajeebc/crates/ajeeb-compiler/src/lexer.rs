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
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) -> Result<(), CompileError> {
        if self.peek() == Some('/') {
            self.advance();
            if self.peek() == Some('/') {
                while let Some(c) = self.advance() {
                    if c == '\n' {
                        break;
                    }
                }
            } else if self.peek() == Some('*') {
                self.advance();
                let start_line = self.line;
                let start_col = self.col - 2;
                let mut depth = 1;
                while let Some(c) = self.advance() {
                    if c == '*' && self.peek() == Some('/') {
                        self.advance();
                        depth -= 1;
                        if depth == 0 { break; }
                    } else if c == '/' && self.peek() == Some('*') {
                        self.advance();
                        depth += 1;
                    }
                }
                if depth > 0 {
                    return Err(CompileError::new(
                        start_line,
                        start_col,
                        "Block comment khatam nahi hua! EOF tak `*/` nahi mila.".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn read_string(&mut self) -> Result<Token, CompileError> {
        let start_line = self.line;
        let start_col = self.col;
        let mut s = String::new();
        loop {
            match self.advance() {
                None => {
                    return Err(CompileError::new(
                        start_line,
                        start_col,
                        "String khatam nahi hui! Closing quote (\") chahiye.".to_string(),
                    ))
                }
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('t') => s.push('\t'),
                    Some('"') => s.push('"'),
                    Some('\\') => s.push('\\'),
                    Some('0') => s.push('\0'),
                    _ => {
                        return Err(CompileError::new(
                            self.line,
                            self.col,
                            "Galat escape sequence. Sirf \\n, \\t, \\\", \\\\, \\0 allowed hain."
                                .to_string(),
                        ))
                    }
                },
                Some(c) => s.push(c),
            }
        }
        Ok(Token::StringLiteral(s))
    }

    fn read_number(&mut self, first: char) -> Result<Token, CompileError> {
        let mut s = String::new();
        s.push(first);
        let mut is_float = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else if c == '.' && !is_float {
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        is_float = true;
                        s.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        if is_float {
            let val: f64 = s.parse().map_err(|_| {
                CompileError::new(
                    self.line,
                    self.col,
                    "Float number parse nahi ho paaya.".to_string(),
                )
            })?;
            Ok(Token::FloatLiteral(val))
        } else {
            let val: i64 = s.parse().map_err(|_| {
                CompileError::new(
                    self.line,
                    self.col,
                    "Number bahut bada hai! i64 me fit nahi ho raha.".to_string(),
                )
            })?;
            Ok(Token::Number(val))
        }
    }

    fn read_identifier(&mut self, first: char) -> Token {
        let mut s = String::new();
        s.push(first);
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "set" => Token::Set,
            "const" => Token::Const,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "function" => Token::Function,
            "fn" => Token::Function,
            "return" => Token::Return,
            "true" => Token::True,
            "false" => Token::False,
            "int" => Token::Int,
            "float" => Token::Float,
            "string" => Token::String,
            "bool" => Token::Bool,
            "void" => Token::Void,
            "class" => Token::Class,
            "self" => Token::SelfKwd,
            "new" => Token::New,
            "for" => Token::For,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "import" => Token::Import,
            "pub" => Token::Pub,
            "struct" => Token::Struct,
            "enum" => Token::Enum,
            "match" => Token::Match,
            "trait" => Token::Trait,
            "impl" => Token::Impl,
            _ => Token::Identifier(s),
        }
    }

    fn read_at_keyword(&mut self) -> Result<Token, CompileError> {
        let start_line = self.line;
        let start_col = self.col;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        match s.as_str() {
            "import" => Ok(Token::AtImport),
            _ => Err(CompileError::new(
                start_line,
                start_col,
                format!("Unknown @-keyword: @{}. Sirf '@import' valid hai.", s),
            )),
        }
    }

    #[allow(dead_code)]
    pub fn next_token(&mut self) -> Result<Token, CompileError> {
        let (tok, _line, _col) = self.next_token_spanned()?;
        Ok(tok)
    }

    pub fn next_token_spanned(&mut self) -> Result<(Token, usize, usize), CompileError> {
        loop {
            self.skip_whitespace();
            if self.peek() == Some('/') {
                let saved = self.pos;
                self.advance();
                if self.peek() == Some('/') || self.peek() == Some('*') {
                    self.pos = saved;
                    self.skip_comment()?;
                    continue;
                }
                self.pos = saved;
            }
            break;
        }

        let start_line = self.line;
        let start_col = self.col;

        match self.advance() {
            None => Ok((Token::Eof, start_line, start_col)),
            Some(c) => match c {
                '+' => Ok((Token::Plus, start_line, start_col)),
                '-' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        Ok((Token::Arrow, start_line, start_col))
                    } else {
                        Ok((Token::Minus, start_line, start_col))
                    }
                }
                '*' => Ok((Token::Star, start_line, start_col)),
                '/' => Ok((Token::Slash, start_line, start_col)),
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Ok((Token::Eq, start_line, start_col))
                    } else if self.peek() == Some('>') {
                        self.advance();
                        Ok((Token::FatArrow, start_line, start_col))
                    } else {
                        Ok((Token::Assign, start_line, start_col))
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Ok((Token::Neq, start_line, start_col))
                    } else {
                        Ok((Token::Not, start_line, start_col))
                    }
                }
                '&' => {
                    if self.peek() == Some('&') {
                        self.advance();
                        Ok((Token::And, start_line, start_col))
                    } else {
                        Err(CompileError::new(
                            start_line,
                            start_col,
                            "Akela '&' kaam nahi karega. '&&' use karo.".to_string(),
                        ))
                    }
                }
                '|' => {
                    if self.peek() == Some('|') {
                        self.advance();
                        Ok((Token::Or, start_line, start_col))
                    } else {
                        Err(CompileError::new(
                            start_line,
                            start_col,
                            "Akela '|' kaam nahi karega. '||' use karo.".to_string(),
                        ))
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Ok((Token::Le, start_line, start_col))
                    } else {
                        Ok((Token::Lt, start_line, start_col))
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        Ok((Token::Ge, start_line, start_col))
                    } else {
                        Ok((Token::Gt, start_line, start_col))
                    }
                }
                ';' => Ok((Token::Semicolon, start_line, start_col)),
                ':' => {
                    if self.peek() == Some(':') {
                        self.advance();
                        Ok((Token::DoubleColon, start_line, start_col))
                    } else {
                        Ok((Token::Colon, start_line, start_col))
                    }
                }
                ',' => Ok((Token::Comma, start_line, start_col)),
                '(' => Ok((Token::LParen, start_line, start_col)),
                ')' => Ok((Token::RParen, start_line, start_col)),
                '{' => Ok((Token::LBrace, start_line, start_col)),
                '}' => Ok((Token::RBrace, start_line, start_col)),
                '[' => Ok((Token::LBracket, start_line, start_col)),
                ']' => Ok((Token::RBracket, start_line, start_col)),
                '.' => Ok((Token::Dot, start_line, start_col)),
                '"' => self.read_string().map(|t| (t, start_line, start_col)),
                c if c.is_ascii_digit() => self.read_number(c).map(|t| (t, start_line, start_col)),
                c if c.is_alphabetic() || c == '_' => {
                    Ok((self.read_identifier(c), start_line, start_col))
                }
                '@' => self.read_at_keyword().map(|t| (t, start_line, start_col)),
                _ => Err(CompileError::new(
                    start_line,
                    start_col,
                    format!("Unexpected character '{}'. Yeh kya hai bhai?", c),
                )),
            },
        }
    }
}
