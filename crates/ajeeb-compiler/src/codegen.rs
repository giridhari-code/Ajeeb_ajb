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

    fn global_str(&mut self, s: &str) -> String {
        let idx = self.str_count;
        self.str_count += 1;
        let name = format!(".str.{}", idx);
        // LLVM IR string constants use \XX (two hex digits, NO 'x' prefix) for escapes.
        // \" is NOT supported (treats backslash as literal, quote closes the string).
        // We emit printable chars as-is; everything else as \XX.
        let mut escaped = String::new();
        for b in s.bytes() {
            match b {
                b'\\' => escaped.push_str("\\\\"),
                b'"'  => escaped.push_str("\\22"),
                0x20..=0x7e => escaped.push(b as char),
                _ => write!(escaped, "\\{:02x}", b).unwrap(),
            }
        }
        writeln!(self.globals, "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            name, s.len() + 1, escaped).unwrap();
        name
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

    // Track variable types for method dispatch
    fn track_var_type(&mut self, name: &str, init_expr: &Expr) {
        match init_expr {
            Expr::StructLit { struct_name, .. } => {
                // Strip generic type args: "Box[Int]" -> "Box"
                let base_name = if let Some(bracket_pos) = struct_name.find('[') {
                    struct_name[..bracket_pos].to_string()
                } else {
                    struct_name.clone()
                };
                self.var_types.insert(name.to_string(), ("struct".into(), base_name));
            }
            Expr::EnumCtor { enum_name, .. } | Expr::EnumRef { enum_name, .. } => {
                // Strip generic type args: "Option[Int]" -> "Option"
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    enum_name[..bracket_pos].to_string()
                } else {
                    enum_name.clone()
                };
                self.var_types.insert(name.to_string(), ("enum".into(), base_name));
            }
            _ => {}
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

    fn emit_allocas_for_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                    if !self.variables.contains_key(name) {
                        let reg = self.fresh();
                        writeln!(self.body, "  {} = alloca i64, align 8", reg).unwrap();
                        self.variables.insert(name.clone(), reg.clone());
                    }
                }
                Stmt::ForLoop { init, .. } => {
                    if let Stmt::Let { name, .. } = init.as_ref() {
                        if !self.variables.contains_key(name) {
                            let reg = self.fresh();
                            writeln!(self.body, "  {} = alloca i64, align 8", reg).unwrap();
                            self.variables.insert(name.clone(), reg.clone());
                        }
                    }
                }
                Stmt::FnDef { .. } => {
                    // FnDefs are handled separately in compile()
                }
                Stmt::If { then_block, else_block, .. } => {
                    self.emit_allocas_for_stmts(then_block);
                    if let Some(eb) = else_block {
                        self.emit_allocas_for_stmts(eb);
                    }
                }
                Stmt::While { body, .. } => {
                    self.emit_allocas_for_stmts(body);
                }
                _ => {}
            }
        }
    }

    fn emit_fn_def(&mut self, name: &str, params: &[(String, TypeAnnot)], body: &[Stmt]) -> Result<(), String> {
        // Save global state; restore after function
        let saved_unnamed = self.unnamed_count;
        let saved_string_vars = self.string_vars.clone();
        let saved_string_regs = self.string_regs.clone();
        self.string_regs.clear();
        // In LLVM IR 21+, %0 is reserved internally (function ref?).
        // With N params, first numbered instruction is %(N+1).
        self.unnamed_count = params.len() as u64 + 1;

        let mut fn_body = String::new();
        let mut fn_vars: HashMap<String, String> = HashMap::new();

        let mut params_ir = Vec::new();
        for _ in params {
            params_ir.push("i64".to_string());
        }
        // Header — unnamed params (%0, %1, ...)
        let header = format!("define i64 @{}({}) {{\n", name, params_ir.join(", "));

        // Allocas for parameters — param i is %i
        for (i, (pname, ptype)) in params.iter().enumerate() {
            let param_reg = format!("%{}", i);
            let reg = self.fresh();
            writeln!(fn_body, "  {} = alloca i64, align 8", reg).unwrap();
            writeln!(fn_body, "  store i64 {}, ptr {}", param_reg, reg).unwrap();
            fn_vars.insert(pname.clone(), reg);
            // Track string parameters
            if matches!(ptype, TypeAnnot::String) {
                self.string_vars.insert(pname.clone());
            }
        }
        // Allocas for local variables
        for s in body {
            self.collect_vars(s, &mut fn_vars, &mut fn_body);
        }

        let entry = format!("L{}_{}", self.label_count, name);
        self.label_count += 1;
        writeln!(fn_body, "  br label %{}", entry).unwrap();
        writeln!(fn_body, "{}:", entry).unwrap();
        self.block_terminated = false;

        // Emit body statements
        let saved_body = std::mem::replace(&mut self.body, fn_body);
        let saved_vars = std::mem::replace(&mut self.variables, fn_vars);
        self.emit_stmts(body)?;
        if !self.block_terminated {
            writeln!(self.body, "  ret i64 0").unwrap();
        }
        self.block_terminated = false;
        let full_fn = format!("{}{}}}\n", header, self.body);
        self.functions.push_str(&full_fn);
        self.body = saved_body;
        self.variables = saved_vars;
        self.unnamed_count = saved_unnamed;
        self.string_vars = saved_string_vars;
        self.string_regs = saved_string_regs;
        Ok(())
    }

    fn collect_vars(&mut self, stmt: &Stmt, vars: &mut HashMap<String, String>, out: &mut String) {
        match stmt {
            Stmt::Let { name, .. } | Stmt::Const { name, .. } => {
                if !vars.contains_key(name) {
                    let reg = self.fresh();
                    writeln!(out, "  {} = alloca i64, align 8", reg).unwrap();
                    vars.insert(name.clone(), reg);
                }
            }
            Stmt::ForLoop { init, body, .. } => {
                if let Stmt::Let { name, .. } = init.as_ref() {
                    if !vars.contains_key(name) {
                        let reg = self.fresh();
                        writeln!(out, "  {} = alloca i64, align 8", reg).unwrap();
                        vars.insert(name.clone(), reg);
                    }
                }
                for s in body {
                    self.collect_vars(s, vars, out);
                }
            }
            Stmt::If { then_block, else_block, .. } => {
                for s in then_block {
                    self.collect_vars(s, vars, out);
                }
                if let Some(eb) = else_block {
                    for s in eb {
                        self.collect_vars(s, vars, out);
                    }
                }
            }
            Stmt::While { body, .. } => {
                for s in body {
                    self.collect_vars(s, vars, out);
                }
            }
            _ => {}
        }
    }

    fn emit_stmts(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for stmt in stmts {
            self.emit_stmt(stmt)?;
        }
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::FnDef { name, .. } if name == "main" => {
                Err("Nested main not supported".to_string())
            }
            Stmt::FnDef { name, .. } => {
                Err(format!("Nested function not supported: {}", name))
            }
            Stmt::Let { name, value, .. } => {
                let var_reg = self.variables.get(name).cloned()
                    .ok_or_else(|| format!("Unknown variable: {}", name))?;
                let val = self.emit_expr(value)?;
                writeln!(self.body, "  store i64 {}, ptr {}", val, var_reg).unwrap();
                // Track type info for method dispatch
                self.track_var_type(name, value);
                // Track string type for correct binary ops
                if self.string_regs.contains(&val) {
                    self.string_vars.insert(name.clone());
                }
                Ok(())
            }
            Stmt::Const { name, value, .. } => {
                let var_reg = self.variables.get(name).cloned()
                    .ok_or_else(|| format!("Unknown const: {}", name))?;
                let val = self.emit_expr(value)?;
                writeln!(self.body, "  store i64 {}, ptr {}", val, var_reg).unwrap();
                self.track_var_type(name, value);
                // Track string type for correct binary ops
                if self.string_regs.contains(&val) {
                    self.string_vars.insert(name.clone());
                }
                Ok(())
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                writeln!(self.body, "  {} = icmp ne i64 {}, 0", cond_bool, cond_val).unwrap();
                let label_then = self.fresh_label();
                let label_else = self.fresh_label();
                let label_merge = self.fresh_label();
                writeln!(self.body, "  br i1 {}, label %{}, label %{}", cond_bool, label_then, label_else).unwrap();
                writeln!(self.body, "{}:", label_then).unwrap();
                self.block_terminated = false;
                self.emit_stmts(then_block)?;
                if !self.block_terminated {
                    writeln!(self.body, "  br label %{}", label_merge).unwrap();
                }
                writeln!(self.body, "{}:", label_else).unwrap();
                self.block_terminated = false;
                if let Some(eb) = else_block {
                    self.emit_stmts(eb)?;
                }
                if !self.block_terminated {
                    writeln!(self.body, "  br label %{}", label_merge).unwrap();
                }
                writeln!(self.body, "{}:", label_merge).unwrap();
                self.block_terminated = false;
                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                let label_header = self.fresh_label();
                let label_body = self.fresh_label();
                let label_exit = self.fresh_label();
                self.loops.push((label_header.clone(), label_exit.clone()));
                writeln!(self.body, "  br label %{}", label_header).unwrap();
                writeln!(self.body, "{}:", label_header).unwrap();
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                writeln!(self.body, "  {} = icmp ne i64 {}, 0", cond_bool, cond_val).unwrap();
                writeln!(self.body, "  br i1 {}, label %{}, label %{}", cond_bool, label_body, label_exit).unwrap();
                writeln!(self.body, "{}:", label_body).unwrap();
                self.emit_stmts(body)?;
                writeln!(self.body, "  br label %{}", label_header).unwrap();
                writeln!(self.body, "{}:", label_exit).unwrap();
                self.loops.pop();
                Ok(())
            }
            Stmt::ForLoop { init, condition, update, body, .. } => {
                // Desugar to: init; while(cond) { body; update; }
                self.emit_stmt(init)?;
                let label_header = self.fresh_label();
                let label_body = self.fresh_label();
                let label_update = self.fresh_label();
                let label_exit = self.fresh_label();
                // For for-loops, continue should go to update step
                self.loops.push((label_update.clone(), label_exit.clone()));
                writeln!(self.body, "  br label %{}", label_header).unwrap();
                writeln!(self.body, "{}:", label_header).unwrap();
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                writeln!(self.body, "  {} = icmp ne i64 {}, 0", cond_bool, cond_val).unwrap();
                writeln!(self.body, "  br i1 {}, label %{}, label %{}", cond_bool, label_body, label_exit).unwrap();
                writeln!(self.body, "{}:", label_body).unwrap();
                self.emit_stmts(body)?;
                writeln!(self.body, "  br label %{}", label_update).unwrap();
                writeln!(self.body, "{}:", label_update).unwrap();
                self.emit_stmt(update)?;
                writeln!(self.body, "  br label %{}", label_header).unwrap();
                writeln!(self.body, "{}:", label_exit).unwrap();
                self.loops.pop();
                Ok(())
            }
            Stmt::Break { .. } => {
                let exit = self.loops.last().ok_or("break outside loop")?.1.clone();
                writeln!(self.body, "  br label %{}", exit).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Stmt::Continue { .. } => {
                let header = self.loops.last().ok_or("continue outside loop")?.0.clone();
                writeln!(self.body, "  br label %{}", header).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    let val = self.emit_expr(v)?;
                    writeln!(self.body, "  ret i64 {}", val).unwrap();
                } else {
                    writeln!(self.body, "  ret i64 0").unwrap();
                }
                self.block_terminated = true;
                Ok(())
            }
            Stmt::Expr(expr, ..) => {
                self.emit_expr(expr)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    // Determine which struct a field access expression refers to,
    // then return the positional offset for that field within that struct.
    fn resolve_field_type(&self, obj: &Expr, field: &str) -> Option<TypeAnnot> {
        // Step 1: resolve the struct type name (same logic as resolve_field_offset)
        let struct_type: Option<String> = match obj {
            Expr::StructLit { struct_name, .. } => {
                let base = struct_name.split('[').next().unwrap_or(struct_name);
                Some(base.to_string())
            }
            Expr::Ident(name, ..) => {
                self.var_types.get(name).and_then(|(kind, tn)| {
                    if kind == "struct" { Some(tn.clone()) } else { None }
                })
            }
            Expr::Field { obj: inner, field: inner_field, .. } => {
                // Chain: look up parent's struct type, then find field's type
                let inner_type = match inner.as_ref() {
                    Expr::Ident(v, ..) => self.var_types.get(v).map(|(_, tn)| tn.clone()),
                    _ => None,
                };
                inner_type.and_then(|tn| {
                    self.struct_defs.get(&tn)
                        .and_then(|fields| fields.iter().find(|(n, _)| n == inner_field))
                        .map(|(_, ty)| match ty {
                            TypeAnnot::Class(s) | TypeAnnot::Generic(s) => s.clone(),
                            _ => String::new(),
                        })
                })
            }
            _ => None,
        };
        // Step 2: look up the field's TypeAnnot in that struct's definition
        struct_type.and_then(|st| {
            self.struct_defs.get(&st)
                .and_then(|fields| fields.iter().find(|(n, _)| n == field))
                .map(|(_, ty)| ty.clone())
        })
    }

    // Determine which struct a field access expression refers to,
    // then return the positional offset for that field within that struct.
    fn resolve_field_offset(&self, obj: &Expr, field: &str) -> Option<usize> {
        // Try to determine the struct type from the object expression
        let struct_type = match obj {
            Expr::StructLit { struct_name, .. } => {
                let base = struct_name.split('[').next().unwrap_or(struct_name);
                Some(base.to_string())
            }
            Expr::Ident(name, ..) => {
                self.var_types.get(name).and_then(|(kind, tn)| {
                    if kind == "struct" { Some(tn.clone()) } else { None }
                })
            }
            Expr::Field { obj: inner, field: inner_field, .. } => {
                // Chain: look up parent's struct type, then find field's type
                let inner_type = match inner.as_ref() {
                    Expr::Ident(v, ..) => self.var_types.get(v).map(|(_, tn)| tn.clone()),
                    _ => None,
                };
                inner_type.and_then(|tn| {
                    self.struct_defs.get(&tn)
                        .and_then(|fields| fields.iter().find(|(n, _)| n == inner_field))
                        .map(|(_, ty)| match ty {
                            TypeAnnot::Class(s) | TypeAnnot::Generic(s) => s.clone(),
                            _ => String::new(),
                        })
                })
            }
            _ => None,
        };
        // Use the specific struct's field list; fall back to searching all structs
        if let Some(st) = struct_type {
            self.struct_defs.get(&st)
                .and_then(|fields| fields.iter().position(|(n, _)| n == field))
        } else {
            self.struct_defs.iter()
                .find_map(|(_, fields)| fields.iter().position(|(n, _)| n == field))
        }
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Number(n, ..) => {
                let reg = self.fresh();
                writeln!(self.body, "  {} = add i64 0, {}", reg, n).unwrap();
                Ok(reg)
            }
            Expr::FloatLit(f, ..) => {
                let reg = self.fresh();
                let bits = f.to_bits();
                writeln!(self.body, "  {} = bitcast i64 {} to double", reg, bits).unwrap();
                Ok(reg)
            }
            Expr::Bool(b, ..) => {
                let reg = self.fresh();
                writeln!(self.body, "  {} = add i64 0, {}", reg, if *b { 1 } else { 0 }).unwrap();
                Ok(reg)
            }
            Expr::StringLit(s, ..) => {
                let gname = self.global_str(s);
                let ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0", ptr, gname).unwrap();
                let reg = self.fresh();
                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", reg, ptr).unwrap();
                self.string_regs.insert(reg.clone());
                Ok(reg)
            }
            Expr::Ident(name, ..) => {
                if let Some(var_reg) = self.variables.get(name).cloned() {
                    let reg = self.fresh();
                    writeln!(self.body, "  {} = load i64, ptr {}", reg, var_reg).unwrap();
                    if self.string_vars.contains(name) {
                        self.string_regs.insert(reg.clone());
                    }
                    Ok(reg)
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    // Top-level global variable
                    let reg = self.fresh();
                    writeln!(self.body, "  {} = load i64, ptr @{}", reg, gname).unwrap();
                    if self.string_vars.contains(name) {
                        self.string_regs.insert(reg.clone());
                    }
                    Ok(reg)
                } else if name == "__str_ptr" {
                    let reg = self.fresh();
                    writeln!(self.body, "  {} = add i64 0, 0", reg).unwrap();
                    Ok(reg)
                } else {
                    Err(format!("Undefined variable: {}", name))
                }
            }
            Expr::Binary { left, op, right, .. } => {
                let lhs = self.emit_expr(left)?;
                let rhs = self.emit_expr(right)?;
                let reg = self.fresh();
                match op {
                    BinOp::Add => {
                        // Detect string concatenation: if either operand is a string literal
                        // or is a known string register, call str_concat from the C runtime.
                        // Otherwise, integer addition.
                        let is_str = matches!(left.as_ref(), Expr::StringLit(..))
                            || matches!(right.as_ref(), Expr::StringLit(..))
                            || self.string_regs.contains(&lhs)
                            || self.string_regs.contains(&rhs);
                        if is_str {
                            self.declare_extern("str_concat");
                            let reg2 = self.fresh();
                            writeln!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})", reg2, lhs, rhs).unwrap();
                            self.string_regs.insert(reg2.clone());
                            return Ok(reg2);
                        }
                        writeln!(self.body, "  {} = add i64 {}, {}", reg, lhs, rhs).unwrap();
                    }
                    BinOp::Sub => writeln!(self.body, "  {} = sub i64 {}, {}", reg, lhs, rhs).unwrap(),
                    BinOp::Mul => writeln!(self.body, "  {} = mul i64 {}, {}", reg, lhs, rhs).unwrap(),
                    BinOp::Div => {
                        // Safe division: guard against zero divisor (interpreter returns 0)
                        let is_zero = self.fresh();
                        writeln!(self.body, "  {} = icmp eq i64 {}, 0", is_zero, rhs).unwrap();
                        let safe_rhs = self.fresh();
                        writeln!(self.body, "  {} = select i1 {}, i64 1, i64 {}", safe_rhs, is_zero, rhs).unwrap();
                        let div_raw = self.fresh();
                        writeln!(self.body, "  {} = sdiv i64 {}, {}", div_raw, lhs, safe_rhs).unwrap();
                        let final_reg = self.fresh();
                        writeln!(self.body, "  {} = select i1 {}, i64 0, i64 {}", final_reg, is_zero, div_raw).unwrap();
                        return Ok(final_reg);
                    }
                    BinOp::Eq => {
                        // Use reg for icmp (allocated first), then zext gets the next register
                        writeln!(self.body, "  {} = icmp eq i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Neq => {
                        writeln!(self.body, "  {} = icmp ne i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Lt => {
                        writeln!(self.body, "  {} = icmp slt i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Gt => {
                        writeln!(self.body, "  {} = icmp sgt i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Le => {
                        writeln!(self.body, "  {} = icmp sle i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Ge => {
                        writeln!(self.body, "  {} = icmp sge i64 {}, {}", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        writeln!(self.body, "  {} = zext i1 {} to i64", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::And => writeln!(self.body, "  {} = and i64 {}, {}", reg, lhs, rhs).unwrap(),
                    BinOp::Or => writeln!(self.body, "  {} = or i64 {}, {}", reg, lhs, rhs).unwrap(),
                }
                Ok(reg)
            }
            Expr::Assign { name, value, .. } => {
                let val = self.emit_expr(value)?;
                if let Some(var_reg) = self.variables.get(name) {
                    writeln!(self.body, "  store i64 {}, ptr {}", val, var_reg).unwrap();
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    writeln!(self.body, "  store i64 {}, ptr @{}", val, gname).unwrap();
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
                Ok(val)
            }
            Expr::UnaryNot(val, ..) => {
                let v = self.emit_expr(val)?;
                let cmp = self.fresh();
                writeln!(self.body, "  {} = icmp eq i64 {}, 0", cmp, v).unwrap();
                let reg = self.fresh();
                writeln!(self.body, "  {} = zext i1 {} to i64", reg, cmp).unwrap();
                Ok(reg)
            }
            Expr::UnaryMinus(val, ..) => {
                let v = self.emit_expr(val)?;
                let reg = self.fresh();
                writeln!(self.body, "  {} = sub i64 0, {}", reg, v).unwrap();
                Ok(reg)
            }
            Expr::Group(val, ..) => {
                self.emit_expr(val)
            }
            Expr::FnCall { name, args, .. } => {
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.emit_expr(arg)?);
                }
                match name.as_str() {
                    "println" | "print" => {
                        let is_println = *name == "println";
                        // Convert non-string arguments to strings via itoa
                        let mut string_args = Vec::new();
                        for (i, arg_reg) in compiled_args.iter().enumerate() {
                            if self.string_regs.contains(arg_reg) {
                                string_args.push(arg_reg.clone());
                            } else {
                                // Wrap integer arg in itoa call
                                let buf = self.fresh();
                                writeln!(self.body, "  {} = alloca i8, i64 32", buf).unwrap();
                                let fmt_name = self.global_str("%ld");
                                let fmt_ptr = self.fresh();
                                writeln!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0", fmt_ptr, fmt_name).unwrap();
                                let r = self.fresh();
                                writeln!(self.body, "  {} = call i32 (ptr, i64, ptr, ...) @snprintf(ptr {}, i64 32, ptr {}, i64 {})", r, buf, fmt_ptr, arg_reg).unwrap();
                                let ptr_as_int = self.fresh();
                                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_int, buf).unwrap();
                                self.string_regs.insert(ptr_as_int.clone());
                                string_args.push(ptr_as_int);
                            }
                        }
                        // Concatenate all string arguments
                        if string_args.is_empty() {
                            let fmt_name = self.global_str("");
                            let fmt_ptr = self.fresh();
                            writeln!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0", fmt_ptr, fmt_name).unwrap();
                            let str_ptr = self.fresh();
                            writeln!(self.body, "  {} = inttoptr i64 {} to ptr", str_ptr, fmt_ptr).unwrap();
                            let reg = self.fresh();
                            if is_println {
                                writeln!(self.body, "  {} = call i32 @puts(ptr {})", reg, str_ptr).unwrap();
                            } else {
                                writeln!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})", reg, str_ptr).unwrap();
                            }
                        } else {
                            self.declare_extern("str_concat");
                            let mut concat = string_args[0].clone();
                            for arg in &string_args[1..] {
                                let next_concat = self.fresh();
                                writeln!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})", next_concat, concat, arg).unwrap();
                                concat = next_concat;
                            }
                            let str_ptr = self.fresh();
                            writeln!(self.body, "  {} = inttoptr i64 {} to ptr", str_ptr, concat).unwrap();
                            let reg = self.fresh();
                            if is_println {
                                writeln!(self.body, "  {} = call i32 @puts(ptr {})", reg, str_ptr).unwrap();
                            } else {
                                writeln!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})", reg, str_ptr).unwrap();
                            }
                        }
                        let reg = self.fresh();
                        writeln!(self.body, "  {} = add i64 0, 0", reg).unwrap();
                        Ok(reg)
                    }
                    "itoa" => {
                        let val = compiled_args.first().ok_or("itoa expects 1 argument")?;
                        let buf = self.fresh();
                        writeln!(self.body, "  {} = alloca i8, i64 32", buf).unwrap();
                        let fmt_name = self.global_str("%ld");
                        let fmt_ptr = self.fresh();
                        writeln!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0", fmt_ptr, fmt_name).unwrap();
                        let reg = self.fresh();
                        writeln!(self.body, "  {} = call i32 (ptr, i64, ptr, ...) @snprintf(ptr {}, i64 32, ptr {}, i64 {})", reg, buf, fmt_ptr, val).unwrap();
                        // Return pointer as i64
                        let ptr_as_int = self.fresh();
                        writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_int, buf).unwrap();
                        self.string_regs.insert(ptr_as_int.clone());
                        Ok(ptr_as_int)
                    }
                    _ => {
                        if !self.declare_extern(name) && !self.user_fns.contains(name.as_str()) {
                            return Err(format!("LLVM codegen not supported for interpreter builtin: {}", name));
                        }
                        let args_str = compiled_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                        // Void-returning C runtime functions
                        if matches!(name.as_str(), "setInt" | "strSet" | "writeFile" | "writeAppend" | "writeByte" | "tcp_write" | "tcp_close" | "tls_write" | "tls_close") {
                            writeln!(self.body, "  call void @{}({})", name, args_str).unwrap();
                            let reg = self.fresh();
                            writeln!(self.body, "  {} = add i64 0, 0", reg).unwrap();
                            Ok(reg)
                        } else {
                            let reg = self.fresh();
                            writeln!(self.body, "  {} = call i64 @{}({})", reg, name, args_str).unwrap();
                            // Track string-returning builtins
                            if matches!(name.as_str(),
                                "str_concat" | "itoa" | "substring" | "toUpperCase" | "toLowerCase"
                                | "trim" | "readFile" | "readArg" | "replace" | "chr"
                            ) {
                                self.string_regs.insert(reg.clone());
                            }
                            Ok(reg)
                        }
                    }
                }
            }
            Expr::ArrayLit(items, ..) => {
                if items.is_empty() {
                    let reg = self.fresh();
                    writeln!(self.body, "  {} = add i64 0, 0", reg).unwrap();
                    return Ok(reg);
                }
                // Allocate array on stack and store each element
                let count = items.len() as u64;
                let arr_ptr = self.fresh();
                writeln!(self.body, "  {} = alloca i64, i64 {}", arr_ptr, count).unwrap();
                for (i, item) in items.iter().enumerate() {
                    let val = self.emit_expr(item)?;
                    let elem_ptr = self.fresh();
                    writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, arr_ptr, i).unwrap();
                    writeln!(self.body, "  store i64 {}, ptr {}", val, elem_ptr).unwrap();
                }
                // Convert pointer to i64 for uniform value type
                let ptr_as_i64 = self.fresh();
                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_i64, arr_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Index { obj, index, .. } => {
                let arr_val = self.emit_expr(obj)?;
                let idx = self.emit_expr(index)?;
                let arr_ptr = self.fresh();
                writeln!(self.body, "  {} = inttoptr i64 {} to ptr", arr_ptr, arr_val).unwrap();
                let elem_ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, arr_ptr, idx).unwrap();
                let reg = self.fresh();
                writeln!(self.body, "  {} = load i64, ptr {}", reg, elem_ptr).unwrap();
                Ok(reg)
            }
            Expr::IndexAssign { obj, index, value, .. } => {
                let arr_val = self.emit_expr(obj)?;
                let idx = self.emit_expr(index)?;
                let val = self.emit_expr(value)?;
                let arr_ptr = self.fresh();
                writeln!(self.body, "  {} = inttoptr i64 {} to ptr", arr_ptr, arr_val).unwrap();
                let elem_ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, arr_ptr, idx).unwrap();
                writeln!(self.body, "  store i64 {}, ptr {}", val, elem_ptr).unwrap();
                Ok(val)
            }
            Expr::StructLit { struct_name, fields, .. } => {
                // Allocate memory for struct fields
                let field_count = fields.len() as u64;
                let struct_ptr = self.fresh();
                writeln!(self.body, "  {} = alloca i64, i64 {}", struct_ptr, field_count.max(1)).unwrap();
                // Store each field at its position
                for (i, (fname, fexpr)) in fields.iter().enumerate() {
                    let fval = self.emit_expr(fexpr)?;
                    let elem_ptr = self.fresh();
                    writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, struct_ptr, i).unwrap();
                    writeln!(self.body, "  store i64 {}, ptr {}", fval, elem_ptr).unwrap();
                }
                let ptr_as_i64 = self.fresh();
                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_i64, struct_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Field { obj, field, .. } => {
                let obj_val = self.emit_expr(obj)?;
                let obj_ptr = self.fresh();
                writeln!(self.body, "  {} = inttoptr i64 {} to ptr", obj_ptr, obj_val).unwrap();
                // Determine field offset from the specific struct type
                let offset = self.resolve_field_offset(obj, field).unwrap_or(0);
                let elem_ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, obj_ptr, offset).unwrap();
                let reg = self.fresh();
                writeln!(self.body, "  {} = load i64, ptr {}", reg, elem_ptr).unwrap();
                // Track string-typed fields
                if let Some(ty) = self.resolve_field_type(obj, field) {
                    if matches!(ty, TypeAnnot::String) {
                        self.string_regs.insert(reg.clone());
                    }
                }
                Ok(reg)
            }
            Expr::FieldAssign { obj, field, value, .. } => {
                let val = self.emit_expr(value)?;
                let obj_val = self.emit_expr(obj)?;
                let obj_ptr = self.fresh();
                writeln!(self.body, "  {} = inttoptr i64 {} to ptr", obj_ptr, obj_val).unwrap();
                let offset = self.resolve_field_offset(obj, field).unwrap_or(0);
                let elem_ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, obj_ptr, offset).unwrap();
                writeln!(self.body, "  store i64 {}, ptr {}", val, elem_ptr).unwrap();
                // If obj is an Ident, re-store the whole struct to the variable
                if let Expr::Ident(var_name, ..) = obj.as_ref() {
                    if let Some(var_reg) = self.variables.get(var_name) {
                        writeln!(self.body, "  store i64 {}, ptr {}", obj_val, var_reg).unwrap();
                    }
                }
                Ok(val)
            }
            Expr::MethodCall { obj, method, args, .. } => {
                // Evaluate receiver first
                let obj_val = self.emit_expr(obj)?;
                // Determine receiver type from the object expression
                let receiver_type = match obj.as_ref() {
                    Expr::StructLit { struct_name, .. } => Some(struct_name.clone()),
                    Expr::EnumCtor { enum_name, .. } | Expr::EnumRef { enum_name, .. } => Some(enum_name.clone()),
                    Expr::Ident(var, ..) => self.var_types.get(var).map(|(_, tn)| tn.clone()),
                    Expr::Field { obj: inner, field, .. } => {
                        let inner_type = match inner.as_ref() {
                            Expr::Ident(v, ..) => self.var_types.get(v).map(|(_, tn)| tn.clone()),
                            _ => None,
                        };
                        inner_type.and_then(|tn| {
                            self.struct_defs.get(&tn)
                                .and_then(|fields| fields.iter().find(|(n, _)| n == field))
                                .map(|(_, ty)| match ty {
                                    TypeAnnot::Class(s) | TypeAnnot::Generic(s) => s.clone(),
                                    _ => String::new(),
                                })
                        })
                    }
                    _ => None,
                };
                if let Some(rt) = receiver_type {
                    // Check inherent methods first, then trait methods
                    let mangled = self.method_map.get(&(rt.clone(), method.clone())).cloned()
                        .or_else(|| {
                            self.method_map.iter()
                                .find(|(k, _)| k.0 == rt && k.1.starts_with(&format!("{}@", method)))
                                .map(|(_, v)| v.clone())
                        });
                    if let Some(mangled_name) = mangled {
                        let mut call_args = vec![obj_val];
                        for a in args {
                            call_args.push(self.emit_expr(a)?);
                        }
                        let args_str = call_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                        let reg = self.fresh();
                        writeln!(self.body, "  {} = call i64 @{}({})", reg, mangled_name, args_str).unwrap();
                        return Ok(reg);
                    }
                }
                Err(format!("LLVM codegen: cannot resolve method {} on receiver", method))
            }
            Expr::EnumCtor { enum_name, variant, args, .. } => {
                // Allocate memory for enum: [variant_id, data0, data1, ...]
                let payload_count = args.len() as u64;
                let enum_ptr = self.fresh();
                writeln!(self.body, "  {} = alloca i64, i64 {}", enum_ptr, (payload_count + 1).max(2)).unwrap();
                // Store variant ID at offset 0
                let tag_id = self.enum_variant_ids.get(&(enum_name.clone(), variant.clone())).copied().unwrap_or(0);
                writeln!(self.body, "  store i64 {}, ptr {}", tag_id, enum_ptr).unwrap();
                // Store payload values
                for (i, a) in args.iter().enumerate() {
                    let aval = self.emit_expr(a)?;
                    let elem_ptr = self.fresh();
                    writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", elem_ptr, enum_ptr, i + 1).unwrap();
                    writeln!(self.body, "  store i64 {}, ptr {}", aval, elem_ptr).unwrap();
                }
                let ptr_as_i64 = self.fresh();
                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_i64, enum_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::EnumRef { enum_name, variant, .. } => {
                let enum_ptr = self.fresh();
                writeln!(self.body, "  {} = alloca i64, i64 2", enum_ptr).unwrap();
                let tag_id = self.enum_variant_ids.get(&(enum_name.clone(), variant.clone())).copied().unwrap_or(0);
                writeln!(self.body, "  store i64 {}, ptr {}", tag_id, enum_ptr).unwrap();
                // Also store 0 at offset 1 (no payload data)
                let zero = self.fresh();
                writeln!(self.body, "  {} = add i64 0, 0", zero).unwrap();
                let elem_ptr = self.fresh();
                writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 1", elem_ptr, enum_ptr).unwrap();
                writeln!(self.body, "  store i64 {}, ptr {}", zero, elem_ptr).unwrap();
                let ptr_as_i64 = self.fresh();
                writeln!(self.body, "  {} = ptrtoint ptr {} to i64", ptr_as_i64, enum_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Match { value, arms, .. } => {
                let scrutinee_val = self.emit_expr(value)?;
                let result_ptr = self.fresh();
                writeln!(self.body, "  {} = alloca i64, align 8", result_ptr).unwrap();
                let default = self.fresh();
                writeln!(self.body, "  {} = add i64 0, 0", default).unwrap();
                writeln!(self.body, "  store i64 {}, ptr {}", default, result_ptr).unwrap();
                let merge_label = self.fresh_label();

                // Convert enum pointer
                let enum_ptr = self.fresh();
                writeln!(self.body, "  {} = inttoptr i64 {} to ptr", enum_ptr, scrutinee_val).unwrap();
                let tag_reg = self.fresh();
                writeln!(self.body, "  {} = load i64, ptr {}", tag_reg, enum_ptr).unwrap();

                let next_label = self.fresh_label();
                writeln!(self.body, "  br label %{}", next_label).unwrap();
                writeln!(self.body, "{}:", next_label).unwrap();

                for arm in arms {
                    let arm_label = self.fresh_label();
                    let fallthrough_label = self.fresh_label();

                    match &arm.pattern {
                        Pattern::Wildcard => {
                            // Always match: branch to arm body
                            writeln!(self.body, "  br label %{}", arm_label).unwrap();
                        }
                        Pattern::EnumVariant { enum_name, variant, bindings } => {
                            let expected_tag = self.enum_variant_ids
                                .get(&(enum_name.clone(), variant.clone()))
                                .copied().unwrap_or(-1);
                            let cmp = self.fresh();
                            writeln!(self.body, "  {} = icmp eq i64 {}, {}", cmp, tag_reg, expected_tag).unwrap();
                            writeln!(self.body, "  br i1 {}, label %{}, label %{}", cmp, arm_label, fallthrough_label).unwrap();
                        }
                        Pattern::Int(n) => {
                            let cmp = self.fresh();
                            writeln!(self.body, "  {} = icmp eq i64 {}, {}", cmp, scrutinee_val, n).unwrap();
                            writeln!(self.body, "  br i1 {}, label %{}, label %{}", cmp, arm_label, fallthrough_label).unwrap();
                        }
                        Pattern::String(s) => {
                            let sname = self.global_str(s);
                            let sptr = self.fresh();
                            writeln!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0", sptr, sname).unwrap();
                            let s_as_i64 = self.fresh();
                            writeln!(self.body, "  {} = ptrtoint ptr {} to i64", s_as_i64, sptr).unwrap();
                            self.declare_extern("strcmp_ajeeb");
                            let cmp_result = self.fresh();
                            writeln!(self.body, "  {} = call i64 @strcmp_ajeeb(i64 {}, i64 {})", cmp_result, scrutinee_val, s_as_i64).unwrap();
                            let cmp_bool = self.fresh();
                            writeln!(self.body, "  {} = icmp eq i64 {}, 0", cmp_bool, cmp_result).unwrap();
                            writeln!(self.body, "  br i1 {}, label %{}, label %{}", cmp_bool, arm_label, fallthrough_label).unwrap();
                        }
                    }

                    // Arm body
                    writeln!(self.body, "{}:", arm_label).unwrap();
                    // Bind pattern variables
                    if let Pattern::EnumVariant { bindings, .. } = &arm.pattern {
                        for (i, bname) in bindings.iter().enumerate() {
                            let offset = i + 1; // data starts at offset 1
                            let data_ptr = self.fresh();
                            writeln!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}", data_ptr, enum_ptr, offset).unwrap();
                            let data_val = self.fresh();
                            writeln!(self.body, "  {} = load i64, ptr {}", data_val, data_ptr).unwrap();
                            // Store binding into a local variable
                            let binding_alloca = self.fresh();
                            writeln!(self.body, "  {} = alloca i64, align 8", binding_alloca).unwrap();
                            writeln!(self.body, "  store i64 {}, ptr {}", data_val, binding_alloca).unwrap();
                            self.variables.insert(bname.clone(), binding_alloca);
                        }
                    }

                    // Emit arm body
                    if let Some(stmts) = &arm.body_block {
                        for s in stmts {
                            self.emit_stmt(s)?;
                        }
                    } else {
                        let arm_result = self.emit_expr(&arm.body)?;
                        writeln!(self.body, "  store i64 {}, ptr {}", arm_result, result_ptr).unwrap();
                    }

                    // Clean up binding variables
                    if let Pattern::EnumVariant { bindings, .. } = &arm.pattern {
                        for bname in bindings {
                            self.variables.remove(bname);
                        }
                    }

                    writeln!(self.body, "  br label %{}", merge_label).unwrap();

                    // Fallthrough label (check next arm)
                    if !matches!(arm.pattern, Pattern::Wildcard) {
                        writeln!(self.body, "{}:", fallthrough_label).unwrap();
                    }
                }

                // Only need fallthrough branch if no wildcard covered all cases
                let has_wildcard = arms.iter().any(|a| matches!(a.pattern, Pattern::Wildcard));
                if !has_wildcard {
                    writeln!(self.body, "  br label %{}", merge_label).unwrap();
                }
                writeln!(self.body, "{}:", merge_label).unwrap();
                let result = self.fresh();
                writeln!(self.body, "  {} = load i64, ptr {}", result, result_ptr).unwrap();
                Ok(result)
            }
            Expr::GenericCall { name, args, .. } => {
                // Strip type args: same as regular FnCall
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.emit_expr(arg)?);
                }
                if !self.declare_extern(name) && !self.user_fns.contains(name.as_str()) {
                    return Err(format!("LLVM codegen: unknown function {}", name));
                }
                let args_str = compiled_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                let reg = self.fresh();
                writeln!(self.body, "  {} = call i64 @{}({})", reg, name, args_str).unwrap();
                Ok(reg)
            }
            Expr::AssociatedFnCall { type_name, method, args, .. } => {
                let base_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                let mangled = format!("{}_{}", base_name, method);
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.emit_expr(arg)?);
                }
                if !self.user_fns.contains(mangled.as_str()) {
                    return Err(format!("LLVM codegen: unknown associated function '{}::{}'", type_name, method));
                }
                let args_str = compiled_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                let reg = self.fresh();
                writeln!(self.body, "  {} = call i64 @{}({})", reg, mangled, args_str).unwrap();
                Ok(reg)
            }
            _ => Err(format!("Unsupported expression: {:?}", expr)),
        }
    }
}
