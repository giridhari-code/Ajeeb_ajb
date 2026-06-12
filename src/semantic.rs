use crate::ast::*;
use crate::error::CompileError;
use std::collections::HashMap;

pub struct SemanticAnalyzer {
    pub errors: Vec<CompileError>,
    scopes: Vec<HashMap<String, TypeAnnot>>,
    functions: HashMap<String, (Vec<(String, TypeAnnot)>, TypeAnnot)>,
    current_function: Option<(String, TypeAnnot)>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut global = HashMap::new();
        global.insert("print".to_string(), TypeAnnot::Void);
        global.insert("println".to_string(), TypeAnnot::Void);
        global.insert("itoa".to_string(), TypeAnnot::String);
        global.insert("len".to_string(), TypeAnnot::Int);
        global.insert("charCode".to_string(), TypeAnnot::Int);
        global.insert("strcmp".to_string(), TypeAnnot::Int);
        global.insert("readFile".to_string(), TypeAnnot::String);
        global.insert("writeFile".to_string(), TypeAnnot::Void);
        global.insert("writeAppend".to_string(), TypeAnnot::Void);
        global.insert("readArg".to_string(), TypeAnnot::String);
        global.insert("substring".to_string(), TypeAnnot::String);
        global.insert("indexOf".to_string(), TypeAnnot::Int);
        global.insert("contains".to_string(), TypeAnnot::Bool);
        global.insert("toUpperCase".to_string(), TypeAnnot::String);
        global.insert("toLowerCase".to_string(), TypeAnnot::String);
        global.insert("trim".to_string(), TypeAnnot::String);
        global.insert("split".to_string(), TypeAnnot::Array(Box::new(TypeAnnot::String)));
        global.insert("replace".to_string(), TypeAnnot::String);
        global.insert("startsWith".to_string(), TypeAnnot::Bool);
        global.insert("endsWith".to_string(), TypeAnnot::Bool);
        global.insert("getStateBuf".to_string(), TypeAnnot::String);
        global.insert("getOutbuf".to_string(), TypeAnnot::String);
        global.insert("rdB".to_string(), TypeAnnot::Int);
        global.insert("getInt".to_string(), TypeAnnot::Int);
        global.insert("wrB".to_string(), TypeAnnot::Void);
        global.insert("setInt".to_string(), TypeAnnot::Void);
        global.insert("strcpy".to_string(), TypeAnnot::Void);
        global.insert("strSet".to_string(), TypeAnnot::Void);
        global.insert("chr".to_string(), TypeAnnot::Int);
        global.insert("writeByte".to_string(), TypeAnnot::Void);
        global.insert("rdPos".to_string(), TypeAnnot::Int);
        global.insert("wrPos".to_string(), TypeAnnot::Void);
        global.insert("isDigit".to_string(), TypeAnnot::Bool);
        global.insert("isAlpha".to_string(), TypeAnnot::Bool);
        global.insert("isAlphaNum".to_string(), TypeAnnot::Bool);
        global.insert("isSpace".to_string(), TypeAnnot::Bool);
        global.insert("strcmp_ajeeb".to_string(), TypeAnnot::Int);
        SemanticAnalyzer {
            errors: Vec::new(),
            scopes: vec![global],
            functions: HashMap::new(),
            current_function: None,
        }
    }

    pub fn analyze(&mut self, program: &[Stmt]) {
        // First pass: collect all function signatures
        for stmt in program {
            if let Stmt::FnDef { name, params, return_type, line, col, .. } = stmt {
                if self.functions.contains_key(name.as_str()) {
                    self.errors.push(CompileError::new(
                        *line,
                        *col,
                        format!("Duplicate function '{}' is already defined", name),
                    ));
                }
                self.functions.insert(name.clone(), (params.clone(), return_type.clone()));
            }
        }
        // Second pass: check top-level statements
        for stmt in program {
            self.check_stmt(stmt);
        }
    }

    fn err(&self, line: usize, col: usize, msg: String) -> CompileError {
        CompileError::new(line, col, msg)
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn lookup_var(&self, name: &str) -> Option<(usize, TypeAnnot)> {
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(ty) = scope.get(name) {
                return Some((i, ty.clone()));
            }
        }
        None
    }

    fn declare_var(&mut self, name: &str, ty: TypeAnnot, line: usize, col: usize) {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(name) {
                self.errors.push(self.err(
                    line,
                    col,
                    format!("Duplicate variable '{}' in the same scope", name),
                ));
                return;
            }
            scope.insert(name.to_string(), ty);
        }
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, type_ann, value, line, col, .. } | Stmt::Const { name, type_ann, value, line, col, .. } => {
                let value_ty = self.infer_expr_type(value);
                if let Some(ann) = type_ann {
                    if !self.types_match(ann, &value_ty) {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!(
                                "Type mismatch: variable '{}' annotated as {:?} but assigned {:?}",
                                name, ann, value_ty
                            ),
                        ));
                    }
                    self.declare_var(name, ann.clone(), *line, *col);
                } else {
                    self.declare_var(name, value_ty, *line, *col);
                }
            }
            Stmt::FnDef { name, params, body, .. } => {
                self.enter_scope();
                for (pname, pty) in params {
                    self.declare_var(pname, pty.clone(), 0, 0);
                }
                let return_ty = self.functions.get(name.as_str()).map(|(_, r)| r.clone()).unwrap_or(TypeAnnot::Void);
                let prev_fn = self.current_function.replace((name.clone(), return_ty));
                for s in body {
                    self.check_stmt(s);
                }
                self.current_function = prev_fn;
                self.exit_scope();
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                self.infer_expr_type(condition);
                self.enter_scope();
                for s in then_block {
                    self.check_stmt(s);
                }
                self.exit_scope();
                if let Some(el) = else_block {
                    self.enter_scope();
                    for s in el {
                        self.check_stmt(s);
                    }
                    self.exit_scope();
                }
            }
            Stmt::While { condition, body, .. } => {
                self.infer_expr_type(condition);
                self.enter_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.exit_scope();
            }
            Stmt::ForLoop { init, condition, update, body, .. } => {
                self.check_stmt(init);
                self.infer_expr_type(condition);
                self.check_stmt(update);
                self.enter_scope();
                for s in body {
                    self.check_stmt(s);
                }
                self.exit_scope();
            }
            Stmt::Return { value, line, col, .. } => {
                let return_ty = value.as_ref().map(|v| self.infer_expr_type(v)).unwrap_or(TypeAnnot::Void);
                if let Some((fn_name, expected_ty)) = &self.current_function {
                    if !self.types_match(expected_ty, &return_ty) {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!(
                                "Return type mismatch in '{}': expected {:?} but expression is {:?}",
                                fn_name, expected_ty, return_ty
                            ),
                        ));
                    }
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        "Return statement outside of a function".to_string(),
                    ));
                }
            }
            Stmt::Expr(expr, ..) => {
                self.infer_expr_type(expr);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Class { name, methods, .. } => {
                for m in methods {
                    if let Stmt::FnDef { name: mname, params, .. } = m {
                        let mangled = format!("{}_{}", name, mname);
                        let return_ty = self.functions.get(mname.as_str()).map(|(_, r)| r.clone()).unwrap_or(TypeAnnot::Void);
                        self.functions.insert(mangled, (params.clone(), return_ty));
                    }
                    self.check_stmt(m);
                }
            }
        }
    }

    fn types_match(&self, expected: &TypeAnnot, actual: &TypeAnnot) -> bool {
        if expected == actual {
            return true;
        }
        // String and Int are interchangeable (both intptr_t at runtime)
        if (*expected == TypeAnnot::Int && *actual == TypeAnnot::String)
            || (*expected == TypeAnnot::String && *actual == TypeAnnot::Int)
        {
            return true;
        }
        if let (TypeAnnot::Array(a), TypeAnnot::Array(b)) = (expected, actual) {
            return self.types_match(a, b);
        }
        if let (TypeAnnot::Class(a), TypeAnnot::Class(b)) = (expected, actual) {
            return a == b;
        }
        false
    }

    fn infer_expr_type(&mut self, expr: &Expr) -> TypeAnnot {
        match expr {
            Expr::Number(_, ..) => TypeAnnot::Int,
            Expr::StringLit(_, ..) => TypeAnnot::String,
            Expr::Bool(_, ..) => TypeAnnot::Bool,
            Expr::Ident(name, line, col) => {
                if let Some((_, ty)) = self.lookup_var(name) {
                    ty
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Undefined variable '{}'", name),
                    ));
                    TypeAnnot::Int
                }
            }
            Expr::Binary { left, right, op, line, col, .. } => {
                let lty = self.infer_expr_type(left);
                let rty = self.infer_expr_type(right);
                match op {
                    BinOp::Add => {
                        if lty == TypeAnnot::String && rty == TypeAnnot::String {
                            return TypeAnnot::String;
                        }
                        if lty != TypeAnnot::Int || rty != TypeAnnot::Int {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("Type mismatch: cannot add {:?} and {:?}", lty, rty),
                            ));
                        }
                        TypeAnnot::Int
                    }
                    BinOp::Sub | BinOp::Mul | BinOp::Div => {
                        if lty != TypeAnnot::Int || rty != TypeAnnot::Int {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("Type mismatch: {:?} expects Int operands, got {:?} and {:?}", op, lty, rty),
                            ));
                        }
                        TypeAnnot::Int
                    }
                    BinOp::Eq | BinOp::Neq => {
                        if !self.types_match(&lty, &rty) {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("Type mismatch: cannot compare {:?} and {:?}", lty, rty),
                            ));
                        }
                        TypeAnnot::Bool
                    }
                    BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                        if !self.types_match(&lty, &rty) {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("Type mismatch: cannot compare {:?} and {:?}", lty, rty),
                            ));
                        }
                        TypeAnnot::Bool
                    }
                    BinOp::And | BinOp::Or => {
                        // Ajeeb allows &&/|| on any type (truthy/falsy)
                        TypeAnnot::Bool
                    }
                }
            }
            Expr::UnaryNot(inner, ..) => {
                let _ity = self.infer_expr_type(inner);
                // Ajeeb allows '!' on any type (int = truthy, string = truthy, etc.)
                TypeAnnot::Bool
            }
            Expr::Group(inner, ..) => self.infer_expr_type(inner),
            Expr::Assign { name, value, line, col, .. } => {
                let val_ty = self.infer_expr_type(value);
                if let Some((_, existing_ty)) = self.lookup_var(name) {
                    if !self.types_match(&existing_ty, &val_ty) {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!(
                                "Type mismatch: cannot assign {:?} to variable '{}' of type {:?}",
                                val_ty, name, existing_ty
                            ),
                        ));
                    }
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Undefined variable '{}' in assignment", name),
                    ));
                }
                val_ty
            }
            Expr::FnCall { name, args, line, col, .. } => {
                let fn_info = self.functions.get(name).cloned();
                if let Some((params, return_ty)) = fn_info {
                    let pcount = params.len();
                    if pcount != args.len() {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!(
                                "Function '{}' expects {} arguments but got {}",
                                name,
                                pcount,
                                args.len()
                            ),
                        ));
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let arg_ty = self.infer_expr_type(arg);
                        if i < pcount {
                            let param_ty = &params[i].1;
                            if !self.types_match(param_ty, &arg_ty) {
                                self.errors.push(self.err(
                                    *line,
                                    *col,
                                    format!(
                                        "Type mismatch: argument {} of '{}' expected {:?} but got {:?}",
                                        i + 1, name, param_ty, arg_ty
                                    ),
                                ));
                            }
                        }
                    }
                    return_ty
                } else {
                    // Builtins that aren't in the function table
                    match name.as_str() {
                        "print" | "println" | "writeFile" | "writeAppend"
                        | "wrB" | "setInt" | "strcpy" | "strSet"
                        | "writeByte" | "wrPos" => TypeAnnot::Void,
                        "len" | "charCode" | "strcmp" | "strcmp_ajeeb"
                        | "rdB" | "getInt" | "rdPos" | "indexOf"
                        | "isDigit" | "isAlpha" | "isAlphaNum" | "isSpace" => TypeAnnot::Int,
                        "itoa" | "readFile" | "readArg" | "getStateBuf" | "getOutbuf"
                        | "substring" | "toUpperCase" | "toLowerCase"
                        | "trim" | "split" | "replace" => TypeAnnot::String,
                        "contains" | "startsWith" | "endsWith" => TypeAnnot::Bool,
                        "chr" => TypeAnnot::Int,
                        _ => {
                            // Unknown function/method — could be a class constructor
                            TypeAnnot::Int
                        }
                    }
                }
            }
            Expr::New { .. } => TypeAnnot::Int,
            Expr::ArrayLit(_, ..) => TypeAnnot::Array(Box::new(TypeAnnot::Int)),
            Expr::Index { obj, .. } => {
                let obj_ty = self.infer_expr_type(obj);
                match obj_ty {
                    TypeAnnot::Array(inner) => *inner,
                    TypeAnnot::String => TypeAnnot::Int,
                    _ => TypeAnnot::Int,
                }
            }
            Expr::IndexAssign { obj, value, .. } => {
                self.infer_expr_type(obj);
                self.infer_expr_type(value);
                TypeAnnot::Int
            }
            Expr::Field { obj, .. } => {
                self.infer_expr_type(obj);
                TypeAnnot::Int
            }
            Expr::FieldAssign { obj, value, .. } => {
                self.infer_expr_type(obj);
                self.infer_expr_type(value);
                TypeAnnot::Int
            }
        }
    }
}
