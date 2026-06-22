use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum HirType {
    Int,
    Float,
    Bool,
    Str,
    Void,
    Named(String),
    Array(Box<HirType>),
    Generic(String, Vec<HirType>),
    Unknown,
}

impl HirType {
    pub fn is_unknown(&self) -> bool {
        matches!(self, HirType::Unknown)
    }

    pub fn is_compatible_with(&self, other: &HirType) -> bool {
        if self == other { return true; }
        if self.is_unknown() || other.is_unknown() { return true; }
        match (self, other) {
            (HirType::Int, HirType::Float) | (HirType::Float, HirType::Int) => true,
            (HirType::Int, HirType::Str) | (HirType::Str, HirType::Int) => true,
            (HirType::Array(a), HirType::Array(b)) => a.is_compatible_with(b),
            _ => false,
        }
    }
}

impl fmt::Display for HirType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HirType::Int => write!(f, "Int"),
            HirType::Float => write!(f, "Float"),
            HirType::Bool => write!(f, "Bool"),
            HirType::Str => write!(f, "Str"),
            HirType::Void => write!(f, "Void"),
            HirType::Named(n) => write!(f, "{}", n),
            HirType::Array(inner) => write!(f, "[{}]", inner),
            HirType::Generic(name, args) => {
                write!(f, "{}<", name)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", a)?;
                }
                write!(f, ">")
            }
            HirType::Unknown => write!(f, "?"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HirFn {
    pub name: String,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
    pub body: Vec<HirStmt>,
    pub is_generic: bool,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum HirStmt {
    Set {
        name: String,
        ty: HirType,
        value: HirExpr,
    },
    Return(HirExpr),
    If {
        cond: HirExpr,
        then: Vec<HirStmt>,
        else_: Vec<HirStmt>,
    },
    While {
        cond: HirExpr,
        body: Vec<HirStmt>,
    },
    For {
        init: Box<HirStmt>,
        cond: HirExpr,
        update: Box<HirStmt>,
        body: Vec<HirStmt>,
    },
    Expr(HirExpr),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub enum HirBinOp {
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

#[derive(Debug, Clone)]
pub enum HirExpr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Var {
        name: String,
        ty: HirType,
    },
    BinOp {
        op: HirBinOp,
        left: Box<HirExpr>,
        right: Box<HirExpr>,
        ty: HirType,
    },
    Call {
        name: String,
        args: Vec<HirExpr>,
        ty: HirType,
    },
    MethodCall {
        receiver: Box<HirExpr>,
        method: String,
        args: Vec<HirExpr>,
        ty: HirType,
    },
    StructLit {
        name: String,
        fields: Vec<(String, HirExpr)>,
        ty: HirType,
    },
    FieldAccess {
        obj: Box<HirExpr>,
        field: String,
        ty: HirType,
    },
    FieldAssign {
        obj: Box<HirExpr>,
        field: String,
        value: Box<HirExpr>,
        ty: HirType,
    },
    ArrayLit {
        elems: Vec<HirExpr>,
        ty: HirType,
    },
    Index {
        obj: Box<HirExpr>,
        idx: Box<HirExpr>,
        ty: HirType,
    },
    IndexAssign {
        obj: Box<HirExpr>,
        idx: Box<HirExpr>,
        value: Box<HirExpr>,
        ty: HirType,
    },
    EnumCtor {
        enum_name: String,
        variant: String,
        args: Vec<HirExpr>,
        ty: HirType,
    },
    UnaryMinus(Box<HirExpr>, HirType),
    UnaryNot(Box<HirExpr>, HirType),
    Assign {
        name: String,
        value: Box<HirExpr>,
        ty: HirType,
    },
}

impl HirExpr {
    pub fn ty(&self) -> &HirType {
        match self {
            HirExpr::Int(_) => &HirType::Int,
            HirExpr::Float(_) => &HirType::Float,
            HirExpr::Str(_) => &HirType::Str,
            HirExpr::Bool(_) => &HirType::Bool,
            HirExpr::Var { ty, .. } => ty,
            HirExpr::BinOp { ty, .. } => ty,
            HirExpr::Call { ty, .. } => ty,
            HirExpr::MethodCall { ty, .. } => ty,
            HirExpr::StructLit { ty, .. } => ty,
            HirExpr::FieldAccess { ty, .. } => ty,
            HirExpr::FieldAssign { ty, .. } => ty,
            HirExpr::ArrayLit { ty, .. } => ty,
            HirExpr::Index { ty, .. } => ty,
            HirExpr::IndexAssign { ty, .. } => ty,
            HirExpr::EnumCtor { ty, .. } => ty,
            HirExpr::UnaryMinus(_, ty) => ty,
            HirExpr::UnaryNot(_, ty) => ty,
            HirExpr::Assign { ty, .. } => ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HirStructDef {
    pub name: String,
    pub fields: Vec<(String, HirType)>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HirEnumDef {
    pub name: String,
    pub variants: Vec<(String, Vec<HirType>)>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HirTraitDef {
    pub name: String,
    pub methods: Vec<HirTraitMethod>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HirTraitMethod {
    pub name: String,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
}

#[derive(Debug, Clone)]
pub struct HirImplBlock {
    pub trait_name: Option<String>,
    pub type_name: String,
    pub methods: Vec<HirFn>,
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HirProgram {
    pub functions: Vec<HirFn>,
    pub structs: Vec<HirStructDef>,
    pub enums: Vec<HirEnumDef>,
    pub traits: Vec<HirTraitDef>,
    pub impls: Vec<HirImplBlock>,
}
