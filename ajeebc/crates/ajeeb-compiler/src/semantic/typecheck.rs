use crate::ast::*;
use crate::error::CompileError;
use std::collections::HashMap;
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Set { name, type_ann, value, line, col, .. }
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
            Stmt::FnDef { name, type_param_bounds, params, body, .. } => {
                self.enter_scope();
                let mut saved_bounds = Vec::new();
                for (param, bounds) in type_param_bounds {
                    let old = self.type_param_bounds.insert(param.clone(), bounds.clone());
                    saved_bounds.push((param.clone(), old));
                }
                for (pname, pty) in params {
                    if pname == "self" && matches!(pty, TypeAnnot::Void) {
                        if let Some((ref tn, _)) = self.current_impl {
                            self.declare_var(pname, TypeAnnot::Class(tn.clone()), 0, 0);
                        } else if let Some(ref cn) = self.current_class {
                            self.declare_var(pname, TypeAnnot::Class(cn.clone()), 0, 0);
                        } else {
                            self.declare_var(pname, pty.clone(), 0, 0);
                        }
                    } else {
                        self.declare_var(pname, pty.clone(), 0, 0);
                    }
                }
                // Auto-declare 'self' for class methods that don't have explicit self param
                if self.current_class.is_some() && !params.iter().any(|(n, _)| n == "self") {
                    if let Some(ref cn) = self.current_class {
                        self.declare_var("self", TypeAnnot::Class(cn.clone()), 0, 0);
                    }
                }
                let lookup_name = if let Some(ref class) = self.current_class {
                    format!("{}_{}", class, name)
                } else if let Some((ref tn, ref trait_n)) = self.current_impl {
                    if trait_n.is_empty() {
                        format!("{}_{}", tn, name)
                    } else {
                        format!("{}_{}_{}", tn, trait_n, name)
                    }
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
                for (param, old) in saved_bounds {
                    match old {
                        Some(v) => { self.type_param_bounds.insert(param, v); }
                        None => { self.type_param_bounds.remove(&param); }
                    }
                }
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
            Stmt::ImplBlock { trait_name, trait_type_args: _, type_params, type_param_bounds, type_name, methods, .. } => {
                let mut saved_bounds = Vec::new();
                for (tp, bounds) in type_param_bounds {
                    let old = self.type_param_bounds.insert(tp.clone(), bounds.clone());
                    saved_bounds.push((tp.clone(), old));
                }
                let base_type_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                if let Some(ref trait_name) = trait_name {
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
                    let old_impl = self.current_impl.replace((base_type_name.to_string(), trait_name.clone()));
                    for m in methods {
                        self.check_stmt(m);
                    }
                    self.current_impl = old_impl;
                } else {
                    let old_impl = self.current_impl.replace((base_type_name.to_string(), String::new()));
                    for m in methods {
                        self.check_stmt(m);
                    }
                    self.current_impl = old_impl;
                }
                for (tp, old_val) in saved_bounds {
                    match old_val {
                        Some(bounds) => { self.type_param_bounds.insert(tp, bounds); }
                        None => { self.type_param_bounds.remove(&tp); }
                    }
                }
            }
            Stmt::TraitDef { .. } | Stmt::StructDef { .. } | Stmt::EnumDef { .. } => {}
        }
    }

    pub(super) fn types_match(&self, expected: &TypeAnnot, actual: &TypeAnnot) -> bool {
        if expected == actual {
            return true;
        }
        if matches!(expected, TypeAnnot::Generic(_)) || matches!(actual, TypeAnnot::Generic(_)) {
            return true;
        }
        if (*expected == TypeAnnot::Int && *actual == TypeAnnot::Float)
            || (*expected == TypeAnnot::Float && *actual == TypeAnnot::Int)
            || (*expected == TypeAnnot::Int && *actual == TypeAnnot::String)
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
        if let (TypeAnnot::Parameterized { base, .. }, TypeAnnot::Class(_)) = (expected, actual) {
            return self.types_match(base, actual);
        }
        if let (TypeAnnot::Class(_), TypeAnnot::Parameterized { base, .. }) = (expected, actual) {
            return self.types_match(expected, base);
        }
        if let (TypeAnnot::Parameterized { base: b1, args: a1 }, TypeAnnot::Parameterized { base: b2, args: a2 }) = (expected, actual) {
            return self.types_match(b1, b2) && a1.len() == a2.len() && a1.iter().zip(a2).all(|(x, y)| self.types_match(x, y));
        }
        false
    }

    pub(super) fn infer_expr_type(&mut self, expr: &Expr) -> TypeAnnot {
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
            Expr::AssociatedFnCall { type_name, method, args, line, col } => {
                let base_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                let mangled = format!("{}_{}", base_name, method);
                if let Some((params, return_ty)) = self.functions.get(&mangled).cloned() {
                    let pcount = params.len();
                    if pcount != args.len() {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Associated function '{}' expects {} arguments but got {}", method, pcount, args.len()),
                        ));
                    }
                    return return_ty;
                }
                self.errors.push(self.err(
                    *line, *col,
                    format!("No associated function '{}' found for type '{}'", method, type_name),
                ));
                TypeAnnot::Int
            }
            Expr::MethodCall { obj, method, args, line, col } => {
                let obj_ty = self.infer_expr_type(obj);
                let type_name = match &obj_ty {
                    TypeAnnot::Class(n) => Some(n.clone()),
                    TypeAnnot::Parameterized { base, .. } => {
                        if let TypeAnnot::Class(n) = base.as_ref() { Some(n.clone()) } else { None }
                    }
                    _ => None,
                };
                if let Some(ref tn) = type_name {
                    let base_tn = if let Some(bracket_pos) = tn.find('[') {
                        &tn[..bracket_pos]
                    } else {
                        tn.as_str()
                    };
                    let mangled = format!("{}_{}", base_tn, method);
                    if let Some((params, return_ty)) = self.functions.get(&mangled).cloned() {
                        let pcount = params.len();
                        let expected_no_self = if pcount > 0 { pcount - 1 } else { 0 };
                        if pcount != args.len() + 1 {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Method '{}' expects {} arguments but got {}", method, expected_no_self, args.len()),
                            ));
                        }
                        return return_ty;
                    }
                    if let Some(impls) = self.impls.get(base_tn) {
                        for (trait_name, _, _) in impls {
                            let mangled = format!("{}_{}_{}", base_tn, trait_name, method);
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
                } else if let TypeAnnot::Generic(ref type_param) = obj_ty {
                    if let Some(bounds) = self.type_param_bounds.get(type_param).cloned() {
                        for trait_name in &bounds {
                            if let Some(trait_methods) = self.traits.get(trait_name) {
                                if let Some(trait_m) = trait_methods.iter().find(|tm| tm.name == *method) {
                                    let pcount = trait_m.params.len();
                                    if pcount != args.len() + 1 {
                                        self.errors.push(self.err(
                                            *line, *col,
                                            format!("Method '{}' (from trait '{}') expects {} arguments but got {}",
                                                method, trait_name, pcount - 1, args.len()),
                                        ));
                                    }
                                    return trait_m.return_type.clone();
                                }
                            }
                        }
                    }
                    self.errors.push(self.err(
                        *line, *col,
                        format!("No method '{}' found for generic type '{}'", method, type_param),
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
                let base_name = if let Some(bracket_pos) = struct_name.find('[') {
                    &struct_name[..bracket_pos]
                } else {
                    struct_name.as_str()
                };
                let got_args = Self::count_type_args_in_name(struct_name);
                if let Some(&expected_args) = self.struct_type_params.get(base_name) {
                    if expected_args > 0 {
                        if got_args == 0 {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic struct '{}' requires {} type argument(s) but got 0", base_name, expected_args),
                            ));
                        } else if got_args != expected_args {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic struct '{}' expects {} type argument(s) but got {}", base_name, expected_args, got_args),
                            ));
                        }
                    } else if got_args > 0 {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Struct '{}' does not accept type arguments", base_name),
                        ));
                    }
                } else if got_args > 0 {
                    self.errors.push(self.err(
                        *line, *col,
                        format!("Unknown struct '{}'", base_name),
                    ));
                }
                self.validate_struct_bounds(struct_name, *line, *col);
                let def_fields = self.struct_defs.get(base_name).cloned().unwrap_or_default();
                if !def_fields.is_empty() || self.struct_defs.contains_key(base_name) {
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
                    TypeAnnot::Class(base_name.to_string())
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
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    &enum_name[..bracket_pos]
                } else {
                    enum_name.as_str()
                };
                let got_args = Self::count_type_args_in_name(enum_name);
                if let Some(&expected_args) = self.enum_type_params.get(base_name) {
                    if expected_args > 0 {
                        if got_args == 0 {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic enum '{}' requires {} type argument(s) but got 0", base_name, expected_args),
                            ));
                        } else if got_args != expected_args {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic enum '{}' expects {} type argument(s) but got {}", base_name, expected_args, got_args),
                            ));
                        }
                    } else if got_args > 0 {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Enum '{}' does not accept type arguments", base_name),
                        ));
                    }
                } else if got_args > 0 {
                    self.errors.push(self.err(
                        *line, *col,
                        format!("Unknown enum '{}'", base_name),
                    ));
                }
                self.validate_enum_bounds(enum_name, *line, *col);
                if let Some(variants) = self.enum_defs.get(base_name) {
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
                TypeAnnot::Class(base_name.to_string())
            }
            Expr::EnumCtor { enum_name, variant, args, line, col } => {
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    &enum_name[..bracket_pos]
                } else {
                    enum_name.as_str()
                };
                let got_args = Self::count_type_args_in_name(enum_name);
                if let Some(&expected_args) = self.enum_type_params.get(base_name) {
                    if expected_args > 0 {
                        if got_args == 0 {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic enum '{}' requires {} type argument(s) but got 0", base_name, expected_args),
                            ));
                        } else if got_args != expected_args {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Generic enum '{}' expects {} type argument(s) but got {}", base_name, expected_args, got_args),
                            ));
                        }
                    } else if got_args > 0 {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Enum '{}' does not accept type arguments", base_name),
                        ));
                    }
                } else if got_args > 0 {
                    self.errors.push(self.err(
                        *line, *col,
                        format!("Unknown enum '{}'", base_name),
                    ));
                }
                self.validate_enum_bounds(enum_name, *line, *col);
                let var_def = self.enum_defs.get(base_name)
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
                let enum_name_opt = match &val_ty {
                    TypeAnnot::Class(name) => {
                        let base = if let Some(bracket_pos) = name.find('[') {
                            &name[..bracket_pos]
                        } else {
                            name.as_str()
                        };
                        Some(base.to_string())
                    }
                    TypeAnnot::Parameterized { base, .. } => {
                        if let TypeAnnot::Class(name) = base.as_ref() {
                            let base = if let Some(bracket_pos) = name.find('[') {
                                &name[..bracket_pos]
                            } else {
                                name.as_str()
                            };
                            Some(base.to_string())
                        } else { None }
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
                    match &arm.pattern {
                        Pattern::Wildcard => { has_wildcard = true; }
                        Pattern::EnumVariant { enum_name, variant, bindings } => {
                            let val_ty_base = match &val_ty {
                                TypeAnnot::Class(n) => {
                                    let base = if let Some(bracket_pos) = n.find('[') {
                                        &n[..bracket_pos]
                                    } else {
                                        n.as_str()
                                    };
                                    TypeAnnot::Class(base.to_string())
                                }
                                other => other.clone(),
                            };
                            let expected_ty = TypeAnnot::Class(enum_name.clone());
                            if !self.types_match(&val_ty_base, &expected_ty) {
                                self.errors.push(self.err(
                                    *line, *col,
                                    format!("Match value type {:?} does not match enum '{}'", val_ty, enum_name),
                                ));
                            }
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
                self.infer_fn_call_generic(name, type_args, args, *line, *col)
            }
        }
    }

    pub(super) fn class_fields_exist(&self, name: &str) -> bool {
        self.struct_defs.contains_key(name) || self.functions.contains_key(&format!("{}_new", name))
    }

    pub(super) fn infer_fn_call(&mut self, name: &str, args: &[Expr], line: usize, col: usize) -> TypeAnnot {
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
            for arg in args {
                self.infer_expr_type(arg);
            }
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
                | "isDigit" | "isAlpha" | "isAlphaNum" | "isSpace"
                | "exec" | "mkdir" => TypeAnnot::Int,
                "chr_str" | "itoa" | "readFile" | "readArg" | "getStateBuf" | "getOutbuf"
                | "substring" | "toUpperCase" | "toLowerCase"
                | "trim" | "replace" | "str_concat" | "getStr" => TypeAnnot::String,
                "split" => TypeAnnot::Array(Box::new(TypeAnnot::String)),
                "contains" | "startsWith" | "endsWith" => TypeAnnot::Bool,
                "chr" => TypeAnnot::String,
                _ => TypeAnnot::Int,
            }
        }
    }
}
