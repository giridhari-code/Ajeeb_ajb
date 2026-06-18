pub mod generics;
pub mod modules;
pub mod traits;
pub mod typecheck;

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
    ("getStr", TypeAnnot::String),
    ("exec", TypeAnnot::Int),
    ("mkdir", TypeAnnot::Int),
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
