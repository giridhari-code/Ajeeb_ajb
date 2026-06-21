#[derive(Debug, Clone, PartialEq)]
pub enum TypeAnnot {
    Int,
    Float,
    String,
    Bool,
    Void,
    Array(Box<TypeAnnot>),
    Class(String),
    Generic(String),                           // Type parameter reference: T
    Parameterized {                            // Instantiated generic: List[Int]
        base: Box<TypeAnnot>,
        args: Vec<TypeAnnot>,
    },
    Function {
        param_types: Vec<TypeAnnot>,
        return_type: Box<TypeAnnot>,
    },
}

#[derive(Debug, Clone)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct ClassField {
    pub name: String,
    pub type_ann: TypeAnnot,
    pub pub_: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Visibility {
    Pub,
    Priv,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub c_import: bool,  // true = C shared library import (@import "lib.so")
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub name: String,
    pub type_ann: TypeAnnot,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: String,
    pub fields: Vec<TypeAnnot>,
}

#[derive(Debug, Clone)]
pub struct TraitMethod {
    pub name: String,
    pub params: Vec<(String, TypeAnnot)>,
    pub return_type: TypeAnnot,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: Expr,
    pub body_block: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Wildcard,
    EnumVariant {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,
    },
    Int(i64),
    String(String),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Set {
        name: String,
        type_ann: Option<TypeAnnot>,
        value: Expr,
        pub_: bool,
        line: usize,
        col: usize,
    },
    Const {
        name: String,
        type_ann: Option<TypeAnnot>,
        value: Expr,
        pub_: bool,
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
        type_params: Vec<String>,              // Generic type parameter names
        type_param_bounds: Vec<(String, Vec<String>)>,  // (param_name, trait_bounds)
        params: Vec<(String, TypeAnnot)>,
        return_type: TypeAnnot,
        body: Vec<Stmt>,
        pub_: bool,
        line: usize,
        col: usize,
    },
    Class {
        name: String,
        fields: Vec<ClassField>,
        methods: Vec<Stmt>,
        pub_: bool,
        line: usize,
        col: usize,
    },
    Import(ImportDecl),
    StructDef {
        name: String,
        type_params: Vec<String>,
        type_param_bounds: Vec<(String, Vec<String>)>,
        fields: Vec<StructField>,
        pub_: bool,
        line: usize,
        col: usize,
    },
    EnumDef {
        name: String,
        type_params: Vec<String>,
        type_param_bounds: Vec<(String, Vec<String>)>,
        variants: Vec<EnumVariantDef>,
        pub_: bool,
        line: usize,
        col: usize,
    },
    TraitDef {
        name: String,
        type_params: Vec<String>,
        type_param_bounds: Vec<(String, Vec<String>)>,
        methods: Vec<TraitMethod>,
        pub_: bool,
        line: usize,
        col: usize,
    },
    ImplBlock {
        trait_name: Option<String>,              // None = inherent impl, Some = trait impl
        trait_type_args: Vec<String>,            // Type args for generic trait: impl Display[Int] for T
        type_params: Vec<String>,                // Generic type params: impl[T] or impl[T: Bound]
        type_param_bounds: Vec<(String, Vec<String>)>,  // (param_name, trait_bounds)
        type_name: String,
        methods: Vec<Stmt>,
        line: usize,
        col: usize,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(i64, usize, usize),
    FloatLit(f64, usize, usize),
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
    GenericCall {
        name: String,
        type_args: Vec<TypeAnnot>,
        args: Vec<Expr>,
        line: usize,
        col: usize,
    },
    MethodCall {
        obj: Box<Expr>,
        method: String,
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
    UnaryMinus(Box<Expr>, usize, usize),
    UnaryNot(Box<Expr>, usize, usize),
    Group(Box<Expr>, usize, usize),
    StructLit {
        struct_name: String,
        fields: Vec<(String, Expr)>,
        line: usize,
        col: usize,
    },
    EnumRef {
        enum_name: String,
        variant: String,
        line: usize,
        col: usize,
    },
    EnumCtor {
        enum_name: String,
        variant: String,
        args: Vec<Expr>,
        line: usize,
        col: usize,
    },
    ClosureCall {
        callee: Box<Expr>,
        args: Vec<Expr>,
        line: usize,
        col: usize,
    },
    AssociatedFnCall {
        type_name: String,
        method: String,
        args: Vec<Expr>,
        line: usize,
        col: usize,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
        line: usize,
        col: usize,
    },
    Lambda {
        params: Vec<(String, TypeAnnot)>,
        return_type: Option<TypeAnnot>,
        body: LambdaBody,
        line: usize,
        col: usize,
    },
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
