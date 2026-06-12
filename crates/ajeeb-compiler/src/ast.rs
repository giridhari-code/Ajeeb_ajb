#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnot {
    Int,
    String,
    Bool,
    Void,
    Array(Box<TypeAnnot>),
    Class(String),
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let {
        name: String,
        type_ann: Option<TypeAnnot>,
        value: Expr,
        line: usize,
        col: usize,
    },
    Const {
        name: String,
        type_ann: Option<TypeAnnot>,
        value: Expr,
        line: usize,
        col: usize,
    },
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_block: Option<Vec<Stmt>>,
        line: usize,
        col: usize,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
        line: usize,
        col: usize,
    },
    ForLoop {
        init: Box<Stmt>,
        condition: Expr,
        update: Box<Stmt>,
        body: Vec<Stmt>,
        line: usize,
        col: usize,
    },
    Break {
        line: usize,
        col: usize,
    },
    Continue {
        line: usize,
        col: usize,
    },
    Return {
        value: Option<Expr>,
        line: usize,
        col: usize,
    },
    Expr(Expr, usize, usize),
    FnDef {
        name: String,
        params: Vec<(String, TypeAnnot)>,
        return_type: TypeAnnot,
        body: Vec<Stmt>,
        line: usize,
        col: usize,
    },
    Class {
        name: String,
        fields: Vec<ClassField>,
        methods: Vec<Stmt>,
        line: usize,
        col: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64, usize, usize),
    StringLit(String, usize, usize),
    Bool(bool, usize, usize),
    Ident(String, usize, usize),
    Binary {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
        line: usize,
        col: usize,
    },
    Assign {
        name: String,
        value: Box<Expr>,
        line: usize,
        col: usize,
    },
    IndexAssign {
        obj: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
        line: usize,
        col: usize,
    },
    FnCall {
        name: String,
        args: Vec<Expr>,
        line: usize,
        col: usize,
    },
    New {
        class_name: String,
        line: usize,
        col: usize,
    },
    ArrayLit(Vec<Expr>, usize, usize),
    Index {
        obj: Box<Expr>,
        index: Box<Expr>,
        line: usize,
        col: usize,
    },
    Field {
        obj: Box<Expr>,
        field: String,
        line: usize,
        col: usize,
    },
    FieldAssign {
        obj: Box<Expr>,
        field: String,
        value: Box<Expr>,
        line: usize,
        col: usize,
    },
    UnaryNot(Box<Expr>, usize, usize),
    Group(Box<Expr>, usize, usize),
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}
