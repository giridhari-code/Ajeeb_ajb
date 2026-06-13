use crate::ast::*;
use crate::error::CompileError;
use std::collections::HashMap;

pub struct SemanticAnalyzer {
    pub errors: Vec<CompileError>,
    scopes: Vec<HashMap<String, TypeAnnot>>,
    functions: HashMap<String, (Vec<(String, TypeAnnot)>, TypeAnnot)>,
    struct_defs: HashMap<String, Vec<(String, TypeAnnot)>>,
    enum_defs: HashMap<String, Vec<EnumVariantDef>>,
    traits: HashMap<String, Vec<TraitMethod>>,
    impls: HashMap<String, Vec<(String, Vec<Stmt>)>>, // type_name -> [(trait_name, methods)]
    current_function: Option<(String, TypeAnnot)>,
    current_class: Option<String>,
    current_impl: Option<(String, String)>, // (type_name, trait_name)
    module_prefix: String,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut global = HashMap::new();
        for (name, ty) in builtin_functions() {
            global.insert(name.to_string(), ty.clone());
        }
        SemanticAnalyzer {
            errors: Vec::new(),
            scopes: vec![global],
            functions: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            current_function: None,
            current_class: None,
            current_impl: None,
            module_prefix: String::new(),
        }
    }

    pub fn set_module_prefix(&mut self, prefix: &str) {
        self.module_prefix = prefix.to_string();
    }

    pub fn analyze(&mut self, program: &[Stmt]) {
        // First pass: collect all function signatures, class methods, structs, enums
        for stmt in program {
            match stmt {
                Stmt::FnDef { name, params, return_type, line, col, .. } => {
                    let fq_name = self.qualify(name);
                    if self.functions.contains_key(&fq_name) {
                        self.errors.push(CompileError::new(
                            *line,
                            *col,
                            format!("Duplicate function '{}' is already defined", fq_name),
                        ));
                    }
                    self.functions.insert(fq_name, (params.clone(), return_type.clone()));
                }
                Stmt::Class { name, methods, .. } => {
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                            let mangled = format!("{}_{}", name, mname);
                            self.functions.insert(mangled, (params.clone(), return_type.clone()));
                        }
                    }
                }
                Stmt::StructDef { name, fields, .. } => {
                    let ft: Vec<(String, TypeAnnot)> = fields.iter().map(|f| (f.name.clone(), f.type_ann.clone())).collect();
                    self.struct_defs.insert(name.clone(), ft);
                }
                Stmt::EnumDef { name, variants, .. } => {
                    self.enum_defs.insert(name.clone(), variants.clone());
                }
                Stmt::TraitDef { name, methods, .. } => {
                    self.traits.insert(name.clone(), methods.clone());
                }
                Stmt::ImplBlock { trait_name, type_name, methods, line, col } => {
                    // Check trait exists
                    if !self.traits.contains_key(trait_name) {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Unknown trait '{}' in impl", trait_name),
                        ));
                    }
                    // Check type exists
                    if !self.struct_defs.contains_key(type_name) && !self.enum_defs.contains_key(type_name) && !self.functions.contains_key(&format!("{}_new", type_name)) {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Unknown type '{}' in impl", type_name),
                        ));
                    }
                    // Register impl methods with mangled names for function lookup
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                            let mangled = format!("{}_{}_{}", type_name, trait_name, mname);
                            self.functions.insert(mangled, (params.clone(), return_type.clone()));
                        }
                    }
                    self.impls.entry(type_name.clone())
                        .or_default()
                        .push((trait_name.clone(), methods.clone()));
                }
                _ => {}
            }
        }

        // Second pass: register imported module functions
        for stmt in program {
            if let Stmt::Import(import) = stmt {
                let prefix = import.alias.clone().unwrap_or_else(|| {
                    import.path.last().cloned().unwrap_or_default()
                });
                let fn_keys: Vec<String> = self.functions.keys()
                    .filter(|k| k.starts_with(&format!("{}::", prefix)) || k.starts_with(&format!("{}_", prefix)))
                    .cloned()
                    .collect();
                for key in fn_keys {
                    if let Some((_params, ret)) = self.functions.get(&key).cloned() {
                        let local_name = key.split("::").last().unwrap_or(&key).to_string();
                        if !self.scopes[0].contains_key(&local_name) {
                            self.scopes[0].insert(local_name, ret);
                        }
                    }
                }
            }
        }

        // Third pass: check top-level statements
        for stmt in program {
            self.check_stmt(stmt);
        }
    }

    fn qualify(&self, name: &str) -> String {
        if self.module_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.module_prefix, name)
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
            Stmt::Let { name, type_ann, value, line, col, .. }
            | Stmt::Const { name, type_ann, value, line, col, .. } => {
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
                // Look up return type: try mangled name (class/impl method) or qualified name
                let lookup_name = if let Some(ref class) = self.current_class {
                    format!("{}_{}", class, name)
                } else if let Some((ref tn, ref trait_n)) = self.current_impl {
                    format!("{}_{}_{}", tn, trait_n, name)
                } else {
                    self.qualify(name)
                };
                let return_ty = self.functions.get(&lookup_name)
                    .map(|(_, r)| r.clone())
                    .unwrap_or(TypeAnnot::Void);
                let prev_fn = self.current_function.replace((lookup_name, return_ty));
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
                                "Return type mismatch in '{}': expected {:?} but got {:?}",
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
            Stmt::Import(_) => {}
            Stmt::Class { name, methods, .. } => {
                let old_class = self.current_class.replace(name.clone());
                for m in methods {
                    if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                        let mangled = format!("{}_{}", name, mname);
                        self.functions.insert(mangled, (params.clone(), return_type.clone()));
                    }
                    self.check_stmt(m);
                }
                self.current_class = old_class;
            }
            Stmt::ImplBlock { trait_name, type_name, methods, .. } => {
                // Check that all required trait methods are implemented
                if let Some(trait_methods) = self.traits.get(trait_name) {
                    let impl_method_names: Vec<&str> = methods.iter().filter_map(|m| {
                        if let Stmt::FnDef { name, .. } = m { Some(name.as_str()) } else { None }
                    }).collect();
                    for tm in trait_methods {
                        if !impl_method_names.contains(&tm.name.as_str()) {
                            self.errors.push(self.err(
                                0, 0,
                                format!("Impl for '{}' on '{}' is missing method '{}'", trait_name, type_name, tm.name),
                            ));
                        }
                    }
                }
                // Check method bodies with impl context set
                let old_impl = self.current_impl.replace((type_name.clone(), trait_name.clone()));
                for m in methods {
                    self.check_stmt(m);
                }
                self.current_impl = old_impl;
            }
            Stmt::TraitDef { .. } | Stmt::StructDef { .. } | Stmt::EnumDef { .. } => {
                // Already collected in first pass; nothing to check at statement level
            }
        }
    }

    fn types_match(&self, expected: &TypeAnnot, actual: &TypeAnnot) -> bool {
        if expected == actual {
            return true;
        }
        // Generic type parameters match anything
        if matches!(expected, TypeAnnot::Generic(_)) || matches!(actual, TypeAnnot::Generic(_)) {
            return true;
        }
        if (*expected == TypeAnnot::Int && *actual == TypeAnnot::String)
            || (*expected == TypeAnnot::String && *actual == TypeAnnot::Int)
        {
            return true;
        }
        if (*expected == TypeAnnot::Int && *actual == TypeAnnot::Float)
            || (*expected == TypeAnnot::Float && *actual == TypeAnnot::Int)
        {
            return true;
        }
        if let (TypeAnnot::Array(a), TypeAnnot::Array(b)) = (expected, actual) {
            return self.types_match(a, b);
        }
        if let (TypeAnnot::Class(a), TypeAnnot::Class(b)) = (expected, actual) {
            return a == b;
        }
        if let (TypeAnnot::Class(a), TypeAnnot::Int) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Int, TypeAnnot::Class(a)) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Class(a), TypeAnnot::String) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::String, TypeAnnot::Class(a)) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Class(a), TypeAnnot::Array(_)) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Array(_), TypeAnnot::Class(a)) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Class(a), TypeAnnot::Bool) = (expected, actual) {
            return a == "Array";
        }
        if let (TypeAnnot::Bool, TypeAnnot::Class(a)) = (expected, actual) {
            return a == "Array";
        }
        // Parameterized types match their base class
        if let (TypeAnnot::Parameterized { base, .. }, TypeAnnot::Class(_)) = (expected, actual) {
            return self.types_match(base, actual);
        }
        if let (TypeAnnot::Class(_), TypeAnnot::Parameterized { base, .. }) = (expected, actual) {
            return self.types_match(expected, base);
        }
        // Parameterized types match if base and args match
        if let (TypeAnnot::Parameterized { base: b1, args: a1 }, TypeAnnot::Parameterized { base: b2, args: a2 }) = (expected, actual) {
            return self.types_match(b1, b2) && a1.len() == a2.len() && a1.iter().zip(a2).all(|(x, y)| self.types_match(x, y));
        }
        false
    }

    fn infer_expr_type(&mut self, expr: &Expr) -> TypeAnnot {
        match expr {
            Expr::Number(_, ..) => TypeAnnot::Int,
            Expr::FloatLit(_, ..) => TypeAnnot::Float,
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
                        if lty == TypeAnnot::Float || rty == TypeAnnot::Float {
                            return TypeAnnot::Float;
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
                        if lty == TypeAnnot::Float || rty == TypeAnnot::Float {
                            return TypeAnnot::Float;
                        }
                        if lty != TypeAnnot::Int || rty != TypeAnnot::Int {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("{:?} expects Int/Float operands, got {:?} and {:?}", op, lty, rty),
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
                        TypeAnnot::Bool
                    }
                }
            }
            Expr::UnaryMinus(inner, ..) => {
                let ity = self.infer_expr_type(inner);
                if ity == TypeAnnot::Float {
                    TypeAnnot::Float
                } else {
                    TypeAnnot::Int
                }
            }
            Expr::UnaryNot(inner, ..) => {
                let _ity = self.infer_expr_type(inner);
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
                self.infer_fn_call(name, args, *line, *col)
            }
            Expr::MethodCall { obj, method, args, line, col } => {
                let obj_ty = self.infer_expr_type(obj);
                let type_name = match &obj_ty {
                    TypeAnnot::Class(n) => Some(n.clone()),
                    _ => None,
                };
                if let Some(ref tn) = type_name {
                    // Check class methods first
                    let mangled = format!("{}_{}", tn, method);
                    if let Some((params, return_ty)) = self.functions.get(&mangled).cloned() {
                        let pcount = params.len();
                        if pcount != args.len() + 1 {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Method '{}' expects {} arguments but got {}", method, pcount - 1, args.len()),
                            ));
                        }
                        return return_ty;
                    }
                    // Check trait impl methods
                    if let Some(impls) = self.impls.get(tn) {
                        for (trait_name, _) in impls {
                            let mangled = format!("{}_{}_{}", tn, trait_name, method);
                            if let Some((params, return_ty)) = self.functions.get(&mangled).cloned() {
                                let pcount = params.len();
                                if pcount != args.len() + 1 {
                                    self.errors.push(self.err(
                                        *line, *col,
                                        format!("Method '{}' (from trait '{}') expects {} arguments but got {}",
                                            method, trait_name, pcount - 1, args.len()),
                                    ));
                                }
                                return return_ty;
                            }
                        }
                    }
                    self.errors.push(self.err(
                        *line, *col,
                        format!("No method '{}' found for type '{}'", method, tn),
                    ));
                } else {
                    self.errors.push(self.err(
                        *line, *col,
                        format!("Method call on non-class/struct/enum type {:?}", obj_ty),
                    ));
                }
                TypeAnnot::Int
            }
            Expr::New { class_name, line, col, .. } => {
                if !self.class_fields_exist(class_name) {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Unknown class '{}'", class_name),
                    ));
                }
                TypeAnnot::Class(class_name.clone())
            }
            Expr::ArrayLit(_, ..) => TypeAnnot::Array(Box::new(TypeAnnot::Int)),
            Expr::Index { obj, .. } => {
                let obj_ty = self.infer_expr_type(obj);
                match obj_ty {
                    TypeAnnot::Array(inner) => *inner,
                    TypeAnnot::String => TypeAnnot::Int,
                    TypeAnnot::Class(ref n) if n == "Array" => TypeAnnot::Class("Array".to_string()),
                    _ => TypeAnnot::Int,
                }
            }
            Expr::IndexAssign { obj, value, .. } => {
                self.infer_expr_type(obj);
                self.infer_expr_type(value);
                TypeAnnot::Int
            }
            Expr::Field { obj, field, line, col } => {
                let obj_ty = self.infer_expr_type(obj);
                match &obj_ty {
                    TypeAnnot::Class(struct_name) => {
                        if let Some(fields) = self.struct_defs.get(struct_name) {
                            for (fname, fty) in fields {
                                if fname == field {
                                    return fty.clone();
                                }
                            }
                        }
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!("Struct '{}' has no field '{}'", struct_name, field),
                        ));
                        TypeAnnot::Int
                    }
                    _ => TypeAnnot::Int,
                }
            }
            Expr::FieldAssign { obj, value, .. } => {
                self.infer_expr_type(obj);
                self.infer_expr_type(value);
                TypeAnnot::Int
            }
            Expr::StructLit { struct_name, fields, line, col } => {
                let def_fields = self.struct_defs.get(struct_name).cloned().unwrap_or_default();
                if !def_fields.is_empty() || self.struct_defs.contains_key(struct_name) {
                    for (fname, fexpr) in fields {
                        let fty = self.infer_expr_type(fexpr);
                        let mut found = false;
                        for (dfname, dfty) in &def_fields {
                            if dfname == fname {
                                if !self.types_match(dfty, &fty) {
                                    self.errors.push(self.err(
                                        *line,
                                        *col,
                                        format!("Struct field '{}' expects {:?} but got {:?}", fname, dfty, fty),
                                    ));
                                }
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!("Struct '{}' has no field '{}'", struct_name, fname),
                            ));
                        }
                    }
                    TypeAnnot::Class(struct_name.clone())
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Unknown struct '{}'", struct_name),
                    ));
                    TypeAnnot::Int
                }
            }
            Expr::EnumRef { enum_name, variant, line, col } => {
                if let Some(variants) = self.enum_defs.get(enum_name) {
                    let found = variants.iter().any(|v| v.name == *variant);
                    if !found {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!("Unknown variant '{}' in enum '{}'", variant, enum_name),
                        ));
                    }
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Unknown enum '{}'", enum_name),
                    ));
                }
                TypeAnnot::Class(enum_name.clone())
            }
            Expr::EnumCtor { enum_name, variant, args, line, col } => {
                let var_def = self.enum_defs.get(enum_name)
                    .and_then(|variants| variants.iter().find(|v| v.name == *variant))
                    .cloned();
                if let Some(var_def) = &var_def {
                    if var_def.fields.len() != args.len() {
                        self.errors.push(self.err(
                            *line,
                            *col,
                            format!(
                                "Enum variant '{}::{}' expects {} fields but got {}",
                                enum_name, variant, var_def.fields.len(), args.len()
                            ),
                        ));
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let arg_ty = self.infer_expr_type(arg);
                        if i < var_def.fields.len() && !self.types_match(&var_def.fields[i], &arg_ty) {
                            self.errors.push(self.err(
                                *line,
                                *col,
                                format!(
                                    "Enum variant '{}::{}' field {} expects {:?} but got {:?}",
                                    enum_name, variant, i + 1, var_def.fields[i], arg_ty
                                ),
                            ));
                        }
                    }
                } else {
                    self.errors.push(self.err(
                        *line,
                        *col,
                        format!("Unknown variant '{}' in enum '{}'", variant, enum_name),
                    ));
                }
                TypeAnnot::Class(enum_name.clone())
            }
            Expr::Match { value, arms, line, col } => {
                let val_ty = self.infer_expr_type(value);
                // Pre-compute enum info for variant field types
                let enum_name_opt = match &val_ty {
                    TypeAnnot::Class(name) => Some(name.clone()),
                    TypeAnnot::Parameterized { base, .. } => {
                        if let TypeAnnot::Class(name) = base.as_ref() { Some(name.clone()) } else { None }
                    }
                    _ => None,
                };
                let enum_variants = enum_name_opt.as_ref()
                    .and_then(|name| self.enum_defs.get(name))
                    .cloned();
                let mut covered_variants: Vec<String> = Vec::new();
                let mut has_wildcard = false;
                let mut result_ty = TypeAnnot::Void;
                for arm in arms {
                    self.enter_scope();
                    // Check pattern and declare bindings
                    match &arm.pattern {
                        Pattern::Wildcard => { has_wildcard = true; }
                        Pattern::EnumVariant { enum_name, variant, bindings } => {
                            let expected_ty = TypeAnnot::Class(enum_name.clone());
                            if !self.types_match(&val_ty, &expected_ty) {
                                self.errors.push(self.err(
                                    *line, *col,
                                    format!("Match value type {:?} does not match enum '{}'", val_ty, enum_name),
                                ));
                            }
                            // Check variant exists and declare bindings
                            if let Some(ref variants) = enum_variants {
                                let found = variants.iter().any(|v| v.name == *variant);
                                if !found {
                                    self.errors.push(self.err(
                                        *line, *col,
                                        format!("Unknown variant '{}' in enum '{}'", variant, enum_name),
                                    ));
                                }
                                if let Some(var_def) = variants.iter().find(|v| v.name == *variant) {
                                    for (i, bname) in bindings.iter().enumerate() {
                                        if i < var_def.fields.len() {
                                            self.declare_var(bname, var_def.fields[i].clone(), *line, *col);
                                        }
                                    }
                                }
                            }
                            covered_variants.push(variant.clone());
                        }
                        Pattern::Int(_) => {}
                        Pattern::String(_) => {}
                    }
                    // Check arm body
                    let arm_ty = if let Some(stmts) = &arm.body_block {
                        for s in stmts {
                            self.check_stmt(s);
                        }
                        stmts.iter().rev().find_map(|s| {
                            if let Stmt::Expr(e, ..) = s { Some(self.infer_expr_type(e)) } else { None }
                        }).unwrap_or(TypeAnnot::Void)
                    } else {
                        self.infer_expr_type(&arm.body)
                    };
                    if matches!(result_ty, TypeAnnot::Void) {
                        result_ty = arm_ty;
                    }
                    self.exit_scope();
                }
                // Exhaustiveness check
                if let (Some(ref enum_name), Some(ref variants)) = (enum_name_opt, enum_variants) {
                    if !has_wildcard {
                        for v in variants {
                            if !covered_variants.contains(&v.name) {
                                self.errors.push(self.err(
                                    *line, *col,
                                    format!("Non-exhaustive match: missing variant '{}::{}'", enum_name, v.name),
                                ));
                            }
                        }
                    }
                }
                result_ty
            }
            Expr::GenericCall { name, type_args, args, line, col } => {
                // For now, check the call with the original type signature (Generic params match anything)
                self.infer_fn_call_generic(name, type_args, args, *line, *col)
            }
        }
    }

    fn class_fields_exist(&self, name: &str) -> bool {
        // Check both class and struct definitions
        self.struct_defs.contains_key(name) || self.functions.contains_key(&format!("{}_new", name))
    }

    fn infer_fn_call(&mut self, name: &str, args: &[Expr], line: usize, col: usize) -> TypeAnnot {
        if let Some((params, return_ty)) = self.functions.get(name).cloned() {
            let pcount = params.len();
            if pcount != args.len() {
                self.errors.push(self.err(
                    line,
                    col,
                    format!(
                        "Function '{}' expects {} arguments but got {}",
                        name, pcount, args.len()
                    ),
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_ty = self.infer_expr_type(arg);
                if i < pcount {
                    let param_ty = &params[i].1;
                    if !self.types_match(param_ty, &arg_ty) {
                        self.errors.push(self.err(
                            line,
                            col,
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
            match name {
                "tcp_listen" | "tcp_accept" | "tcp_connect" | "tls_connect"
                | "sqlite_open" | "sqlite_exec" | "now_ms" => TypeAnnot::Int,
                "dns_lookup" | "tls_read" => TypeAnnot::String,
                "tls_write" | "tls_close" => TypeAnnot::Void,
                "tcp_read" | "sqlite_last_error" => TypeAnnot::String,
                "tcp_write" | "tcp_close" | "sqlite_close" => TypeAnnot::Void,
                "sqlite_query" => TypeAnnot::Array(Box::new(TypeAnnot::Int)),
                "call_fn" => TypeAnnot::Int,
                "print" | "println" | "writeFile" | "writeAppend"
                | "wrB" | "setInt" | "strcpy" | "strSet"
                | "writeByte" | "wrPos" => TypeAnnot::Void,
                "len" | "arr_len" | "charCode" | "strcmp" | "strcmp_ajeeb"
                | "rdB" | "getInt" | "rdPos" | "indexOf"
                | "isDigit" | "isAlpha" | "isAlphaNum" | "isSpace" => TypeAnnot::Int,
                "chr_str" | "itoa" | "readFile" | "readArg" | "getStateBuf" | "getOutbuf"
                | "substring" | "toUpperCase" | "toLowerCase"
                | "trim" | "replace" | "str_concat" => TypeAnnot::String,
                "split" => TypeAnnot::Array(Box::new(TypeAnnot::String)),
                "contains" | "startsWith" | "endsWith" => TypeAnnot::Bool,
                "chr" => TypeAnnot::Int,
                _ => TypeAnnot::Int,
            }
        }
    }

    fn infer_fn_call_generic(&mut self, name: &str, type_args: &[TypeAnnot], args: &[Expr], line: usize, col: usize) -> TypeAnnot {
        if let Some((params, return_ty)) = self.functions.get(name).cloned() {
            let type_params: Vec<String> = params.iter()
                .filter_map(|(_, t)| if let TypeAnnot::Generic(s) = t { Some(s.clone()) } else { None })
                .collect();
            // Build type substitution map: T -> Int etc.
            let mut subst = std::collections::HashMap::new();
            for (i, tp) in type_params.iter().enumerate() {
                if i < type_args.len() {
                    subst.insert(tp.clone(), type_args[i].clone());
                }
            }
            // For now, just use original types (Generic matches everything via types_match)
            // Count check
            let pcount = params.len();
            if pcount != args.len() {
                self.errors.push(self.err(
                    line, col,
                    format!("Function '{}' expects {} arguments but got {}", name, pcount, args.len()),
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_ty = self.infer_expr_type(arg);
                if i < pcount {
                    let param_ty = &params[i].1;
                    if !self.types_match(param_ty, &arg_ty) {
                        self.errors.push(self.err(
                            line, col,
                            format!("Type mismatch: argument {} of '{}' expected {:?} but got {:?}", i + 1, name, param_ty, arg_ty),
                        ));
                    }
                }
            }
            return_ty
        } else {
            self.infer_fn_call(name, args, line, col)
        }
    }
}

fn builtin_functions() -> Vec<(&'static str, TypeAnnot)> {
    vec![
    ("print", TypeAnnot::Void),
    ("println", TypeAnnot::Void),
    ("arr_len", TypeAnnot::Int),
    ("tcp_listen", TypeAnnot::Int),
    ("tcp_accept", TypeAnnot::Int),
    ("tcp_read", TypeAnnot::String),
    ("tcp_write", TypeAnnot::Void),
    ("tcp_close", TypeAnnot::Void),
    ("tcp_connect", TypeAnnot::Int),
    ("dns_lookup", TypeAnnot::String),
    ("tls_connect", TypeAnnot::Int),
    ("tls_read", TypeAnnot::String),
    ("tls_write", TypeAnnot::Void),
    ("tls_close", TypeAnnot::Void),
    ("now_ms", TypeAnnot::Int),
    ("sqlite_open", TypeAnnot::Int),
    ("sqlite_close", TypeAnnot::Void),
    ("sqlite_exec", TypeAnnot::Int),
    ("sqlite_query", TypeAnnot::Array(Box::new(TypeAnnot::Int))),
	("sqlite_last_error", TypeAnnot::String),
	("itoa", TypeAnnot::String),
	("lib_open", TypeAnnot::Int),
	("lib_sym", TypeAnnot::Int),
	("lib_call", TypeAnnot::Int),
	("call_fn", TypeAnnot::Int),
	("assert_eq", TypeAnnot::Void),
	("assert_neq", TypeAnnot::Void),
	("assert_contains", TypeAnnot::Void),
	("len", TypeAnnot::Int),
    ("charCode", TypeAnnot::Int),
    ("strcmp", TypeAnnot::Int),
    ("readFile", TypeAnnot::String),
    ("writeFile", TypeAnnot::Void),
    ("writeAppend", TypeAnnot::Void),
    ("readArg", TypeAnnot::String),
    ("substring", TypeAnnot::String),
    ("indexOf", TypeAnnot::Int),
    ("contains", TypeAnnot::Bool),
    ("toUpperCase", TypeAnnot::String),
    ("toLowerCase", TypeAnnot::String),
    ("trim", TypeAnnot::String),
    ("split", TypeAnnot::Array(Box::new(TypeAnnot::String))),
    ("replace", TypeAnnot::String),
    ("startsWith", TypeAnnot::Bool),
    ("endsWith", TypeAnnot::Bool),
    ("getStateBuf", TypeAnnot::String),
    ("getOutbuf", TypeAnnot::String),
    ("rdB", TypeAnnot::Int),
    ("getInt", TypeAnnot::Int),
    ("wrB", TypeAnnot::Void),
    ("setInt", TypeAnnot::Void),
    ("strcpy", TypeAnnot::Void),
    ("strSet", TypeAnnot::Void),
    ("chr", TypeAnnot::Int),
    ("chr_str", TypeAnnot::String),
    ("writeByte", TypeAnnot::Void),
    ("rdPos", TypeAnnot::Int),
    ("wrPos", TypeAnnot::Void),
    ("isDigit", TypeAnnot::Bool),
    ("isAlpha", TypeAnnot::Bool),
    ("isAlphaNum", TypeAnnot::Bool),
    ("isSpace", TypeAnnot::Bool),
    ("strcmp_ajeeb", TypeAnnot::Int),
    ("str_concat", TypeAnnot::String),
]
}
