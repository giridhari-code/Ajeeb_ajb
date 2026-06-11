#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Let, Const, If, Else, While, Function, Return, True, False,
    Int, String, Bool, Void,
    Identifier(String), Number(i64), StringLiteral(String),
    Plus, Minus, Star, Slash, Assign,
    Eq, Neq, Lt, Gt, Le, Ge, And, Or, Not,
    Semicolon, Colon, Comma, Arrow, Dot,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket,
    Class, SelfKwd, New, For,
    Break, Continue,
    Eof,
}
