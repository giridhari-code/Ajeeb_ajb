pub mod expr;
pub mod generic;
pub mod methods;
pub mod mir;
pub mod stmt;
pub mod strings;
pub mod types;

use crate::ast::{Stmt, TypeAnnot};
use crate::hir::HirType;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

fn host_target_triple() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64-unknown-linux-gnu",
        "x86_64" => "x86_64-unknown-linux-gnu",
        _ => "aarch64-unknown-linux-gnu",
    }
}

pub struct Codegen {
    body: String,
    globals: String,
    functions: String,
    unnamed_count: u64,
    label_count: u64,
    str_count: u64,
    variables: HashMap<String, String>,
    globals_map: HashMap<String, String>,  // top-level vars → LLVM global names
    mir_temps: HashMap<String, String>,   // MIR temp names → LLVM SSA register names
    loops: Vec<(String, String)>,
    block_terminated: bool,
    declared_extern: HashSet<String>,
    user_fns: HashSet<String>,
    // Type tracking for structs, enums, and method dispatch
    struct_defs: HashMap<String, Vec<(String, TypeAnnot)>>,     // name → ordered fields
    enum_defs: HashMap<String, Vec<(String, Vec<TypeAnnot>)>>,  // name → [(variant, field_types)]
    enum_variant_ids: HashMap<(String, String), i64>,           // (enum, variant) → integer tag
    var_types: HashMap<String, (String, String)>,               // var → (kind: "struct"|"enum", type_name)
    // For each type+method, the mangled function name registered via ImplBlock
    method_map: HashMap<(String, String), String>,              // (type_name, method) → mangled_fn
    // String type tracking — prevents integer-add when operand is a string
    string_vars: HashSet<String>,   // variable names known to hold strings
    string_regs: HashSet<String>,   // LLVM register names holding string pointers
    // Function return type tracking — for propagating string_regs through calls
    fn_return_types: HashMap<String, TypeAnnot>,  // fn_name → return type
    // Generic type parameter tracking — for resolving generic fields
    struct_type_params: HashMap<String, Vec<String>>,  // struct_name → type_param names
    // Boolean type tracking — for printing true/false instead of 1/0
    bool_vars: HashSet<String>,     // variable names known to hold booleans
    bool_regs: HashSet<String>,     // LLVM register names holding boolean values
    // Float type tracking — for fadd/fsub etc instead of i64 ops
    float_vars: HashSet<String>,    // variable names known to hold floats
    float_regs: HashSet<String>,    // LLVM register names holding float values (as i64 bit pattern)
    // Array type tracking — for printing arrays recursively
    array_vars: HashSet<String>,    // variable names known to hold arrays
    array_regs: HashSet<String>,    // LLVM register names holding array pointers
    // Generic function monomorphization — store generic fn bodies for specialization
    generic_fns: HashMap<String, (Vec<String>, Vec<(String, TypeAnnot)>, Vec<Stmt>)>,  // fn_name → (type_params, params, body)
    monomorphized: HashSet<String>,  // Already generated specialized functions
    // Enum type tracking — for comparing enum tags instead of pointers
    enum_vars: HashSet<String>,     // variable names known to hold enum values
    enum_regs: HashSet<String>,      // LLVM register names holding enum pointers (ptrtoint results)
    // Array element type tracking — for propagating string_regs through __index
    array_elem_types: HashMap<String, HirType>,  // var_name → element type of array
}

impl Codegen {
    pub fn new() -> Self {
        let mut g = String::new();
        writeln!(g, "target datalayout = \"e-m:e-i64:64-f80:128-n8:16:32:64-S128\"").unwrap();
        writeln!(g, "target triple = \"{}\"", host_target_triple()).unwrap();
        writeln!(g, "").unwrap();
        writeln!(g, "declare i32 @puts(ptr)").unwrap();
        writeln!(g, "declare i32 @printf(ptr, ...)").unwrap();
        writeln!(g, "declare i32 @snprintf(ptr, i64, ptr, ...)").unwrap();

        // Global buffers expected by C runtime
        writeln!(g, "@__ajeeb_buf = global [4194304 x i8] zeroinitializer").unwrap();
        writeln!(g, "@__ajeeb_outbuf = global [65536 x i8] zeroinitializer").unwrap();
        writeln!(g, "@stderr = external global ptr").unwrap();
        writeln!(g, "").unwrap();
        Codegen {
            body: String::new(),
            globals: g,
            functions: String::new(),
            unnamed_count: 0,
            label_count: 0,
            str_count: 0,
            variables: HashMap::new(),
            globals_map: HashMap::new(),
            mir_temps: HashMap::new(),
            loops: Vec::new(),
            block_terminated: false,
            declared_extern: HashSet::new(),
            user_fns: HashSet::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            enum_variant_ids: HashMap::new(),
            var_types: HashMap::new(),
            method_map: HashMap::new(),
            string_vars: HashSet::new(),
            string_regs: HashSet::new(),
            fn_return_types: HashMap::new(),
            struct_type_params: HashMap::new(),
            bool_vars: HashSet::new(),
            bool_regs: HashSet::new(),
            array_vars: HashSet::new(),
            array_regs: HashSet::new(),
            generic_fns: HashMap::new(),
            monomorphized: HashSet::new(),
            enum_regs: HashSet::new(),
            enum_vars: HashSet::new(),
            float_vars: HashSet::new(),
            float_regs: HashSet::new(),
            array_elem_types: HashMap::new(),
        }
    }

    fn fresh(&mut self) -> String {
        let n = self.unnamed_count;
        self.unnamed_count += 1;
        format!("%{}", n)
    }

    fn fresh_label(&mut self) -> String {
        let n = self.label_count;
        self.label_count += 1;
        format!("L{}", n)
    }

    // Lazily declare a C runtime extern function (only once, avoids redefinition conflicts).
    // Returns true if the function was declared (known C runtime), false if unknown.
    fn declare_extern(&mut self, name: &str) -> bool {
        if self.declared_extern.contains(name) || self.user_fns.contains(name) {
            return true; // Already known
        }
        let decl: Option<String> = match name {
            // 0-arg functions
            "getStateBuf" | "getOutbuf"
                => Some(format!("declare i64 @{}()", name)),
            // 1-arg functions
            "len" | "arr_len" | "itoa" | "readArg" | "readFile"
            | "toUpperCase" | "toLowerCase" | "trim"
            | "exec" | "mkdir" | "getStr"
                => Some(format!("declare i64 @{}(i64)", name)),
            // 2-arg functions
            "str_concat" | "contains"
            | "getInt" | "startsWith" | "endsWith"
            | "charCode" | "strcmp_ajeeb" | "chr"
                => Some(format!("declare i64 @{}(i64, i64)", name)),
            // 3-arg functions
            "indexOf" => Some(format!("declare i64 @{}(i64, i64)", name)),
            "substring" => Some("declare i64 @substring(i64, i64, i64)".into()),
            "replace" => Some("declare i64 @replace(i64, i64, i64)".into()),
            "lib_open" => Some(format!("declare i64 @lib_open(i64)")),
            "lib_sym" => Some(format!("declare i64 @lib_sym(i64, i64)")),
            "tcp_listen" | "tcp_accept" | "tls_connect" | "allocBuf" => Some(format!("declare i64 @{}(i64)", name)),
            "tcp_connect" => Some(format!("declare i64 @tcp_connect(i64, i64)")),
            "tcp_read" => Some(format!("declare i64 @tcp_read(i64, i64)")),
            "dns_lookup" | "tls_read" => Some(format!("declare i64 @{}(i64)", name)),
            "tcp_write" | "tls_write" => Some(format!("declare void @{}(i64, i64)", name)),
            "setInt" | "strSet" => Some(format!("declare void @{}(i64, i64, i64)", name)),
            "tcp_close" | "tls_close" => Some(format!("declare void @{}(i64)", name)),
            "writeFile" | "writeAppend" | "writeByte" => Some(format!("declare void @{}(i64, i64)", name)),
            "exit" => Some("declare void @exit(i32)".into()),
            "malloc" => Some("declare ptr @malloc(i64)".into()),
            "free" => Some("declare void @free(ptr)".into()),
            "array_to_string" => Some("declare i64 @array_to_string(i64, i64)".into()),
            "__array_lit" => Some("declare i64 @__array_lit(i64, ...)".into()),
            "__index" => Some("declare i64 @__index(i64, i64)".into()),
            "__index_assign" => Some("declare i64 @__index_assign(i64, i64, i64)".into()),
            "fprintf" => Some("declare i32 @fprintf(ptr, ptr, ...)".into()),
            "stderr_ptr" => None, // Not a function, handled separately
            _ => None, // Not a known C extern
        };
        if let Some(d) = &decl {
            self.declared_extern.insert(name.to_string());
            writeln!(self.globals, "{}", d).unwrap();
            true
        } else {
            false
        }
    }

    pub fn compile(&mut self, stmts: &[Stmt]) -> Result<String, String> {
        // First pass: collect user-defined functions, struct defs, enum defs, impl blocks
        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, return_type, type_params, params, body, .. } => {
                    self.user_fns.insert(name.clone());
                    self.fn_return_types.insert(name.clone(), return_type.clone());
                    // Store generic function body for monomorphization
                    if !type_params.is_empty() {
                        self.generic_fns.insert(name.clone(), (type_params.clone(), params.clone(), body.clone()));
                    }
                }
                Stmt::StructDef { name, fields, type_params, .. } => {
                    let field_list: Vec<(String, TypeAnnot)> = fields.iter()
                        .map(|f| (f.name.clone(), f.type_ann.clone()))
                        .collect();
                    self.struct_defs.insert(name.clone(), field_list);
                    if !type_params.is_empty() {
                        self.struct_type_params.insert(name.clone(), type_params.clone());
                    }
                    // Also register by base name without generics for lookup
                    let base = name.split('[').next().unwrap_or(name);
                    if base != name && !self.struct_defs.contains_key(base) {
                        self.struct_defs.insert(base.to_string(), self.struct_defs[name].clone());
                    }
                }
                Stmt::Class { name, fields, .. } => {
                    // Register class fields in struct_defs (same as struct)
                    let field_list: Vec<(String, TypeAnnot)> = fields.iter()
                        .map(|f| (f.name.clone(), f.type_ann.clone()))
                        .collect();
                    self.struct_defs.insert(name.clone(), field_list);
                }
                Stmt::EnumDef { name, variants, .. } => {
                    let var_list: Vec<(String, Vec<TypeAnnot>)> = variants.iter()
                        .map(|v| (v.name.clone(), v.fields.clone()))
                        .collect();
                    self.enum_defs.insert(name.clone(), var_list.clone());
                    // Assign integer IDs to each variant
                    for (i, (vname, _)) in var_list.iter().enumerate() {
                        self.enum_variant_ids.insert((name.clone(), vname.clone()), i as i64);
                    }
                }
                Stmt::ImplBlock { trait_name, type_name, methods, .. } => {
                    // Strip generic type args: "Box[T]" -> "Box"
                    let base_type_name = if let Some(bracket_pos) = type_name.find('[') {
                        &type_name[..bracket_pos]
                    } else {
                        type_name.as_str()
                    };
                    if let Some(ref trait_name) = trait_name {
                        // Trait impl: mangled as Type_Trait_method
                        // Use a distinct key (type, method@trait) to avoid overwriting inherent methods
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, body, return_type, .. } = m.clone() {
                                let mangled = format!("{}_{}_{}", base_type_name, trait_name, mname);
                                self.user_fns.insert(mangled.clone());
                                self.fn_return_types.insert(mangled.clone(), return_type.clone());
                                let trait_key = format!("{}@{}", mname, trait_name);
                                self.method_map.insert((base_type_name.to_string(), trait_key), mangled.clone());
                                if self.struct_defs.contains_key(base_type_name) {
                                    for (pname, _) in &params {
                                        if pname == "self" {
                                            self.var_types.insert("self".to_string(), ("struct".into(), base_type_name.to_string()));
                                        }
                                    }
                                }
                                self.emit_fn_def(&mangled, &params, &body)?;
                            }
                        }
                    } else {
                        // Inherent impl: mangled as Type_method
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, body, return_type, .. } = m.clone() {
                                let mangled = format!("{}_{}", base_type_name, mname);
                                self.user_fns.insert(mangled.clone());
                                self.fn_return_types.insert(mangled.clone(), return_type.clone());
                                self.method_map.insert((base_type_name.to_string(), mname.clone()), mangled.clone());
                                if self.struct_defs.contains_key(base_type_name) {
                                    for (pname, _) in &params {
                                        if pname == "self" {
                                            self.var_types.insert("self".to_string(), ("struct".into(), base_type_name.to_string()));
                                        }
                                    }
                                }
                                self.emit_fn_def(&mangled, &params, &body)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut main_stmts = Vec::new();

        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, body, type_params, .. } if name == "main" => {
                    main_stmts.extend(body.clone());
                }
                Stmt::FnDef { name, params, body, type_params, .. } => {
                    // Skip generic functions — they'll be monomorphized when called
                    if type_params.is_empty() {
                        self.emit_fn_def(name, params, body)?;
                    }
                }
                Stmt::Set { name, value, .. } | Stmt::Const { name, value, .. } => {
                    // Top-level variables become LLVM globals accessible from any function
                    let gname = format!("__ajb_global_{}", name);
                    writeln!(self.globals, "@{} = global i64 0", gname).unwrap();
                    self.globals_map.insert(name.clone(), gname);
                    // Keep in main_stmts for inline initialization
                    main_stmts.push(stmt.clone());
                }
                _ => {
                    main_stmts.push(stmt.clone());
                }
            }
        }

        writeln!(self.body, "define i64 @main() {{").unwrap();
        self.unnamed_count = 1;
        self.emit_allocas_for_stmts(&main_stmts);
        let entry = self.fresh_label();
        writeln!(self.body, "  br label %{}", entry).unwrap();
        writeln!(self.body, "{}:", entry).unwrap();
        // Emit stmts in original order — globals are already initialized inline
        // because global_init_stmts are now part of main_stmts
        self.emit_stmts(&main_stmts)?;
        writeln!(self.body, "  ret i64 0").unwrap();
        writeln!(self.body, "}}").unwrap();

        Ok(format!("{}{}{}", self.globals, self.functions, self.body))
    }

    pub fn write_ir_to_file(&self, path: &str) -> Result<(), String> {
        let full = format!("{}{}{}", self.globals, self.functions, self.body);
        std::fs::write(path, &full).map_err(|e| format!("Failed to write IR: {}", e))
    }

}


