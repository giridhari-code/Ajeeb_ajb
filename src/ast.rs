#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnot {
    Int, String, Bool, Void,
    Array(Box<TypeAnnot>),
    Class(String),
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: String,
    pub type_ann: TypeAnnot,
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub fields: Vec<ClassField>,
    pub methods: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, type_ann: Option<TypeAnnot>, value: Expr },
    Const { name: String, type_ann: Option<TypeAnnot>, value: Expr },
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Expr, body: Vec<Stmt> },
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
    Field { obj: Box<Expr>, field: String, class_name: Option<String> },
    FieldAssign { obj: Box<Expr>, field: String, class_name: Option<String>, value: Box<Expr> },
    Group(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, Lt, Gt, Le, Ge,
    And, Or,
}
