use crate::hir::*;
use std::collections::HashMap;

pub struct ThirChecker {
    errors: Vec<String>,
    fn_signatures: HashMap<String, (Vec<HirType>, HirType)>,
    struct_fields: HashMap<String, Vec<(String, HirType)>>,
    enum_variants: HashMap<String, Vec<(String, Vec<HirType>)>>,
    type_env: HashMap<String, HirType>,
}

impl ThirChecker {
    pub fn new() -> Self {
        ThirChecker {
            errors: Vec::new(),
            fn_signatures: HashMap::new(),
            struct_fields: HashMap::new(),
            enum_variants: HashMap::new(),
            type_env: HashMap::new(),
        }
    }

    pub fn check(&mut self, prog: &HirProgram) -> Vec<String> {
        self.errors.clear();

        // Register struct definitions
        for s in &prog.structs {
            self.struct_fields.insert(s.name.clone(), s.fields.clone());
        }

        // Register enum definitions
        for e in &prog.enums {
            self.enum_variants.insert(e.name.clone(), e.variants.clone());
        }

        // Register function signatures
        for f in &prog.functions {
            self.fn_signatures.insert(
                f.name.clone(),
                (f.params.iter().map(|(_, t)| t.clone()).collect(), f.return_type.clone()),
            );
        }

        // Register impl method signatures
        for imp in &prog.impls {
            for m in &imp.methods {
                let mangled = if let Some(ref trait_name) = imp.trait_name {
                    format!("{}_{}_{}", imp.type_name, trait_name, m.name)
                } else {
                    format!("{}_{}", imp.type_name, m.name)
                };
                self.fn_signatures.insert(
                    mangled,
                    (m.params.iter().map(|(_, t)| t.clone()).collect(), m.return_type.clone()),
                );
            }
        }

        // Check each function
        for f in &prog.functions {
            self.check_fn(f);
        }

        // Check impl methods
        for imp in &prog.impls {
            for m in &imp.methods {
                self.check_fn(m);
            }
        }

        self.errors.clone()
    }

    fn check_fn(&mut self, f: &HirFn) {
        self.type_env.clear();
        for (name, ty) in &f.params {
            self.type_env.insert(name.clone(), ty.clone());
        }
        for stmt in &f.body {
            self.check_stmt(stmt, &f.return_type);
        }
    }

    fn check_stmt(&mut self, stmt: &HirStmt, expected_return: &HirType) {
        match stmt {
            HirStmt::Set { name, ty, value } => {
                self.check_expr(value);
                if !ty.is_unknown() && !value.ty().is_unknown() && !ty.is_compatible_with(value.ty()) {
                    self.errors.push(format!(
                        "Type mismatch: expected {}, got {} — variable '{}'",
                        ty, value.ty(), name
                    ));
                }
                self.type_env.insert(name.clone(), ty.clone());
            }
            HirStmt::Return(expr) => {
                self.check_expr(expr);
                if !expected_return.is_void() && !expected_return.is_unknown() && !expr.ty().is_unknown() {
                    if !expected_return.is_compatible_with(expr.ty()) {
                        self.errors.push(format!(
                            "Return type mismatch: expected {}, got {}",
                            expected_return, expr.ty()
                        ));
                    }
                }
            }
            HirStmt::If { cond, then, else_ } => {
                self.check_expr(cond);
                for s in then {
                    self.check_stmt(s, expected_return);
                }
                for s in else_ {
                    self.check_stmt(s, expected_return);
                }
            }
            HirStmt::While { cond, body } => {
                self.check_expr(cond);
                for s in body {
                    self.check_stmt(s, expected_return);
                }
            }
            HirStmt::For { init, cond, update, body } => {
                self.check_stmt(init, expected_return);
                self.check_expr(cond);
                self.check_stmt(update, expected_return);
                for s in body {
                    self.check_stmt(s, expected_return);
                }
            }
            HirStmt::Expr(expr) => {
                self.check_expr(expr);
            }
            HirStmt::Break | HirStmt::Continue => {}
        }
    }

    fn check_expr(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Var { name, ty } => {
                if ty.is_unknown() {
                    if let Some(known) = self.type_env.get(name) {
                        // Type was known from context
                        let _ = known;
                    }
                }
            }
            HirExpr::BinOp { op, left, right, ty: _ } => {
                self.check_expr(left);
                self.check_expr(right);
                // Type compatibility check
                if !left.ty().is_unknown() && !right.ty().is_unknown() {
                    match op {
                        HirBinOp::Add | HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Div => {
                            if !left.ty().is_compatible_with(right.ty()) {
                                self.errors.push(format!(
                                    "Type mismatch: cannot apply {:?} to {} and {}",
                                    op, left.ty(), right.ty()
                                ));
                            }
                        }
                        _ => {}
                    }
                }
            }
            HirExpr::Call { name, args, ty: _ } => {
                for arg in args {
                    self.check_expr(arg);
                }
                if let Some((params, _)) = self.fn_signatures.get(name) {
                    if params.len() != args.len() {
                        self.errors.push(format!(
                            "Wrong arg count: '{}' expects {} args, got {}",
                            name, params.len(), args.len()
                        ));
                    }
                }
            }
            HirExpr::MethodCall { receiver, method, args, ty: _ } => {
                self.check_expr(receiver);
                for arg in args {
                    self.check_expr(arg);
                }
                // Check method exists on type
                if let HirType::Named(type_name) = receiver.ty() {
                    let inherent = format!("{}_{}", type_name, method);
                    // Also check trait impl patterns: type_trait_method
                    let found = self.fn_signatures.contains_key(&inherent)
                        || self.fn_signatures.keys().any(|k| {
                            k.starts_with(&format!("{}_", type_name))
                                && k.ends_with(&format!("_{}", method))
                                && k.len() > inherent.len()
                        });
                    if !found {
                        // Also check struct fields (field access as method)
                        if let Some(fields) = self.struct_fields.get(type_name) {
                            if !fields.iter().any(|(n, _)| n == method) {
                                self.errors.push(format!(
                                    "No method '{}' found for type '{}'",
                                    method, type_name
                                ));
                            }
                        } else {
                            self.errors.push(format!(
                                "No method '{}' found for type '{}'",
                                method, type_name
                            ));
                        }
                    }
                }
            }
            HirExpr::FieldAccess { obj, field, ty: _ } => {
                self.check_expr(obj);
                if let HirType::Named(type_name) = obj.ty() {
                    if let Some(fields) = self.struct_fields.get(type_name) {
                        if !fields.iter().any(|(n, _)| n == field) {
                            self.errors.push(format!(
                                "No field '{}' on struct '{}'",
                                field, type_name
                            ));
                        }
                    }
                }
            }
            HirExpr::FieldAssign { obj, field, value, ty: _ } => {
                self.check_expr(obj);
                self.check_expr(value);
                if let HirType::Named(type_name) = obj.ty() {
                    if let Some(fields) = self.struct_fields.get(type_name) {
                        if !fields.iter().any(|(n, _)| n == field) {
                            self.errors.push(format!(
                                "No field '{}' on struct '{}'",
                                field, type_name
                            ));
                        }
                    }
                }
            }
            HirExpr::StructLit { name, fields, ty: _ } => {
                for (_, val) in fields {
                    self.check_expr(val);
                }
                if let Some(def_fields) = self.struct_fields.get(name) {
                    if def_fields.len() != fields.len() {
                        self.errors.push(format!(
                            "Struct '{}' expects {} fields, got {}",
                            name, def_fields.len(), fields.len()
                        ));
                    }
                    for (fname, fval) in fields {
                        if let Some((_, expected_ty)) = def_fields.iter().find(|(n, _)| n == fname) {
                            if !expected_ty.is_unknown() && !fval.ty().is_unknown() {
                                if !expected_ty.is_compatible_with(fval.ty()) {
                                    self.errors.push(format!(
                                        "Type mismatch in struct '{}': field '{}' expects {}, got {}",
                                        name, fname, expected_ty, fval.ty()
                                    ));
                                }
                            }
                        } else {
                            self.errors.push(format!(
                                "No field '{}' in struct '{}'",
                                fname, name
                            ));
                        }
                    }
                }
            }
            HirExpr::EnumCtor { enum_name, variant, args, ty: _ } => {
                for arg in args {
                    self.check_expr(arg);
                }
                if let Some(variants) = self.enum_variants.get(enum_name) {
                    if let Some((_, expected_args)) = variants.iter().find(|(n, _)| n == variant) {
                        if expected_args.len() != args.len() {
                            self.errors.push(format!(
                                "Enum variant '{}::{}' expects {} args, got {}",
                                enum_name, variant, expected_args.len(), args.len()
                            ));
                        }
                    } else {
                        self.errors.push(format!(
                            "Unknown variant '{}' in enum '{}'",
                            variant, enum_name
                        ));
                    }
                }
            }
            HirExpr::Index { obj, idx, ty: _ } => {
                self.check_expr(obj);
                self.check_expr(idx);
            }
            HirExpr::IndexAssign { obj, idx, value, ty: _ } => {
                self.check_expr(obj);
                self.check_expr(idx);
                self.check_expr(value);
            }
            HirExpr::ArrayLit { elems, ty: _ } => {
                for elem in elems {
                    self.check_expr(elem);
                }
            }
            HirExpr::UnaryMinus(inner, _) | HirExpr::UnaryNot(inner, _) => {
                self.check_expr(inner);
            }
            HirExpr::Assign { name, value, ty: _ } => {
                self.check_expr(value);
                if !self.type_env.contains_key(name) {
                    self.errors.push(format!(
                        "Unknown variable: '{}' — declare karo pehle", name
                    ));
                }
            }
            HirExpr::Int(_) | HirExpr::Float(_) | HirExpr::Str(_) | HirExpr::Bool(_) => {}
        }
    }
}

impl HirType {
    pub fn is_void(&self) -> bool {
        matches!(self, HirType::Void)
    }
}
