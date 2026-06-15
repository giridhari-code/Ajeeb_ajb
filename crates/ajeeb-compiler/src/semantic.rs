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
    impls: HashMap<String, Vec<(String, Vec<String>, Vec<Stmt>)>>, // type_name -> [(trait_name, trait_type_args, methods)]
    current_function: Option<(String, TypeAnnot)>,
    current_class: Option<String>,
    current_impl: Option<(String, String)>, // (type_name, trait_name)
    module_prefix: String,
    type_param_bounds: HashMap<String, Vec<String>>, // generic param -> trait bounds
    fn_generic_params: HashMap<String, Vec<String>>, // fn name -> generic param names
    fn_generic_bounds: HashMap<String, Vec<(String, Vec<String>)>>, // fn name -> [(param, bounds)]
    struct_type_params: HashMap<String, usize>,      // struct name -> number of type params
    enum_type_params: HashMap<String, usize>,        // enum name -> number of type params
    trait_type_params: HashMap<String, Vec<String>>,  // trait name -> generic type params
    struct_type_param_bounds: HashMap<String, Vec<(String, Vec<String>)>>,  // struct name -> [(param, bounds)]
    enum_type_param_bounds: HashMap<String, Vec<(String, Vec<String>)>>,    // enum name -> [(param, bounds)]
    trait_type_param_bounds: HashMap<String, Vec<(String, Vec<String>)>>,   // trait name -> [(param, bounds)]
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
            type_param_bounds: HashMap::new(),
            fn_generic_params: HashMap::new(),
            fn_generic_bounds: HashMap::new(),
            struct_type_params: HashMap::new(),
            enum_type_params: HashMap::new(),
            trait_type_params: HashMap::new(),
            struct_type_param_bounds: HashMap::new(),
            enum_type_param_bounds: HashMap::new(),
            trait_type_param_bounds: HashMap::new(),
        }
    }

    pub fn set_module_prefix(&mut self, prefix: &str) {
        self.module_prefix = prefix.to_string();
    }

    pub fn analyze(&mut self, program: &[Stmt]) {
        // First pass: collect all function signatures, class methods, structs, enums
        for stmt in program {
            match stmt {
                Stmt::FnDef { name, type_params, type_param_bounds, params, return_type, line, col, .. } => {
                    let fq_name = self.qualify(name);
                    if self.functions.contains_key(&fq_name) {
                        self.errors.push(CompileError::new(
                            *line,
                            *col,
                            format!("Duplicate function '{}' is already defined", fq_name),
                        ));
                    }
                    self.functions.insert(fq_name, (params.clone(), return_type.clone()));
                    // Track generic params and their bounds for method resolution
                    if !type_params.is_empty() {
                        self.fn_generic_params.insert(name.clone(), type_params.clone());
                        self.fn_generic_bounds.insert(name.clone(), type_param_bounds.clone());
                        // Also populate type_param_bounds for use during body checking
                        for (param, bounds) in type_param_bounds {
                            self.type_param_bounds.insert(param.clone(), bounds.clone());
                        }
                    }
                }
                Stmt::Class { name, methods, .. } => {
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                            let mangled = format!("{}_{}", name, mname);
                            self.functions.insert(mangled, (params.clone(), return_type.clone()));
                        }
                    }
                }
                Stmt::StructDef { name, type_params, type_param_bounds, fields, line, col, .. } => {
                    // Check for duplicate struct name
                    if self.struct_defs.contains_key(name) {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Duplicate struct '{}' is already defined", name),
                        ));
                    }
                    // Track number of type params for arity validation
                    self.struct_type_params.insert(name.clone(), type_params.len());
                    // Store bounds for validation
                    if !type_param_bounds.is_empty() {
                        self.struct_type_param_bounds.insert(name.clone(), type_param_bounds.clone());
                    }
                    // Check for duplicate field names
                    let mut seen_fields = std::collections::HashSet::new();
                    for field in fields {
                        if !seen_fields.insert(field.name.clone()) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Duplicate field '{}' in struct '{}'", field.name, name),
                            ));
                        }
                        // Check for unknown field type
                        match &field.type_ann {
                            TypeAnnot::Class(type_name) => {
                                if !self.struct_defs.contains_key(type_name)
                                    && !self.enum_defs.contains_key(type_name)
                                    && self.functions.get(&format!("{}_new", type_name)).is_none()
                                {
                                    self.errors.push(self.err(
                                        *line, *col,
                                        format!("Unknown type '{}' for field '{}' in struct '{}'", type_name, field.name, name),
                                    ));
                                }
                            }
                            _ => {}
                        }
                    }
                    let ft: Vec<(String, TypeAnnot)> = fields.iter().map(|f| (f.name.clone(), f.type_ann.clone())).collect();
                    self.struct_defs.insert(name.clone(), ft);
                }
                Stmt::EnumDef { name, type_params, type_param_bounds, variants, line, col, .. } => {
                    // Check for duplicate enum name
                    if self.enum_defs.contains_key(name) {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Duplicate enum '{}' is already defined", name),
                        ));
                    }
                    // Track number of type params for arity validation
                    self.enum_type_params.insert(name.clone(), type_params.len());
                    // Store bounds for validation
                    if !type_param_bounds.is_empty() {
                        self.enum_type_param_bounds.insert(name.clone(), type_param_bounds.clone());
                    }
                    // Check for duplicate variant names and unknown payload types
                    let mut seen_variants = std::collections::HashSet::new();
                    for variant in variants {
                        if !seen_variants.insert(variant.name.clone()) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Duplicate variant '{}' in enum '{}'", variant.name, name),
                            ));
                        }
                        // Check for unknown payload types
                        for field_type in &variant.fields {
                            if let TypeAnnot::Class(type_name) = field_type {
                                if !self.struct_defs.contains_key(type_name)
                                    && !self.enum_defs.contains_key(type_name)
                                    && self.functions.get(&format!("{}_new", type_name)).is_none()
                                {
                                    self.errors.push(self.err(
                                        *line, *col,
                                        format!("Unknown type '{}' for variant '{}' in enum '{}'", type_name, variant.name, name),
                                    ));
                                }
                            }
                        }
                    }
                    self.enum_defs.insert(name.clone(), variants.clone());
                }
                Stmt::TraitDef { name, type_params, type_param_bounds, methods, line, col, .. } => {
                    // Check for duplicate trait name
                    if self.traits.contains_key(name) {
                        self.errors.push(self.err(
                            *line, *col,
                            format!("Duplicate trait '{}' is already defined", name),
                        ));
                    }
                    // Check for duplicate method names within the trait
                    let mut seen_methods = std::collections::HashSet::new();
                    for m in methods {
                        if !seen_methods.insert(m.name.clone()) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Duplicate method '{}' in trait '{}'", m.name, name),
                            ));
                        }
                    }
                    // Store trait type params and bounds for generic trait validation
                    self.trait_type_params.insert(name.clone(), type_params.clone());
                    if !type_param_bounds.is_empty() {
                        self.trait_type_param_bounds.insert(name.clone(), type_param_bounds.clone());
                    }
                    self.traits.insert(name.clone(), methods.clone());
                }
                Stmt::ImplBlock { trait_name, trait_type_args, type_params, type_param_bounds, type_name, methods, line, col } => {
                    if let Some(ref trait_name) = trait_name {
                        // Trait impl: impl Trait for Type { ... }
                        // Check trait exists
                        if !self.traits.contains_key(trait_name) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Unknown trait '{}' in impl", trait_name),
                            ));
                        }
                        // Validate generic trait type arg count
                        if let Some(expected_params) = self.trait_type_params.get(trait_name) {
                            if expected_params.len() != trait_type_args.len() {
                                self.errors.push(self.err(
                                    *line, *col,
                                    format!("Generic trait '{}' expects {} type argument(s) but got {}",
                                        trait_name, expected_params.len(), trait_type_args.len()),
                                ));
                            }
                        } else if !trait_type_args.is_empty() {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Trait '{}' does not accept type arguments", trait_name),
                            ));
                        }
                        // Build type substitution map: T -> Int
                        let mut type_subst: HashMap<String, String> = HashMap::new();
                        if let Some(expected_params) = self.trait_type_params.get(trait_name) {
                            for (i, tp) in expected_params.iter().enumerate() {
                                if i < trait_type_args.len() {
                                    type_subst.insert(tp.clone(), trait_type_args[i].clone());
                                }
                            }
                        }
                        // Strip generic type args from type_name for existence check: "Option[Int]" -> "Option"
                        let base_type_name = if let Some(bracket_pos) = type_name.find('[') {
                            &type_name[..bracket_pos]
                        } else {
                            type_name.as_str()
                        };
                        // Check type exists
                        if !self.struct_defs.contains_key(base_type_name) && !self.enum_defs.contains_key(base_type_name) && !self.functions.contains_key(&format!("{}_new", base_type_name)) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Unknown type '{}' in impl", type_name),
                            ));
                        }
                        // Check for duplicate impl (same trait + same type args on same type)
                        if let Some(existing_impls) = self.impls.get(base_type_name) {
                            for (existing_trait, existing_args, _) in existing_impls {
                                if existing_trait == trait_name && existing_args == trait_type_args {
                                    self.errors.push(self.err(
                                        *line, *col,
                                        format!("Duplicate impl of trait '{}' for type '{}'", trait_name, type_name),
                                    ));
                                }
                            }
                        }
                        // Validate method signatures against trait declaration
                        if let Some(trait_methods) = self.traits.get(trait_name) {
                            for m in methods {
                                if let Stmt::FnDef { name: mname, params, return_type, line: mline, col: mcol, .. } = m {
                                    if let Some(trait_m) = trait_methods.iter().find(|tm| tm.name == *mname) {
                                        // Check parameter count (impl method gets self as first param)
                                        let expected_params = trait_m.params.len();
                                        let actual_params = params.len();
                                        if expected_params != actual_params {
                                            self.errors.push(self.err(
                                                *mline, *mcol,
                                                format!("Method '{}' in impl for '{}' on '{}' has {} parameters but trait declares {}",
                                                    mname, trait_name, type_name, actual_params, expected_params),
                                            ));
                                        }
                                        // Check return type with type substitution
                                        let expected_return = Self::substitute_type_params(&trait_m.return_type, &type_subst);
                                        if expected_return != *return_type {
                                            self.errors.push(self.err(
                                                *mline, *mcol,
                                                format!("Method '{}' in impl for '{}' on '{}' has return type {:?} but trait declares {:?}",
                                                    mname, trait_name, type_name, return_type, expected_return),
                                            ));
                                        }
                                    } else {
                                        // Extra method not in trait
                                        self.errors.push(self.err(
                                            *mline, *mcol,
                                            format!("Method '{}' in impl for '{}' on '{}' is not declared in trait '{}'",
                                                mname, trait_name, type_name, trait_name),
                                        ));
                                    }
                                }
                            }
                        }
                        // Register impl methods with mangled names for function lookup
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                                let mangled = format!("{}_{}_{}", base_type_name, trait_name, mname);
                                self.functions.insert(mangled, (params.clone(), return_type.clone()));
                            }
                        }
                        self.impls.entry(base_type_name.to_string())
                            .or_default()
                            .push((trait_name.clone(), trait_type_args.clone(), methods.clone()));
                    } else {
                        // Inherent impl: impl Type { ... }
                        // Strip generic type args for existence check: "Box[T]" -> "Box"
                        let base_type_name = if let Some(bracket_pos) = type_name.find('[') {
                            &type_name[..bracket_pos]
                        } else {
                            type_name.as_str()
                        };
                        // Check type exists
                        if !self.struct_defs.contains_key(base_type_name) && !self.enum_defs.contains_key(base_type_name) && !self.functions.contains_key(&format!("{}_new", base_type_name)) {
                            self.errors.push(self.err(
                                *line, *col,
                                format!("Unknown type '{}' in impl", type_name),
                            ));
                        }
                        // Check for duplicate type parameters
                        let mut seen_tp = std::collections::HashSet::new();
                        for (tp, _) in type_param_bounds {
                            if !seen_tp.insert(tp.clone()) {
                                self.errors.push(self.err(
                                    *line, *col,
                                    format!("Duplicate type parameter '{}' in impl", tp),
                                ));
                            }
                        }
                        // Check for duplicate inherent methods
                        let mut seen_methods = std::collections::HashSet::new();
                        for m in methods {
                            if let Stmt::FnDef { name: mname, line: mline, col: mcol, .. } = m {
                                if !seen_methods.insert(mname.clone()) {
                                    self.errors.push(self.err(
                                        *mline, *mcol,
                                        format!("Duplicate method '{}' in inherent impl for '{}'", mname, type_name),
                                    ));
                                }
                            }
                        }
                        // Register inherent methods with mangled names: Type_method
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, return_type, .. } = m {
                                let mangled = format!("{}_{}", base_type_name, mname);
                                // Store generic type params for this method
                                if !type_params.is_empty() {
                                    self.fn_generic_params.insert(mangled.clone(), type_params.clone());
                                    self.fn_generic_bounds.insert(mangled.clone(), type_param_bounds.clone());
                                }
                                // Check for conflict with existing class methods
                                if self.functions.contains_key(&mangled) && mname != "new" {
                                    // Allow overwriting — inherent methods take priority
                                }
                                self.functions.insert(mangled, (params.clone(), return_type.clone()));
                            }
                        }
                    }
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
            Stmt::FnDef { name, type_param_bounds, params, body, .. } => {
                self.enter_scope();
                // Set up type param bounds for method resolution in body
                let mut saved_bounds = Vec::new();
                for (param, bounds) in type_param_bounds {
                    let old = self.type_param_bounds.insert(param.clone(), bounds.clone());
                    saved_bounds.push((param.clone(), old));
                }
                for (pname, pty) in params {
                    // When inside an impl block, override 'self' param type with the impl type
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
                // Look up return type: try mangled name (class/impl method) or qualified name
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
                // Restore type param bounds
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
                // Push impl type params into scope for method body checking
                let mut saved_bounds = Vec::new();
                for (tp, bounds) in type_param_bounds {
                    let old = self.type_param_bounds.insert(tp.clone(), bounds.clone());
                    saved_bounds.push((tp.clone(), old));
                }
                // Strip generic type args for current_impl: "Box[T]" -> "Box"
                let base_type_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                if let Some(ref trait_name) = trait_name {
                    // Trait impl: check that all required trait methods are implemented
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
                    let old_impl = self.current_impl.replace((base_type_name.to_string(), trait_name.clone()));
                    for m in methods {
                        self.check_stmt(m);
                    }
                    self.current_impl = old_impl;
                } else {
                    // Inherent impl: check method bodies with impl context set
                    let old_impl = self.current_impl.replace((base_type_name.to_string(), String::new()));
                    for m in methods {
                        self.check_stmt(m);
                    }
                    self.current_impl = old_impl;
                }
                // Restore saved type param bounds
                for (tp, old_val) in saved_bounds {
                    match old_val {
                        Some(bounds) => { self.type_param_bounds.insert(tp, bounds); }
                        None => { self.type_param_bounds.remove(&tp); }
                    }
                }
            }
            Stmt::TraitDef { .. } | Stmt::StructDef { .. } | Stmt::EnumDef { .. } => {
                // Already collected in first pass; nothing to check at statement level
            }
        }
    }

    /// Count type arguments from an encoded generic name like "Box[Int, String]".
    /// Returns 0 if no brackets, or the number of comma-separated type args inside the outermost [].
    fn count_type_args_in_name(name: &str) -> usize {
        if let Some(start) = name.find('[') {
            let rest = &name[start + 1..];
            if let Some(end) = rest.find(']') {
                let inner = &rest[..end];
                if inner.is_empty() {
                    0
                } else {
                    // Count commas at top level (not inside nested [])
                    let mut depth = 0usize;
                    let mut count = 1usize;
                    for ch in inner.chars() {
                        match ch {
                            '[' => depth += 1,
                            ']' => depth = depth.saturating_sub(1),
                            ',' if depth == 0 => count += 1,
                            _ => {}
                        }
                    }
                    count
                }
            } else {
                0
            }
        } else {
            0
        }
    }

    /// Substitute generic type parameters in a TypeAnnot with concrete types.
    /// Used for generic trait signature matching: T -> Int, etc.
    fn substitute_type_params(ty: &TypeAnnot, subst: &HashMap<String, String>) -> TypeAnnot {
        match ty {
            TypeAnnot::Generic(name) => {
                if let Some(replacement) = subst.get(name) {
                    TypeAnnot::Class(replacement.clone())
                } else {
                    ty.clone()
                }
            }
            TypeAnnot::Parameterized { base, args } => {
                let new_base = Box::new(Self::substitute_type_params(base, subst));
                let new_args = args.iter().map(|a| Self::substitute_type_params(a, subst)).collect();
                TypeAnnot::Parameterized { base: new_base, args: new_args }
            }
            TypeAnnot::Array(inner) => {
                TypeAnnot::Array(Box::new(Self::substitute_type_params(inner, subst)))
            }
            _ => ty.clone(),
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
            Expr::AssociatedFnCall { type_name, method, args, line, col } => {
                // Strip generic type args: "Option[Int]" -> "Option"
                let base_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                // Resolve as {Type}_{method}
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
                    // Strip generic type args for method lookup: "Option[Int]" -> "Option"
                    let base_tn = if let Some(bracket_pos) = tn.find('[') {
                        &tn[..bracket_pos]
                    } else {
                        tn.as_str()
                    };
                    // Check class methods first
                    let mangled = format!("{}_{}", base_tn, method);
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
                    // Generic type parameter: check trait bounds for method resolution
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
                // Strip generic type args from name: "Box[Int]" -> "Box"
                let base_name = if let Some(bracket_pos) = struct_name.find('[') {
                    &struct_name[..bracket_pos]
                } else {
                    struct_name.as_str()
                };
                // Validate generic type argument count
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
                // Validate struct type parameter bounds
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
                // Strip generic type args: "Option[Int]" -> "Option"
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    &enum_name[..bracket_pos]
                } else {
                    enum_name.as_str()
                };
                // Validate generic type argument count
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
                // Validate enum type parameter bounds
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
                // Strip generic type args: "Option[Int]" -> "Option"
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    &enum_name[..bracket_pos]
                } else {
                    enum_name.as_str()
                };
                // Validate generic type argument count
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
                // Validate enum type parameter bounds
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
                // Pre-compute enum info for variant field types
                let enum_name_opt = match &val_ty {
                    TypeAnnot::Class(name) => {
                        // Strip generic type args: "Option[Int]" -> "Option"
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
                    // Check pattern and declare bindings
                    match &arm.pattern {
                        Pattern::Wildcard => { has_wildcard = true; }
                        Pattern::EnumVariant { enum_name, variant, bindings } => {
                            // Strip generic type args from value type for comparison
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
            // Still infer arg types for side-effect checking (e.g. field access validation)
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

    /// Check if a concrete type implements a given trait.
    /// Looks for impl blocks: impl Trait for Type, or impl Trait[Int] for Type.
    fn type_implements_trait(&self, type_name: Option<&str>, trait_name: &str) -> bool {
        if let Some(tn) = type_name {
            // Check direct impl: impl Trait for Type
            if let Some(impls) = self.impls.get(tn) {
                for (impl_trait, _, _) in impls {
                    if impl_trait == trait_name {
                        return true;
                    }
                }
            }
            // Check generic impl: impl Trait[T] for Type (where T is a concrete type)
            // This covers cases like impl Display[Int] for Printer
            if let Some(impls) = self.impls.get(tn) {
                for (impl_trait, _, _) in impls {
                    if impl_trait == trait_name {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn infer_fn_call_generic(&mut self, name: &str, type_args: &[TypeAnnot], args: &[Expr], line: usize, col: usize) -> TypeAnnot {
        if let Some((params, return_ty)) = self.functions.get(name).cloned() {
            let type_params: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                params.iter()
                    .filter_map(|(_, t)| if let TypeAnnot::Generic(s) = t {
                        if seen.insert(s.clone()) { Some(s.clone()) } else { None }
                    } else { None })
                    .collect()
            };
            // Validate type argument count
            let expected = type_params.len();
            let got = type_args.len();
            if expected == 0 {
                if got > 0 {
                    self.errors.push(self.err(
                        line, col,
                        format!("Function '{}' does not accept type arguments", name),
                    ));
                }
            } else if got == 0 {
                self.errors.push(self.err(
                    line, col,
                    format!("Generic function '{}' requires {} type argument(s) but got 0", name, expected),
                ));
            } else if got != expected {
                self.errors.push(self.err(
                    line, col,
                    format!("Generic function '{}' expects {} type argument(s) but got {}", name, expected, got),
                ));
            }
            // Build type substitution map: T -> Int etc.
            let mut subst = std::collections::HashMap::new();
            for (i, tp) in type_params.iter().enumerate() {
                if i < type_args.len() {
                    subst.insert(tp.clone(), type_args[i].clone());
                }
            }
            // Validate bounds: check that concrete types satisfy trait bounds
            if let Some(fn_bounds) = self.fn_generic_bounds.get(name) {
                for (param_name, bounds) in fn_bounds {
                    if let Some(concrete_ty) = subst.get(param_name) {
                        let concrete_name = match concrete_ty {
                            TypeAnnot::Class(n) => Some(n.as_str()),
                            _ => None,
                        };
                        for bound_trait in bounds {
                            if !self.type_implements_trait(concrete_name, bound_trait) {
                                self.errors.push(self.err(
                                    line, col,
                                    format!(
                                        "Type argument does not satisfy bound '{}' for parameter '{}'",
                                        bound_trait, param_name
                                    ),
                                ));
                            }
                        }
                    }
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

    fn extract_type_args_from_name(encoded: &str) -> Vec<String> {
        if let Some(bracket_pos) = encoded.find('[') {
            let end = encoded.rfind(']').unwrap_or(encoded.len());
            let inner = &encoded[bracket_pos+1..end];
            if inner.is_empty() { return vec![]; }
            let mut args = Vec::new();
            let mut depth = 0;
            let mut current = String::new();
            for ch in inner.chars() {
                match ch {
                    '[' => { depth += 1; current.push(ch); }
                    ']' => { depth -= 1; current.push(ch); }
                    ',' if depth == 0 => { args.push(current.trim().to_string()); current = String::new(); }
                    _ => current.push(ch),
                }
            }
            if !current.trim().is_empty() { args.push(current.trim().to_string()); }
            args
        } else {
            vec![]
        }
    }

    fn validate_struct_bounds(&mut self, struct_name: &str, line: usize, col: usize) {
        let base_name = if let Some(bp) = struct_name.find('[') { &struct_name[..bp] } else { struct_name };
        if let Some(bounds) = self.struct_type_param_bounds.get(base_name).cloned() {
            let type_args = Self::extract_type_args_from_name(struct_name);
            for (i, (param_name, bound_traits)) in bounds.iter().enumerate() {
                if i < type_args.len() {
                    let concrete = &type_args[i];
                    let concrete_base = if let Some(bp) = concrete.find('[') { &concrete[..bp] } else { concrete.as_str() };
                    for bound in bound_traits {
                        if !self.type_implements_trait(Some(concrete_base), bound) {
                            self.errors.push(self.err(line, col,
                                format!("Type argument '{}' does not satisfy bound '{}' for parameter '{}'",
                                    concrete, bound, param_name)));
                        }
                    }
                }
            }
        }
    }

    fn validate_enum_bounds(&mut self, enum_name: &str, line: usize, col: usize) {
        let base_name = if let Some(bp) = enum_name.find('[') { &enum_name[..bp] } else { enum_name };
        if let Some(bounds) = self.enum_type_param_bounds.get(base_name).cloned() {
            let type_args = Self::extract_type_args_from_name(enum_name);
            for (i, (param_name, bound_traits)) in bounds.iter().enumerate() {
                if i < type_args.len() {
                    let concrete = &type_args[i];
                    let concrete_base = if let Some(bp) = concrete.find('[') { &concrete[..bp] } else { concrete.as_str() };
                    for bound in bound_traits {
                        if !self.type_implements_trait(Some(concrete_base), bound) {
                            self.errors.push(self.err(line, col,
                                format!("Type argument '{}' does not satisfy bound '{}' for parameter '{}'",
                                    concrete, bound, param_name)));
                        }
                    }
                }
            }
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
