pub mod expr;
pub mod methods;
pub mod stmt;
pub mod strings;
pub mod types;

use crate::ast::{BinOp, Expr, Pattern, Stmt, TypeAnnot};
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
        writeln!(g, "@__ajeeb_buf = global [16384 x i8] zeroinitializer").unwrap();
        writeln!(g, "@__ajeeb_outbuf = global [65536 x i8] zeroinitializer").unwrap();
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
            "len" | "itoa" | "readArg" | "readFile"
            | "toUpperCase" | "toLowerCase" | "trim" | "chr"
                => Some(format!("declare i64 @{}(i64)", name)),
            // 2-arg functions
            "str_concat" | "indexOf" | "contains"
            | "getInt" | "startsWith" | "endsWith"
            | "charCode" | "strcmp_ajeeb"
                => Some(format!("declare i64 @{}(i64, i64)", name)),
            // 3-arg functions
            "substring" => Some("declare i64 @substring(i64, i64, i64)".into()),
            "replace" => Some("declare i64 @replace(i64, i64, i64)".into()),
            "lib_open" => Some(format!("declare i64 @lib_open(i64)")),
            "lib_sym" => Some(format!("declare i64 @lib_sym(i64, i64)")),
            "tcp_listen" | "tcp_accept" | "tcp_connect" | "tls_connect" => Some(format!("declare i64 @{}(i64)", name)),
            "tcp_read" => Some(format!("declare i64 @tcp_read(i64, i64)")),
            "dns_lookup" | "tls_read" => Some(format!("declare i64 @{}(i64, i64)", name)),
            "tcp_write" | "tcp_close" | "tls_write" | "tls_close" | "setInt" | "strSet" => Some(format!("declare void @{}(i64, i64)", name)),
            "writeFile" | "writeAppend" | "writeByte" => Some(format!("declare void @{}(i64, i64)", name)),
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
                Stmt::FnDef { name, .. } => {
                    self.user_fns.insert(name.clone());
                }
                Stmt::StructDef { name, fields, .. } => {
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
                                let trait_key = format!("{}@{}", mname, trait_name);
                                self.method_map.insert((base_type_name.to_string(), trait_key), mangled.clone());
                                self.emit_fn_def(&mangled, &params, &body)?;
                            }
                        }
                    } else {
                        // Inherent impl: mangled as Type_method
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, body, return_type, .. } = m.clone() {
                                let mangled = format!("{}_{}", base_type_name, mname);
                                self.user_fns.insert(mangled.clone());
                                self.method_map.insert((base_type_name.to_string(), mname.clone()), mangled.clone());
                                self.emit_fn_def(&mangled, &params, &body)?;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        let mut main_stmts = Vec::new();
        let mut global_init_stmts = Vec::new(); // top-level let/const init to emit in main

        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, body, .. } if name == "main" => {
                    main_stmts.extend(body.clone());
                }
                Stmt::FnDef { name, params, body, .. } => {
                    self.emit_fn_def(name, params, body)?;
                }
                Stmt::Let { name, value, .. } | Stmt::Const { name, value, .. } => {
                    // Top-level variables become LLVM globals accessible from any function
                    let gname = format!("__ajb_global_{}", name);
                    writeln!(self.globals, "@{} = global i64 0", gname).unwrap();
                    self.globals_map.insert(name.clone(), gname);
                    // Defer initialization to main
                    global_init_stmts.push((name.clone(), value.clone()));
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
        // Emit global initializers
        for (name, val) in &global_init_stmts {
            let v = self.emit_expr(val)?;
            let gname = self.globals_map.get(name).ok_or_else(|| format!("Unknown global variable: {}", name))?;
            writeln!(self.body, "  store i64 {}, ptr @{}", v, gname).unwrap();
        }
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


