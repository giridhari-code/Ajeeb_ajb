pub mod expr;
pub mod methods;
pub mod stmt;
pub mod strings;
pub mod types;

use crate::ast::{Expr, Stmt, TypeAnnot};
use crate::mir::{self as mir_mod, MirBinOp, MirConst, MirOperand, MirProgram, MirRvalue, MirStmt, Terminator};
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
    // Array type tracking — for printing arrays recursively
    array_vars: HashSet<String>,    // variable names known to hold arrays
    array_regs: HashSet<String>,    // LLVM register names holding array pointers
    // Generic function monomorphization — store generic fn bodies for specialization
    generic_fns: HashMap<String, (Vec<String>, Vec<(String, TypeAnnot)>, Vec<Stmt>)>,  // fn_name → (type_params, params, body)
    monomorphized: HashSet<String>,  // Already generated specialized functions
    // Enum type tracking — for comparing enum tags instead of pointers
    enum_vars: HashSet<String>,     // variable names known to hold enum values
    enum_regs: HashSet<String>,      // LLVM register names holding enum pointers (ptrtoint results)
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

    // Type substitution for monomorphization
    fn subst_type_ann(t: &TypeAnnot, subst: &HashMap<String, TypeAnnot>) -> TypeAnnot {
        match t {
            TypeAnnot::Generic(name) => subst.get(name).cloned().unwrap_or(t.clone()),
            TypeAnnot::Array(inner) => TypeAnnot::Array(Box::new(Self::subst_type_ann(inner, subst))),
            TypeAnnot::Class(name) => TypeAnnot::Class(name.clone()),
            other => other.clone(),
        }
    }

    fn subst_expr(e: &Expr, subst: &HashMap<String, TypeAnnot>) -> Expr {
        match e {
            Expr::Ident(name, line, col) => Expr::Ident(name.clone(), *line, *col),
            Expr::MethodCall { obj, method, args, line, col } => {
                Expr::MethodCall {
                    obj: Box::new(Self::subst_expr(obj, subst)),
                    method: method.clone(),
                    args: args.iter().map(|a| Self::subst_expr(a, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            Expr::FnCall { name, args, line, col } => {
                Expr::FnCall {
                    name: name.clone(),
                    args: args.iter().map(|a| Self::subst_expr(a, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            Expr::StringLit(s, line, col) => Expr::StringLit(s.clone(), *line, *col),
            Expr::Number(n, line, col) => Expr::Number(*n, *line, *col),
            Expr::Bool(b, line, col) => Expr::Bool(*b, *line, *col),
            Expr::Binary { left, op, right, line, col } => {
                Expr::Binary {
                    left: Box::new(Self::subst_expr(left, subst)),
                    op: op.clone(),
                    right: Box::new(Self::subst_expr(right, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Expr::UnaryNot(inner, line, col) => {
                Expr::UnaryNot(Box::new(Self::subst_expr(inner, subst)), *line, *col)
            }
            Expr::Assign { name, value, line, col } => {
                Expr::Assign {
                    name: name.clone(),
                    value: Box::new(Self::subst_expr(value, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Expr::ArrayLit(items, line, col) => {
                Expr::ArrayLit(
                    items.iter().map(|i| Self::subst_expr(i, subst)).collect(),
                    *line,
                    *col,
                )
            }
            other => other.clone(),
        }
    }

    fn subst_stmt(s: &Stmt, subst: &HashMap<String, TypeAnnot>) -> Stmt {
        match s {
            Stmt::Set { name, type_ann, value, pub_, line, col } => {
                Stmt::Set {
                    name: name.clone(),
                    type_ann: type_ann.as_ref().map(|t| Self::subst_type_ann(t, subst)),
                    value: Self::subst_expr(value, subst),
                    pub_: *pub_,
                    line: *line,
                    col: *col,
                }
            }
            Stmt::Const { name, type_ann, value, pub_, line, col } => {
                Stmt::Const {
                    name: name.clone(),
                    type_ann: type_ann.as_ref().map(|t| Self::subst_type_ann(t, subst)),
                    value: Self::subst_expr(value, subst),
                    pub_: *pub_,
                    line: *line,
                    col: *col,
                }
            }
            Stmt::Expr(expr, line, col) => {
                Stmt::Expr(Self::subst_expr(expr, subst), *line, *col)
            }
            Stmt::Return { value, line, col } => {
                Stmt::Return {
                    value: value.as_ref().map(|v| Self::subst_expr(v, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Stmt::If { condition, then_block, else_block, line, col } => {
                Stmt::If {
                    condition: Self::subst_expr(condition, subst),
                    then_block: then_block.iter().map(|s| Self::subst_stmt(s, subst)).collect(),
                    else_block: else_block.as_ref().map(|eb| eb.iter().map(|s| Self::subst_stmt(s, subst)).collect()),
                    line: *line,
                    col: *col,
                }
            }
            Stmt::ForLoop { init, condition, update, body, line, col } => {
                Stmt::ForLoop {
                    init: Box::new(Self::subst_stmt(init, subst)),
                    condition: Self::subst_expr(condition, subst),
                    update: Box::new(Self::subst_stmt(update, subst)),
                    body: body.iter().map(|s| Self::subst_stmt(s, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            other => other.clone(),
        }
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
            | "toUpperCase" | "toLowerCase" | "trim"
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
            "tcp_listen" | "tcp_accept" | "tls_connect" => Some(format!("declare i64 @{}(i64)", name)),
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

    /// Compile MIR directly to LLVM IR.
    /// Each MIR function → LLVM function, each BasicBlock → LLVM basic block.
    pub fn compile_mir(&mut self, prog: &MirProgram) -> Result<String, String> {
        // Register struct/enum definitions from MIR for type tracking
        for (name, fields) in &prog.structs {
            let field_list: Vec<(String, TypeAnnot)> = fields.iter()
                .map(|(n, t)| (n.clone(), hir_type_to_type_ann(t)))
                .collect();
            self.struct_defs.insert(name.clone(), field_list);
        }
        for (name, variants) in &prog.enums {
            let var_list: Vec<(String, Vec<TypeAnnot>)> = variants.iter()
                .map(|(vn, vts)| (vn.clone(), vts.iter().map(hir_type_to_type_ann).collect()))
                .collect();
            self.enum_defs.insert(name.clone(), var_list.clone());
            for (i, (vname, _)) in var_list.iter().enumerate() {
                self.enum_variant_ids.insert((name.clone(), vname.clone()), i as i64);
            }
            self.user_fns.insert(name.clone());
        }

        // Register all function names upfront so forward calls resolve
        for f in &prog.functions {
            self.user_fns.insert(f.name.clone());
        }

        // Generate each MIR function as an LLVM function
        for f in &prog.functions {
            self.emit_mir_fn(f)?;
        }

        // If no user-defined main, generate a default entry point
        if !self.user_fns.contains("main") {
            writeln!(self.body, "define i64 @main() {{").unwrap();
            self.unnamed_count = 1;
            let entry = self.fresh_label();
            writeln!(self.body, "  br label %{}", entry).unwrap();
            writeln!(self.body, "{}:", entry).unwrap();
            writeln!(self.body, "  ret i64 0").unwrap();
            writeln!(self.body, "}}").unwrap();
        }

        Ok(format!("{}{}{}", self.globals, self.functions, self.body))
    }

    fn emit_mir_fn(&mut self, f: &mir_mod::MirFn) -> Result<(), String> {
        let saved_unnamed = self.unnamed_count;
        let saved_string_vars = self.string_vars.clone();
        let saved_string_regs = self.string_regs.clone();
        let saved_var_types = self.var_types.clone();
        let saved_bool_regs = self.bool_regs.clone();
        let saved_array_regs = self.array_regs.clone();
        let saved_array_vars = self.array_vars.clone();
        let saved_enum_regs = self.enum_regs.clone();
        let saved_enum_vars = self.enum_vars.clone();
        let saved_mir_temps = self.mir_temps.clone();
        self.string_regs.clear();
        self.bool_regs.clear();
        self.array_regs.clear();
        self.array_vars.clear();
        self.enum_regs.clear();
        self.enum_vars.clear();
        self.mir_temps.clear();
        self.unnamed_count = f.params.len() as u64 + 1;

        let mut fn_body = String::new();
        let mut fn_vars: HashMap<String, String> = HashMap::new();

        // Function signature
        let params_ir: Vec<String> = f.params.iter().map(|_| "i64".to_string()).collect();
        let header = format!("define i64 @{}({}) {{\n", f.name, params_ir.join(", "));

        // Alloca for parameters
        for (i, (pname, _)) in f.params.iter().enumerate() {
            let param_reg = format!("%{}", i);
            let reg = self.fresh();
            write!(fn_body, "  {} = alloca i64, align 8\n", reg).unwrap();
            write!(fn_body, "  store i64 {}, ptr {}\n", param_reg, reg).unwrap();
            fn_vars.insert(pname.clone(), reg);
        }

        // Alloca for all locals
        for (lname, _) in &f.locals {
            if !fn_vars.contains_key(lname) {
                let reg = self.fresh();
                write!(fn_body, "  {} = alloca i64, align 8\n", reg).unwrap();
                fn_vars.insert(lname.clone(), reg);
            }
        }

        // Entry block
        let entry_label = format!("L{}_{}", self.label_count, f.name);
        self.label_count += 1;
        write!(fn_body, "  br label %{}\n", entry_label).unwrap();
        write!(fn_body, "{}:\n", entry_label).unwrap();

        // Save and swap state
        let saved_body = std::mem::replace(&mut self.body, fn_body);
        let saved_vars = std::mem::replace(&mut self.variables, fn_vars);
        self.block_terminated = false;

        // Emit each MIR basic block
        for block in &f.blocks {
            let label = format!("mir_b{}", block.id);
            if !self.block_terminated {
                write!(self.body, "  br label %{}\n", label).unwrap();
            }
            write!(self.body, "{}:\n", label).unwrap();
            self.block_terminated = false;

            // Emit statements
            for stmt in &block.statements {
                self.emit_mir_stmt(stmt)?;
            }

            // Emit terminator
            self.emit_mir_terminator(&block.terminator)?;
        }

        // Ensure function ends with a terminator
        if !self.block_terminated {
            write!(self.body, "  ret i64 0\n").unwrap();
        }

        let full_fn = format!("{}{}}}\n", header, self.body);
        self.functions.push_str(&full_fn);
        self.body = saved_body;
        self.variables = saved_vars;
        self.unnamed_count = saved_unnamed;
        self.string_vars = saved_string_vars;
        self.string_regs = saved_string_regs;
        self.var_types = saved_var_types;
        self.bool_regs = saved_bool_regs;
        self.array_regs = saved_array_regs;
        self.array_vars = saved_array_vars;
        self.enum_regs = saved_enum_regs;
        self.enum_vars = saved_enum_vars;
        self.mir_temps = saved_mir_temps;
        Ok(())
    }

    fn emit_mir_stmt(&mut self, stmt: &MirStmt) -> Result<(), String> {
        match stmt {
            MirStmt::Assign { dest, value } => {
                let val = self.emit_mir_rvalue(value)?;
                // Propagate type tracking (strings, bools, etc.)
                let is_string = self.string_regs.contains(&val);
                let is_bool = self.bool_regs.contains(&val);
                // Check if it's a user variable (store to alloca) or a temp (SSA register)
                if let Some(var_reg) = self.variables.get(dest).cloned() {
                    write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                } else if let Some(gname) = self.globals_map.get(dest).cloned() {
                    write!(self.body, "  store i64 {}, ptr @{}\n", val, gname).unwrap();
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                } else {
                    // MIR temporary - track the SSA register and propagate type info
                    self.mir_temps.insert(dest.clone(), val.clone());
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                }
                Ok(())
            }
            MirStmt::Call { dest, func, args } => {
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.emit_mir_operand(arg)?);
                }
                let args_str = compiled_args.iter()
                    .map(|a| format!("i64 {}", a))
                    .collect::<Vec<_>>()
                    .join(", ");

                // Handle builtins
                match func.as_str() {
                    "println" | "print" => {
                        self.emit_mir_print(compiled_args, func == "println")?;
                    }
                    _ => {
                        if !self.declare_extern(func) && !self.user_fns.contains(func.as_str()) {
                            return Err(format!("MIR codegen: unknown function '{}'", func));
                        }
                        if let Some(dest_name) = dest {
                            let reg = self.fresh();
                            write!(self.body, "  {} = call i64 @{}({})\n", reg, func, args_str).unwrap();
                            // Track return type for known string-returning functions
                            if matches!(func.as_str(),
                                "str_concat" | "itoa" | "substring" | "toUpperCase" | "toLowerCase"
                                | "trim" | "readFile" | "readArg" | "replace"
                            ) {
                                self.string_regs.insert(reg.clone());
                                self.string_vars.insert(dest_name.clone());
                            }
                            if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                write!(self.body, "  store i64 {}, ptr {}\n", reg, var_reg).unwrap();
                            } else {
                                self.mir_temps.insert(dest_name.clone(), reg);
                            }
                        } else {
                            write!(self.body, "  call i64 @{}({})\n", func, args_str).unwrap();
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn emit_mir_rvalue(&mut self, rvalue: &MirRvalue) -> Result<String, String> {
        match rvalue {
            MirRvalue::Use(operand) => self.emit_mir_operand(operand),
            MirRvalue::Const(c) => self.emit_mir_const(c),
            MirRvalue::BinaryOp(op, left, right) => {
                let l = self.emit_mir_operand(left)?;
                let r = self.emit_mir_operand(right)?;
                // Check if this is a string operation
                let is_str_add = *op == MirBinOp::Add
                    && (self.string_regs.contains(&l) || self.string_regs.contains(&r)
                        || self.string_vars.iter().any(|v| {
                            self.mir_temps.get(v).map_or(false, |sr| sr == &l || sr == &r)
                        }));
                if is_str_add {
                    self.declare_extern("str_concat");
                    let reg = self.fresh();
                    write!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})\n", reg, l, r).unwrap();
                    self.string_regs.insert(reg.clone());
                    return Ok(reg);
                }
                let reg = self.fresh();
                match op {
                    MirBinOp::Add => write!(self.body, "  {} = add i64 {}, {}\n", reg, l, r).unwrap(),
                    MirBinOp::Sub => write!(self.body, "  {} = sub i64 {}, {}\n", reg, l, r).unwrap(),
                    MirBinOp::Mul => write!(self.body, "  {} = mul i64 {}, {}\n", reg, l, r).unwrap(),
                    MirBinOp::Div => {
                        let is_zero = self.fresh();
                        write!(self.body, "  {} = icmp eq i64 {}, 0\n", is_zero, r).unwrap();
                        let safe_r = self.fresh();
                        write!(self.body, "  {} = select i1 {}, i64 1, i64 {}\n", safe_r, is_zero, r).unwrap();
                        let div_raw = self.fresh();
                        write!(self.body, "  {} = sdiv i64 {}, {}\n", div_raw, l, safe_r).unwrap();
                        let final_reg = self.fresh();
                        write!(self.body, "  {} = select i1 {}, i64 0, i64 {}\n", final_reg, is_zero, div_raw).unwrap();
                        return Ok(final_reg);
                    }
                    MirBinOp::Eq => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp eq i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Neq => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp ne i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Lt => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp slt i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Gt => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp sgt i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Le => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp sle i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Ge => {
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp sge i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::And => {
                        write!(self.body, "  {} = and i64 {}, {}\n", reg, l, r).unwrap();
                        self.bool_regs.insert(reg.clone());
                    }
                    MirBinOp::Or => {
                        write!(self.body, "  {} = or i64 {}, {}\n", reg, l, r).unwrap();
                        self.bool_regs.insert(reg.clone());
                    }
                }
                Ok(reg)
            }
        }
    }

    fn emit_mir_operand(&mut self, operand: &MirOperand) -> Result<String, String> {
        match operand {
            MirOperand::Var(name) => {
                // Check user variables first (stored in alloca)
                if let Some(var_reg) = self.variables.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr {}\n", reg, var_reg).unwrap();
                    // Propagate type tracking
                    if self.string_vars.contains(name) { self.string_regs.insert(reg.clone()); }
                    if self.bool_vars.contains(name) { self.bool_regs.insert(reg.clone()); }
                    Ok(reg)
                } else if let Some(ssa_reg) = self.mir_temps.get(name).cloned() {
                    // MIR temp - already an SSA register, propagate type info
                    if self.string_vars.contains(name) { self.string_regs.insert(ssa_reg.clone()); }
                    if self.bool_vars.contains(name) { self.bool_regs.insert(ssa_reg.clone()); }
                    Ok(ssa_reg)
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr @{}\n", reg, gname).unwrap();
                    if self.string_vars.contains(name) { self.string_regs.insert(reg.clone()); }
                    if self.bool_vars.contains(name) { self.bool_regs.insert(reg.clone()); }
                    Ok(reg)
                } else {
                    // Function parameter - already an SSA register (%0, %1, etc.)
                    if let Ok(idx) = name.parse::<usize>() {
                        Ok(format!("%{}", idx))
                    } else {
                        Ok(format!("%{}", name))
                    }
                }
            }
            MirOperand::Constant(c) => self.emit_mir_const(c),
        }
    }

    fn emit_mir_const(&mut self, c: &MirConst) -> Result<String, String> {
        match c {
            MirConst::Int(n) => {
                let reg = self.fresh();
                write!(self.body, "  {} = add i64 0, {}\n", reg, n).unwrap();
                Ok(reg)
            }
            MirConst::Str(s) => {
                let gname = self.global_str(s);
                let ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", ptr, gname).unwrap();
                let reg = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", reg, ptr).unwrap();
                self.string_regs.insert(reg.clone());
                Ok(reg)
            }
            MirConst::Bool(b) => {
                let reg = self.fresh();
                write!(self.body, "  {} = add i64 0, {}\n", reg, if *b { 1 } else { 0 }).unwrap();
                self.bool_regs.insert(reg.clone());
                Ok(reg)
            }
        }
    }

    fn emit_mir_terminator(&mut self, term: &Terminator) -> Result<(), String> {
        match term {
            Terminator::Goto(target) => {
                let label = format!("mir_b{}", target);
                write!(self.body, "  br label %{}\n", label).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Terminator::SwitchInt { cond, targets, default } => {
                let cond_val = self.emit_mir_operand(cond)?;
                let cond_bool = self.fresh();
                write!(self.body, "  {} = icmp ne i64 {}, 0\n", cond_bool, cond_val).unwrap();
                if targets.is_empty() {
                    let label = format!("mir_b{}", default);
                    write!(self.body, "  br i1 {}, label %{}, label %{}\n", cond_bool, label, label).unwrap();
                } else {
                    let (_val, target) = &targets[0];
                    let true_label = format!("mir_b{}", target);
                    let false_label = format!("mir_b{}", default);
                    write!(self.body, "  br i1 {}, label %{}, label %{}\n", cond_bool, true_label, false_label).unwrap();
                }
                self.block_terminated = true;
                Ok(())
            }
            Terminator::Return(Some(operand)) => {
                let val = self.emit_mir_operand(operand)?;
                write!(self.body, "  ret i64 {}\n", val).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Terminator::Return(None) => {
                write!(self.body, "  ret i64 0\n").unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Terminator::Unreachable => {
                write!(self.body, "  unreachable\n").unwrap();
                self.block_terminated = true;
                Ok(())
            }
        }
    }

    fn emit_mir_print(&mut self, args: Vec<String>, is_println: bool) -> Result<(), String> {
        if args.is_empty() {
            let fmt_name = self.global_str("");
            let fmt_ptr = self.fresh();
            write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", fmt_ptr, fmt_name).unwrap();
            let str_ptr = self.fresh();
            write!(self.body, "  {} = inttoptr i64 {} to ptr\n", str_ptr, fmt_ptr).unwrap();
            let reg = self.fresh();
            if is_println {
                write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, str_ptr).unwrap();
            } else {
                write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, str_ptr).unwrap();
            }
        } else if args.len() == 1 {
            let arg = &args[0];
            if self.string_regs.contains(arg) || self.bool_regs.contains(arg) {
                // String/bool arg - use directly
                let str_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", str_ptr, arg).unwrap();
                let reg = self.fresh();
                if is_println {
                    write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, str_ptr).unwrap();
                } else {
                    write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, str_ptr).unwrap();
                }
            } else {
                // Numeric arg - format with snprintf
                let buf = self.fresh();
                write!(self.body, "  {} = alloca i8, i64 32\n", buf).unwrap();
                let fmt_name = self.global_str("%ld");
                let fmt_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", fmt_ptr, fmt_name).unwrap();
                let r = self.fresh();
                write!(self.body, "  {} = call i32 (ptr, i64, ptr, ...) @snprintf(ptr {}, i64 32, ptr {}, i64 {})\n", r, buf, fmt_ptr, arg).unwrap();
                let ptr_as_int = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_int, buf).unwrap();
                self.string_regs.insert(ptr_as_int.clone());
                let str_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", str_ptr, ptr_as_int).unwrap();
                let reg = self.fresh();
                if is_println {
                    write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, str_ptr).unwrap();
                } else {
                    write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, str_ptr).unwrap();
                }
            }
        } else {
            // Multiple args - concat strings first
            self.declare_extern("str_concat");
            let mut concat = args[0].clone();
            for arg in &args[1..] {
                let next = self.fresh();
                write!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})\n", next, concat, arg).unwrap();
                concat = next;
            }
            let str_ptr = self.fresh();
            write!(self.body, "  {} = inttoptr i64 {} to ptr\n", str_ptr, concat).unwrap();
            let reg = self.fresh();
            if is_println {
                write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, str_ptr).unwrap();
            } else {
                write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, str_ptr).unwrap();
            }
        }
        Ok(())
    }
}

fn hir_type_to_type_ann(t: &crate::hir::HirType) -> TypeAnnot {
    match t {
        crate::hir::HirType::Int => TypeAnnot::Int,
        crate::hir::HirType::Float => TypeAnnot::Float,
        crate::hir::HirType::Bool => TypeAnnot::Bool,
        crate::hir::HirType::Str => TypeAnnot::String,
        crate::hir::HirType::Void => TypeAnnot::Void,
        crate::hir::HirType::Named(s) => TypeAnnot::Class(s.clone()),
        crate::hir::HirType::Array(inner) => TypeAnnot::Array(Box::new(hir_type_to_type_ann(inner))),
        crate::hir::HirType::Generic(name, args) => {
            if args.is_empty() {
                TypeAnnot::Generic(name.clone())
            } else {
                TypeAnnot::Generic(name.clone())
            }
        }
        crate::hir::HirType::Unknown => TypeAnnot::Void,
    }
}


