use std::collections::HashMap;
use std::fmt::Write;
use crate::ast::{Stmt, TypeAnnot};
use super::Codegen;

impl Codegen {
    pub(super) fn emit_allocas_for_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Set { name, .. } | Stmt::Const { name, .. } => {
                    if !self.variables.contains_key(name) {
                        let reg = self.fresh();
                        write!(self.body, "  {} = alloca i64, align 8\n", reg).unwrap();
                        self.variables.insert(name.clone(), reg.clone());
                    }
                }
                Stmt::ForLoop { init, .. } => {
                    if let Stmt::Set { name, .. } = init.as_ref() {
                        if !self.variables.contains_key(name) {
                            let reg = self.fresh();
                            write!(self.body, "  {} = alloca i64, align 8\n", reg).unwrap();
                            self.variables.insert(name.clone(), reg.clone());
                        }
                    }
                }
                Stmt::FnDef { .. } => {
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

    pub(super) fn emit_fn_def(&mut self, name: &str, params: &[(String, TypeAnnot)], body: &[Stmt]) -> Result<(), String> {
        let saved_unnamed = self.unnamed_count;
        let saved_string_vars = self.string_vars.clone();
        let saved_string_regs = self.string_regs.clone();
        let saved_var_types = self.var_types.clone();
        let saved_bool_regs = self.bool_regs.clone();
        let saved_bool_vars = self.bool_vars.clone();
        let saved_float_regs = self.float_regs.clone();
        let saved_float_vars = self.float_vars.clone();
        let saved_array_regs = self.array_regs.clone();
        let saved_array_vars = self.array_vars.clone();
        let saved_enum_regs = self.enum_regs.clone();
        let saved_enum_vars = self.enum_vars.clone();
        self.string_regs.clear();
        self.bool_regs.clear();
        self.bool_vars.clear();
        self.float_regs.clear();
        self.float_vars.clear();
        self.array_regs.clear();
        self.array_vars.clear();
        self.enum_regs.clear();
        self.enum_vars.clear();
        self.unnamed_count = params.len() as u64 + 1;

        let mut fn_body = String::new();
        let mut fn_vars: HashMap<String, String> = HashMap::new();

        let mut params_ir = Vec::new();
        for _ in params {
            params_ir.push("i64".to_string());
        }
        let header = format!("define i64 @{}({}) {{\n", name, params_ir.join(", "));

        for (i, (pname, ptype)) in params.iter().enumerate() {
            let param_reg = format!("%{}", i);
            let reg = self.fresh();
            write!(fn_body, "  {} = alloca i64, align 8\n", reg).unwrap();
            write!(fn_body, "  store i64 {}, ptr {}\n", param_reg, reg).unwrap();
            fn_vars.insert(pname.clone(), reg);
            if matches!(ptype, TypeAnnot::String) {
                self.string_vars.insert(pname.clone());
            }
        }
        for s in body {
            self.collect_vars(s, &mut fn_vars, &mut fn_body);
        }

        let entry = format!("L{}_{}", self.label_count, name);
        self.label_count += 1;
        write!(fn_body, "  br label %{}\n", entry).unwrap();
        write!(fn_body, "{}:\n", entry).unwrap();
        self.block_terminated = false;

        let saved_body = std::mem::replace(&mut self.body, fn_body);
        let saved_vars = std::mem::replace(&mut self.variables, fn_vars);
        self.emit_stmts(body)?;
        if !self.block_terminated {
            write!(self.body, "  ret i64 0\n").unwrap();
        }
        self.block_terminated = false;
        let full_fn = format!("{}{}}}\n", header, self.body);
        self.functions.push_str(&full_fn);
        self.body = saved_body;
        self.variables = saved_vars;
        self.unnamed_count = saved_unnamed;
        self.string_vars = saved_string_vars;
        self.string_regs = saved_string_regs;
        self.var_types = saved_var_types;
        self.bool_regs = saved_bool_regs;
        self.bool_vars = saved_bool_vars;
        self.float_regs = saved_float_regs;
        self.float_vars = saved_float_vars;
        self.array_regs = saved_array_regs;
        self.array_vars = saved_array_vars;
        self.enum_regs = saved_enum_regs;
        self.enum_vars = saved_enum_vars;
        Ok(())
    }

    pub(super) fn collect_vars(&mut self, stmt: &Stmt, vars: &mut HashMap<String, String>, out: &mut String) {
        match stmt {
            Stmt::Set { name, .. } | Stmt::Const { name, .. } => {
                if !vars.contains_key(name) && !self.globals_map.contains_key(name) {
                    let reg = self.fresh();
                    write!(out, "  {} = alloca i64, align 8\n", reg).unwrap();
                    vars.insert(name.clone(), reg);
                }
            }
            Stmt::ForLoop { init, body, .. } => {
                if let Stmt::Set { name, .. } = init.as_ref() {
                    if !vars.contains_key(name) {
                        let reg = self.fresh();
                        write!(out, "  {} = alloca i64, align 8\n", reg).unwrap();
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

    pub(super) fn emit_stmts(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for stmt in stmts {
            self.emit_stmt(stmt)?;
        }
        Ok(())
    }

    pub(super) fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::FnDef { name, .. } if name == "main" => {
                Err("Nested main not supported".to_string())
            }
            Stmt::FnDef { name, .. } => {
                Err(format!("Nested function not supported: {}", name))
            }
            Stmt::Set { name, value, .. } => {
                let var_reg = if let Some(gname) = self.globals_map.get(name) {
                    format!("@{}", gname)
                } else {
                    self.variables.get(name).cloned()
                        .ok_or_else(|| format!("Unknown variable: {}", name))?
                };
                let val = self.emit_expr(value)?;
                write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                self.track_var_type(name, value);
                if self.string_regs.contains(&val) {
                    self.string_vars.insert(name.clone());
                }
                if self.bool_regs.contains(&val) {
                    self.bool_vars.insert(name.clone());
                }
                if self.array_regs.contains(&val) {
                    self.array_vars.insert(name.clone());
                }
                if self.enum_regs.contains(&val) {
                    self.enum_vars.insert(name.clone());
                }
                Ok(())
            }
            Stmt::Const { name, value, .. } => {
                let var_reg = if let Some(gname) = self.globals_map.get(name) {
                    format!("@{}", gname)
                } else {
                    self.variables.get(name).cloned()
                        .ok_or_else(|| format!("Unknown const: {}", name))?
                };
                let val = self.emit_expr(value)?;
                write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                self.track_var_type(name, value);
                if self.string_regs.contains(&val) {
                    self.string_vars.insert(name.clone());
                }
                if self.bool_regs.contains(&val) {
                    self.bool_vars.insert(name.clone());
                }
                if self.array_regs.contains(&val) {
                    self.array_vars.insert(name.clone());
                }
                if self.enum_regs.contains(&val) {
                    self.enum_vars.insert(name.clone());
                }
                Ok(())
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                write!(self.body, "  {} = icmp ne i64 {}, 0\n", cond_bool, cond_val).unwrap();
                let label_then = self.fresh_label();
                let label_else = self.fresh_label();
                let label_merge = self.fresh_label();
                write!(self.body, "  br i1 {}, label %{}, label %{}\n", cond_bool, label_then, label_else).unwrap();
                write!(self.body, "{}:\n", label_then).unwrap();
                self.block_terminated = false;
                self.emit_stmts(then_block)?;
                if !self.block_terminated {
                    write!(self.body, "  br label %{}\n", label_merge).unwrap();
                }
                write!(self.body, "{}:\n", label_else).unwrap();
                self.block_terminated = false;
                if let Some(eb) = else_block {
                    self.emit_stmts(eb)?;
                }
                if !self.block_terminated {
                    write!(self.body, "  br label %{}\n", label_merge).unwrap();
                }
                write!(self.body, "{}:\n", label_merge).unwrap();
                self.block_terminated = false;
                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                let label_header = self.fresh_label();
                let label_body = self.fresh_label();
                let label_exit = self.fresh_label();
                self.loops.push((label_header.clone(), label_exit.clone()));
                write!(self.body, "  br label %{}\n", label_header).unwrap();
                write!(self.body, "{}:\n", label_header).unwrap();
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                write!(self.body, "  {} = icmp ne i64 {}, 0\n", cond_bool, cond_val).unwrap();
                write!(self.body, "  br i1 {}, label %{}, label %{}\n", cond_bool, label_body, label_exit).unwrap();
                write!(self.body, "{}:\n", label_body).unwrap();
                self.emit_stmts(body)?;
                write!(self.body, "  br label %{}\n", label_header).unwrap();
                write!(self.body, "{}:\n", label_exit).unwrap();
                self.loops.pop();
                Ok(())
            }
            Stmt::ForLoop { init, condition, update, body, .. } => {
                self.emit_stmt(init)?;
                let label_header = self.fresh_label();
                let label_body = self.fresh_label();
                let label_update = self.fresh_label();
                let label_exit = self.fresh_label();
                self.loops.push((label_update.clone(), label_exit.clone()));
                write!(self.body, "  br label %{}\n", label_header).unwrap();
                write!(self.body, "{}:\n", label_header).unwrap();
                let cond_val = self.emit_expr(condition)?;
                let cond_bool = self.fresh();
                write!(self.body, "  {} = icmp ne i64 {}, 0\n", cond_bool, cond_val).unwrap();
                write!(self.body, "  br i1 {}, label %{}, label %{}\n", cond_bool, label_body, label_exit).unwrap();
                write!(self.body, "{}:\n", label_body).unwrap();
                self.emit_stmts(body)?;
                write!(self.body, "  br label %{}\n", label_update).unwrap();
                write!(self.body, "{}:\n", label_update).unwrap();
                self.emit_stmt(update)?;
                write!(self.body, "  br label %{}\n", label_header).unwrap();
                write!(self.body, "{}:\n", label_exit).unwrap();
                self.loops.pop();
                Ok(())
            }
            Stmt::Break { .. } => {
                let exit = self.loops.last().ok_or("break outside loop")?.1.clone();
                write!(self.body, "  br label %{}\n", exit).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Stmt::Continue { .. } => {
                let header = self.loops.last().ok_or("continue outside loop")?.0.clone();
                write!(self.body, "  br label %{}\n", header).unwrap();
                self.block_terminated = true;
                Ok(())
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    let val = self.emit_expr(v)?;
                    write!(self.body, "  ret i64 {}\n", val).unwrap();
                } else {
                    write!(self.body, "  ret i64 0\n").unwrap();
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
}
