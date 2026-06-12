#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnot {
    Int, String, Bool, Void,
    Array(Box<TypeAnnot>),
    Class(String),
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, value: Expr },
    Const { name: String, value: Expr },
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Expr, body: Vec<Stmt> },
    ForLoop { init: Box<Stmt>, condition: Expr, update: Box<Stmt>, body: Vec<Stmt> },
    Break,
    Continue,
    Return { value: Option<Expr> },
    Expr(Expr),
    FnDef { name: String, params: Vec<(String, TypeAnnot)>, return_type: TypeAnnot, body: Vec<Stmt> },
    Class { name: String, fields: Vec<ClassField>, methods: Vec<Stmt> },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64),
    StringLit(String),
    Bool(bool),
    Ident(String),
    Binary { left: Box<Expr>, op: BinOp, right: Box<Expr> },
    Assign { name: String, value: Box<Expr> },
    IndexAssign { obj: Box<Expr>, index: Box<Expr>, value: Box<Expr> },
    FnCall { name: String, args: Vec<Expr> },
    New { class_name: String },
    ArrayLit(Vec<Expr>),
    Index { obj: Box<Expr>, index: Box<Expr> },
    Field { obj: Box<Expr>, field: String },
    FieldAssign { obj: Box<Expr>, field: String, value: Box<Expr> },
    UnaryNot(Box<Expr>),
    Group(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, Lt, Gt, Le, Ge,
    And, Or,
}
