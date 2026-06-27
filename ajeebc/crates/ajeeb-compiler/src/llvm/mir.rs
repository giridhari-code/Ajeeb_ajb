use std::collections::HashMap;
use std::fmt::Write;
use crate::ast::{Expr, Stmt, TypeAnnot};
use crate::hir::HirType;
use crate::mir::{self as mir_mod, MirBinOp, MirConst, MirOperand, MirProgram, MirRvalue, MirStmt, Terminator};
use super::Codegen;

impl Codegen {
    pub fn compile_mir(&mut self, prog: &MirProgram) -> Result<String, String> {
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

        for f in &prog.functions {
            self.user_fns.insert(f.name.clone());
            self.fn_return_types.insert(f.name.clone(), hir_type_to_type_ann(&f.return_type));
        }

        // Emit main last so all called functions are defined before it.
        // This avoids the need for forward declarations (LLVM doesn't allow
        // both 'declare' and 'define' for the same function).
        let mut non_main: Vec<&mir_mod::MirFn> = prog.functions.iter().filter(|f| f.name != "main").collect();
        let main_fn: Option<&mir_mod::MirFn> = prog.functions.iter().find(|f| f.name == "main");
        for f in &non_main {
            self.emit_mir_fn(f)?;
        }
        if let Some(m) = main_fn {
            self.emit_mir_fn(m)?;
        }

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

    pub(super) fn emit_mir_fn(&mut self, f: &mir_mod::MirFn) -> Result<(), String> {
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
        let saved_array_elem_types = self.array_elem_types.clone();
        let saved_enum_regs = self.enum_regs.clone();
        let saved_enum_vars = self.enum_vars.clone();
        let saved_mir_temps = self.mir_temps.clone();
        self.string_regs.clear();
        self.string_vars.clear();
        self.bool_regs.clear();
        self.bool_vars.clear();
        self.var_types.clear();
        self.float_regs.clear();
        self.float_vars.clear();
        self.array_regs.clear();
        self.array_vars.clear();
        self.enum_regs.clear();
        self.enum_vars.clear();
        self.mir_temps.clear();
        self.unnamed_count = f.params.len() as u64 + 1;

        let mut fn_body = String::new();
        let mut fn_vars: HashMap<String, String> = HashMap::new();

        let params_ir: Vec<String> = f.params.iter().map(|_| "i64".to_string()).collect();
        let header = format!("define i64 @{}({}) {{\n", f.name, params_ir.join(", "));

        for (i, (pname, _)) in f.params.iter().enumerate() {
            let param_reg = format!("%{}", i);
            let reg = self.fresh();
            write!(fn_body, "  {} = alloca i64, align 8\n", reg).unwrap();
            write!(fn_body, "  store i64 {}, ptr {}\n", param_reg, reg).unwrap();
            fn_vars.insert(pname.clone(), reg);
        }

        for (lname, ltype) in &f.locals {
            if !fn_vars.contains_key(lname) {
                let reg = self.fresh();
                write!(fn_body, "  {} = alloca i64, align 8\n", reg).unwrap();
                fn_vars.insert(lname.clone(), reg);
            }
            // Track array element types for __index type propagation
            if let HirType::Array(inner) = ltype {
                self.array_elem_types.insert(lname.clone(), (**inner).clone());
            }
        }

        let entry_label = format!("L{}_{}", self.label_count, f.name);
        self.label_count += 1;
        write!(fn_body, "  br label %{}\n", entry_label).unwrap();
        write!(fn_body, "{}:\n", entry_label).unwrap();

        let saved_body = std::mem::replace(&mut self.body, fn_body);
        let saved_vars = std::mem::replace(&mut self.variables, fn_vars);
        self.block_terminated = false;

        for block in &f.blocks {
            let label = format!("mir_b{}", block.id);
            if !self.block_terminated {
                write!(self.body, "  br label %{}\n", label).unwrap();
            }
            write!(self.body, "{}:\n", label).unwrap();
            self.block_terminated = false;

            for stmt in &block.statements {
                self.emit_mir_stmt(stmt)?;
            }

            self.emit_mir_terminator(&block.terminator)?;
        }

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
        self.bool_vars = saved_bool_vars;
        self.float_regs = saved_float_regs;
        self.float_vars = saved_float_vars;
        self.array_regs = saved_array_regs;
        self.array_vars = saved_array_vars;
        self.array_elem_types = saved_array_elem_types;
        self.enum_regs = saved_enum_regs;
        self.enum_vars = saved_enum_vars;
        self.mir_temps = saved_mir_temps;
        Ok(())
    }

    pub(super) fn emit_mir_struct_ctor(&mut self, full_name: &str, fields: &[(String, TypeAnnot)]) -> Result<(), String> {
        let saved_unnamed = self.unnamed_count;
        self.unnamed_count = fields.len() as u64 + 1;
        let mut body = String::new();
        let params: Vec<String> = (0..fields.len()).map(|i| format!("i64 %{}", i)).collect();
        let header = format!("define i64 @{}({}) {{\n", full_name, params.join(", "));
        let size = (fields.len().max(1) * 8) as i64;
        let s1 = self.fresh();
        write!(body, "  {} = add i64 0, {}\n", s1, size).unwrap();
        self.declare_extern("malloc");
        let s2 = self.fresh();
        write!(body, "  {} = call ptr @malloc(i64 {})\n", s2, s1).unwrap();
        for i in 0..fields.len() {
            let elem = self.fresh();
            write!(body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem, s2, i).unwrap();
            write!(body, "  store i64 %{}, ptr {}\n", i, elem).unwrap();
        }
        let res = self.fresh();
        write!(body, "  {} = ptrtoint ptr {} to i64\n", res, s2).unwrap();
        write!(body, "  ret i64 {}\n", res).unwrap();
        body.push_str("}\n");
        self.functions.push_str(&header);
        self.functions.push_str(&body);
        self.unnamed_count = saved_unnamed;
        Ok(())
    }

    pub(super) fn emit_mir_struct_getter(&mut self, struct_name: &str, field_name: &str, fields: &[(String, TypeAnnot)]) -> Result<(), String> {
        let saved_unnamed = self.unnamed_count;
        self.unnamed_count = 2;
        let offset = fields.iter().position(|(n, _)| n == field_name).unwrap_or(0);
        let full_name = format!("__struct_get_{}_{}", struct_name, field_name);
        let header = format!("define i64 @{}(i64 %0) {{\n", full_name);
        let mut body = String::new();
        let r1 = self.fresh();
        write!(body, "  {} = inttoptr i64 %0 to ptr\n", r1).unwrap();
        let r2 = self.fresh();
        write!(body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", r2, r1, offset).unwrap();
        let r3 = self.fresh();
        write!(body, "  {} = load i64, ptr {}\n", r3, r2).unwrap();
        write!(body, "  ret i64 {}\n", r3).unwrap();
        body.push_str("}\n");
        self.functions.push_str(&header);
        self.functions.push_str(&body);
        self.unnamed_count = saved_unnamed;
        Ok(())
    }

    pub(super) fn emit_mir_struct_setter(&mut self, struct_name: &str, field_name: &str, fields: &[(String, TypeAnnot)]) -> Result<(), String> {
        let saved_unnamed = self.unnamed_count;
        self.unnamed_count = 3;
        let offset = fields.iter().position(|(n, _)| n == field_name).unwrap_or(0);
        let full_name = format!("__struct_set_{}_{}", struct_name, field_name);
        let header = format!("define i64 @{}(i64 %0, i64 %1) {{\n", full_name);
        let mut body = String::new();
        let r1 = self.fresh();
        write!(body, "  {} = inttoptr i64 %0 to ptr\n", r1).unwrap();
        let r2 = self.fresh();
        write!(body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", r2, r1, offset).unwrap();
        write!(body, "  store i64 %1, ptr {}\n", r2).unwrap();
        write!(body, "  ret i64 %0\n").unwrap();
        body.push_str("}\n");
        self.functions.push_str(&header);
        self.functions.push_str(&body);
        self.unnamed_count = saved_unnamed;
        Ok(())
    }

    pub(super) fn emit_mir_stmt(&mut self, stmt: &MirStmt) -> Result<(), String> {
        match stmt {
            MirStmt::Assign { dest, value } => {
                let val = self.emit_mir_rvalue(value)?;
                let is_string = self.string_regs.contains(&val);
                let is_bool = self.bool_regs.contains(&val);
                let is_float = self.float_regs.contains(&val);
                if let Some(var_reg) = self.variables.get(dest).cloned() {
                    write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                    if is_float { self.float_vars.insert(dest.clone()); }
                } else if let Some(gname) = self.globals_map.get(dest).cloned() {
                    write!(self.body, "  store i64 {}, ptr @{}\n", val, gname).unwrap();
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                    if is_float { self.float_vars.insert(dest.clone()); }
                } else {
                    self.mir_temps.insert(dest.clone(), val.clone());
                    if is_string { self.string_vars.insert(dest.clone()); }
                    if is_bool { self.bool_vars.insert(dest.clone()); }
                    if is_float { self.float_vars.insert(dest.clone()); }
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

                match func.as_str() {
                    "println" | "print" => {
                        self.emit_mir_print(compiled_args, func == "println")?;
                    }
                    "assert_eq" => {
                        let left = compiled_args.get(0).ok_or("assert_eq expects 2 arguments")?;
                        let right = compiled_args.get(1).ok_or("assert_eq expects 2 arguments")?;
                        let is_str = self.string_regs.contains(left) || self.string_regs.contains(right);
                        let cmp_reg = if is_str {
                            self.declare_extern("strcmp_ajeeb");
                            let cmp_val = self.fresh();
                            write!(self.body, "  {} = call i64 @strcmp_ajeeb(i64 {}, i64 {})\n", cmp_val, left, right).unwrap();
                            let cr = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, 0\n", cr, cmp_val).unwrap();
                            cr
                        } else {
                            let cr = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, {}\n", cr, left, right).unwrap();
                            cr
                        };
                        let cont_label = self.fresh_label();
                        let fail_label = self.fresh_label();
                        write!(self.body, "  br i1 {}, label %{}, label %{}\n", cmp_reg, cont_label, fail_label).unwrap();
                        write!(self.body, "{}:\n", fail_label).unwrap();
                        self.declare_extern("exit");
                        write!(self.body, "  call void @exit(i32 1)\n").unwrap();
                        write!(self.body, "  unreachable\n").unwrap();
                        write!(self.body, "{}:\n", cont_label).unwrap();
                        if let Some(ref dest_name) = dest {
                            let reg = self.fresh();
                            write!(self.body, "  {} = add i64 0, 0\n", reg).unwrap();
                            if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                write!(self.body, "  store i64 {}, ptr {}\n", reg, var_reg).unwrap();
                            } else {
                                self.mir_temps.insert(dest_name.clone(), reg);
                            }
                        }
                    }
                    _ => {
                        let call_func = if func == "len" && compiled_args.len() == 1
                            && self.array_regs.contains(&compiled_args[0]) {
                            "arr_len".to_string()
                        } else {
                            func.clone()
                        };
                        // __index: array/string indexing with non-constant index
                        if call_func == "__index" && compiled_args.len() == 2 {
                            let obj = &compiled_args[0];
                            let idx = &compiled_args[1];
                            if self.string_regs.contains(obj) {
                                // String indexing → charCode
                                self.declare_extern("charCode");
                                let reg = self.fresh();
                                write!(self.body, "  {} = call i64 @charCode(i64 {}, i64 {})\n", reg, obj, idx).unwrap();
                                if let Some(ref dest_name) = dest {
                                    if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                        write!(self.body, "  store i64 {}, ptr {}\n", reg, var_reg).unwrap();
                                    } else {
                                        self.mir_temps.insert(dest_name.clone(), reg);
                                    }
                                }
                            } else {
                                // Array indexing → GEP + load (offset = idx + 1 for length prefix)
                                let ptr = self.fresh();
                                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", ptr, obj).unwrap();
                                let one = self.fresh();
                                write!(self.body, "  {} = add i64 1, {}\n", one, idx).unwrap();
                                let elem_ptr = self.fresh();
                                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, ptr, one).unwrap();
                                let val = self.fresh();
                                write!(self.body, "  {} = load i64, ptr {}\n", val, elem_ptr).unwrap();
                                // Propagate element type from the array to the loaded value
                                let mut elem_is_string = false;
                                // Check if the obj register has element type info
                                if let Some(HirType::Str) = self.array_elem_types.get(obj) {
                                    elem_is_string = true;
                                }
                                // Also check by matching variable name
                                if !elem_is_string {
                                    for (vname, vreg) in &self.variables {
                                        if vreg == obj {
                                            if let Some(HirType::Str) = self.array_elem_types.get(vname) {
                                                elem_is_string = true;
                                            }
                                            break;
                                        }
                                    }
                                }
                                if elem_is_string {
                                    self.string_regs.insert(val.clone());
                                }
                                // Store element type info for the loaded value
                                if let Some(elem_ty) = self.array_elem_types.get(obj).cloned() {
                                    self.array_elem_types.insert(val.clone(), elem_ty);
                                }
                                if let Some(ref dest_name) = dest {
                                    if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                        write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                                    } else {
                                        self.mir_temps.insert(dest_name.clone(), val);
                                    }
                                }
                            }
                            return Ok(());
                        }
                        // __index_assign: array/string index assignment
                        if call_func == "__index_assign" && compiled_args.len() == 3 {
                            let obj = &compiled_args[0];
                            let idx = &compiled_args[1];
                            let val = &compiled_args[2];
                            if self.string_regs.contains(obj) {
                                // String index assign → strSet
                                self.declare_extern("strSet");
                                write!(self.body, "  call void @strSet(i64 {}, i64 {}, i64 {})\n", obj, idx, val).unwrap();
                            } else {
                                // Array index assign → GEP + store (offset = idx + 1 for length prefix)
                                let ptr = self.fresh();
                                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", ptr, obj).unwrap();
                                let one = self.fresh();
                                write!(self.body, "  {} = add i64 1, {}\n", one, idx).unwrap();
                                let elem_ptr = self.fresh();
                                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, ptr, one).unwrap();
                                write!(self.body, "  store i64 {}, ptr {}\n", val, elem_ptr).unwrap();
                            }
                            if let Some(ref dest_name) = dest {
                                if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                    write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                                } else {
                                    self.mir_temps.insert(dest_name.clone(), val.clone());
                                }
                            }
                            return Ok(());
                        }
                        if !self.declare_extern(&call_func) && !self.user_fns.contains(call_func.as_str()) {
                            let func_name = call_func.clone();
                            if let Some(field_name) = func_name.strip_prefix("__struct_get_") {
                                if let Some(underscore) = field_name.rfind('_') {
                                    let sname = &field_name[..underscore];
                                    let fname = &field_name[underscore + 1..];
                                    let fields_opt = self.struct_defs.get(sname).cloned();
                                    if let Some(fields) = fields_opt {
                                        self.emit_mir_struct_getter(sname, fname, &fields)?;
                                        self.user_fns.insert(func_name);
                                    } else {
                                        return Err(format!("MIR codegen: unknown struct '{}' for getter", sname));
                                    }
                                } else {
                                    return Err(format!("MIR codegen: unknown function '{}'", func));
                                }
                            } else if let Some(field_name) = func_name.strip_prefix("__struct_set_") {
                                if let Some(underscore) = field_name.rfind('_') {
                                    let sname = &field_name[..underscore];
                                    let fname = &field_name[underscore + 1..];
                                    let fields_opt = self.struct_defs.get(sname).cloned();
                                    if let Some(fields) = fields_opt {
                                        self.emit_mir_struct_setter(sname, fname, &fields)?;
                                        self.user_fns.insert(func_name);
                                    } else {
                                        return Err(format!("MIR codegen: unknown struct '{}' for setter", sname));
                                    }
                                } else {
                                    return Err(format!("MIR codegen: unknown function '{}'", func));
                                }
                            } else if func_name.starts_with("__struct_") {
                                if let Some(struct_name) = func_name.strip_prefix("__struct_") {
                                    let fields_opt = self.struct_defs.get(struct_name).cloned();
                                    if let Some(fields) = fields_opt {
                                        self.emit_mir_struct_ctor(&func_name, &fields)?;
                                        self.user_fns.insert(func_name);
                                    } else {
                                        return Err(format!("MIR codegen: unknown struct '{}' for constructor", struct_name));
                                    }
                                } else {
                                    return Err(format!("MIR codegen: unknown function '{}'", func));
                                }
                             } else {
                                 return Err(format!("MIR codegen: unknown function '{}'", func));
                              }
                           }
                            if let Some(dest_name) = dest {
                              let final_args_str = if call_func == "itoa" && compiled_args.len() == 1 {
                                 let a = &compiled_args[0];
                                 if self.float_regs.contains(a) {
                                     let f_bits = self.fresh();
                                     write!(self.body, "  {} = bitcast i64 {} to double\n", f_bits, a).unwrap();
                                     let int_val = self.fresh();
                                     write!(self.body, "  {} = fptosi double {} to i64\n", int_val, f_bits).unwrap();
                                     format!("i64 {}", int_val)
                                 } else {
                                     args_str.clone()
                                 }
                             } else {
                                 args_str.clone()
                             };
                               let reg = self.fresh();
                               write!(self.body, "  {} = call i64 @{}({})\n", reg, call_func, final_args_str).unwrap();
                               let is_string_ret = matches!(call_func.as_str(),
                                   "str_concat" | "itoa" | "substring" | "toUpperCase" | "toLowerCase"
                                   | "trim" | "readFile" | "readArg" | "replace"
                               ) || self.fn_return_types.get(call_func.as_str())
                                  .map(|rt| matches!(rt, TypeAnnot::String))
                                  .unwrap_or(false);
                                if is_string_ret {
                                    self.string_regs.insert(reg.clone());
                                    self.string_vars.insert(dest_name.clone());
                                }
                               if let Some(var_reg) = self.variables.get(dest_name).cloned() {
                                  write!(self.body, "  store i64 {}, ptr {}\n", reg, var_reg).unwrap();
                              } else {
                                  self.mir_temps.insert(dest_name.clone(), reg);
                              }
                          } else {
                              write!(self.body, "  call i64 @{}({})\n", call_func, args_str).unwrap();
                          }
                     }
                }
                Ok(())
            }
        }
    }

    pub(super) fn emit_mir_rvalue(&mut self, rvalue: &MirRvalue) -> Result<String, String> {
        match rvalue {
            MirRvalue::Use(operand) => self.emit_mir_operand(operand),
            MirRvalue::Const(c) => self.emit_mir_const(c),
            MirRvalue::BinaryOp(op, left, right) => {
                let l = self.emit_mir_operand(left)?;
                let r = self.emit_mir_operand(right)?;
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
                let is_float_op = self.float_regs.contains(&l) || self.float_regs.contains(&r)
                    || self.float_vars.iter().any(|v| {
                        self.mir_temps.get(v).map_or(false, |sr| sr == &l || sr == &r)
                    });
                if is_float_op {
                    let bits_l = self.fresh();
                    write!(self.body, "  {} = bitcast i64 {} to double\n", bits_l, l).unwrap();
                    let bits_r = self.fresh();
                    write!(self.body, "  {} = bitcast i64 {} to double\n", bits_r, r).unwrap();
                    let freg = self.fresh();
                    let (fop, use_fcmp) = match op {
                        MirBinOp::Add => ("fadd", false),
                        MirBinOp::Sub => ("fsub", false),
                        MirBinOp::Mul => ("fmul", false),
                        MirBinOp::Div => ("fdiv", false),
                        MirBinOp::Eq => ("fcmp oeq", true),
                        MirBinOp::Neq => ("fcmp une", true),
                        MirBinOp::Lt => ("fcmp olt", true),
                        MirBinOp::Gt => ("fcmp ogt", true),
                        MirBinOp::Le => ("fcmp ole", true),
                        MirBinOp::Ge => ("fcmp oge", true),
                        _ => ("fadd", false),
                    };
                    if use_fcmp {
                        let cmp_reg = self.fresh();
                        write!(self.body, "  {} = {} double {}, {}\n", cmp_reg, fop, bits_l, bits_r).unwrap();
                        let zext_reg = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext_reg, cmp_reg).unwrap();
                        self.bool_regs.insert(zext_reg.clone());
                        return Ok(zext_reg);
                    } else {
                        write!(self.body, "  {} = {} double {}, {}\n", freg, fop, bits_l, bits_r).unwrap();
                        let result = self.fresh();
                        write!(self.body, "  {} = bitcast double {} to i64\n", result, freg).unwrap();
                        self.float_regs.insert(result.clone());
                        return Ok(result);
                    }
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
                        let is_str = self.string_regs.contains(&l) || self.string_regs.contains(&r);
                        if is_str {
                            self.declare_extern("strcmp_ajeeb");
                            let cmp_val = self.fresh();
                            write!(self.body, "  {} = call i64 @strcmp_ajeeb(i64 {}, i64 {})\n", cmp_val, l, r).unwrap();
                            let cmp = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, 0\n", cmp, cmp_val).unwrap();
                            let zext = self.fresh();
                            write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                            self.bool_regs.insert(zext.clone());
                            return Ok(zext);
                        }
                        let cmp = self.fresh();
                        write!(self.body, "  {} = icmp eq i64 {}, {}\n", cmp, l, r).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, cmp).unwrap();
                        self.bool_regs.insert(zext.clone());
                        return Ok(zext);
                    }
                    MirBinOp::Neq => {
                        let is_str = self.string_regs.contains(&l) || self.string_regs.contains(&r);
                        let cmp = if is_str {
                            self.declare_extern("strcmp_ajeeb");
                            let cmp_val = self.fresh();
                            write!(self.body, "  {} = call i64 @strcmp_ajeeb(i64 {}, i64 {})\n", cmp_val, l, r).unwrap();
                            let is_zero = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, 0\n", is_zero, cmp_val).unwrap();
                            let cr = self.fresh();
                            write!(self.body, "  {} = xor i1 {}, 1\n", cr, is_zero).unwrap();
                            cr
                        } else {
                            let cr = self.fresh();
                            write!(self.body, "  {} = icmp ne i64 {}, {}\n", cr, l, r).unwrap();
                            cr
                        };
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

    pub(super) fn emit_mir_operand(&mut self, operand: &MirOperand) -> Result<String, String> {
        match operand {
            MirOperand::Var(name) => {
                if let Some(var_reg) = self.variables.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr {}\n", reg, var_reg).unwrap();
                    if self.string_vars.contains(name) { self.string_regs.insert(reg.clone()); }
                    if self.bool_vars.contains(name) { self.bool_regs.insert(reg.clone()); }
                    if self.float_vars.contains(name) { self.float_regs.insert(reg.clone()); }
                    // Propagate array element type to the loaded register by SSA name
                    if let Some(elem_ty) = self.array_elem_types.get(name).cloned() {
                        self.array_elem_types.insert(reg.clone(), elem_ty);
                    }
                    Ok(reg)
                } else if let Some(ssa_reg) = self.mir_temps.get(name).cloned() {
                    if self.bool_vars.contains(name) { self.bool_regs.insert(ssa_reg.clone()); }
                    if self.string_vars.contains(name) { self.string_regs.insert(ssa_reg.clone()); }
                    if self.float_vars.contains(name) { self.float_regs.insert(ssa_reg.clone()); }
                    Ok(ssa_reg)
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr @{}\n", reg, gname).unwrap();
                    if self.string_vars.contains(name) { self.string_regs.insert(reg.clone()); }
                    if self.bool_vars.contains(name) { self.bool_regs.insert(reg.clone()); }
                    Ok(reg)
                } else {
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

    pub(super) fn emit_mir_const(&mut self, c: &MirConst) -> Result<String, String> {
        match c {
            MirConst::Int(n) => {
                let reg = self.fresh();
                write!(self.body, "  {} = add i64 0, {}\n", reg, n).unwrap();
                Ok(reg)
            }
            MirConst::Float(f) => {
                let reg = self.fresh();
                let bits = f.to_bits() as i64;
                write!(self.body, "  {} = add i64 0, {}\n", reg, bits).unwrap();
                self.float_regs.insert(reg.clone());
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

    pub(super) fn emit_mir_terminator(&mut self, term: &Terminator) -> Result<(), String> {
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
                    // targets[0] is the "true" branch, default is the "false" branch
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

    pub(super) fn emit_mir_print(&mut self, args: Vec<String>, is_println: bool) -> Result<(), String> {
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
            if self.bool_regs.contains(arg) {
                let is_zero = self.fresh();
                write!(self.body, "  {} = icmp eq i64 {}, 0\n", is_zero, arg).unwrap();
                let true_ptr = self.fresh();
                let true_str = self.global_str("true");
                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", true_ptr, true_str).unwrap();
                let false_ptr = self.fresh();
                let false_str = self.global_str("false");
                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", false_ptr, false_str).unwrap();
                let chosen_ptr = self.fresh();
                write!(self.body, "  {} = select i1 {}, ptr {}, ptr {}\n", chosen_ptr, is_zero, false_ptr, true_ptr).unwrap();
                let reg = self.fresh();
                if is_println {
                    write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, chosen_ptr).unwrap();
                } else {
                    write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, chosen_ptr).unwrap();
                }
            } else if self.string_regs.contains(arg) {
                let str_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", str_ptr, arg).unwrap();
                let reg = self.fresh();
                if is_println {
                    write!(self.body, "  {} = call i32 @puts(ptr {})\n", reg, str_ptr).unwrap();
                } else {
                    write!(self.body, "  {} = call i32 (ptr, ...) @printf(ptr {})\n", reg, str_ptr).unwrap();
                }
            } else {
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

pub(super) fn hir_type_to_type_ann(t: &HirType) -> TypeAnnot {
    match t {
        HirType::Int => TypeAnnot::Int,
        HirType::Float => TypeAnnot::Float,
        HirType::Bool => TypeAnnot::Bool,
        HirType::Str => TypeAnnot::String,
        HirType::Void => TypeAnnot::Void,
        HirType::Named(s) => TypeAnnot::Class(s.clone()),
        HirType::Array(inner) => TypeAnnot::Array(Box::new(hir_type_to_type_ann(inner))),
        HirType::Generic(name, _args) => TypeAnnot::Generic(name.clone()),
        HirType::Unknown => TypeAnnot::Void,
    }
}
