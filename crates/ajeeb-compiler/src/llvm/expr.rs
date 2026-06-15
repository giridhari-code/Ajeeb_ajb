use std::fmt::Write;
use crate::ast::{BinOp, Expr, Pattern, TypeAnnot};
use super::Codegen;

impl Codegen {
    pub(super) fn emit_expr(&mut self, expr: &Expr) -> Result<String, String> {
        match expr {
            Expr::Number(n, ..) => {
                let reg = self.fresh();
                write!(self.body, "  {} = add i64 0, {}\n", reg, n).unwrap();
                Ok(reg)
            }
            Expr::FloatLit(f, ..) => {
                let reg = self.fresh();
                let bits = f.to_bits();
                write!(self.body, "  {} = bitcast i64 {} to double\n", reg, bits).unwrap();
                Ok(reg)
            }
            Expr::Bool(b, ..) => {
                let reg = self.fresh();
                write!(self.body, "  {} = add i64 0, {}\n", reg, if *b { 1 } else { 0 }).unwrap();
                Ok(reg)
            }
            Expr::StringLit(s, ..) => {
                let gname = self.global_str(s);
                let ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", ptr, gname).unwrap();
                let reg = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", reg, ptr).unwrap();
                self.string_regs.insert(reg.clone());
                Ok(reg)
            }
            Expr::Ident(name, ..) => {
                if let Some(var_reg) = self.variables.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr {}\n", reg, var_reg).unwrap();
                    if self.string_vars.contains(name) {
                        self.string_regs.insert(reg.clone());
                    }
                    Ok(reg)
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    let reg = self.fresh();
                    write!(self.body, "  {} = load i64, ptr @{}\n", reg, gname).unwrap();
                    if self.string_vars.contains(name) {
                        self.string_regs.insert(reg.clone());
                    }
                    Ok(reg)
                } else if name == "__str_ptr" {
                    let reg = self.fresh();
                    write!(self.body, "  {} = add i64 0, 0\n", reg).unwrap();
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
                        let is_str = matches!(left.as_ref(), Expr::StringLit(..))
                            || matches!(right.as_ref(), Expr::StringLit(..))
                            || self.string_regs.contains(&lhs)
                            || self.string_regs.contains(&rhs);
                        if is_str {
                            self.declare_extern("str_concat");
                            let reg2 = self.fresh();
                            write!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})\n", reg2, lhs, rhs).unwrap();
                            self.string_regs.insert(reg2.clone());
                            return Ok(reg2);
                        }
                        write!(self.body, "  {} = add i64 {}, {}\n", reg, lhs, rhs).unwrap();
                    }
                    BinOp::Sub => write!(self.body, "  {} = sub i64 {}, {}\n", reg, lhs, rhs).unwrap(),
                    BinOp::Mul => write!(self.body, "  {} = mul i64 {}, {}\n", reg, lhs, rhs).unwrap(),
                    BinOp::Div => {
                        let is_zero = self.fresh();
                        write!(self.body, "  {} = icmp eq i64 {}, 0\n", is_zero, rhs).unwrap();
                        let safe_rhs = self.fresh();
                        write!(self.body, "  {} = select i1 {}, i64 1, i64 {}\n", safe_rhs, is_zero, rhs).unwrap();
                        let div_raw = self.fresh();
                        write!(self.body, "  {} = sdiv i64 {}, {}\n", div_raw, lhs, safe_rhs).unwrap();
                        let final_reg = self.fresh();
                        write!(self.body, "  {} = select i1 {}, i64 0, i64 {}\n", final_reg, is_zero, div_raw).unwrap();
                        return Ok(final_reg);
                    }
                    BinOp::Eq => {
                        write!(self.body, "  {} = icmp eq i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Neq => {
                        write!(self.body, "  {} = icmp ne i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Lt => {
                        write!(self.body, "  {} = icmp slt i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Gt => {
                        write!(self.body, "  {} = icmp sgt i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Le => {
                        write!(self.body, "  {} = icmp sle i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::Ge => {
                        write!(self.body, "  {} = icmp sge i64 {}, {}\n", reg, lhs, rhs).unwrap();
                        let zext = self.fresh();
                        write!(self.body, "  {} = zext i1 {} to i64\n", zext, reg).unwrap();
                        return Ok(zext);
                    }
                    BinOp::And => write!(self.body, "  {} = and i64 {}, {}\n", reg, lhs, rhs).unwrap(),
                    BinOp::Or => write!(self.body, "  {} = or i64 {}, {}\n", reg, lhs, rhs).unwrap(),
                }
                Ok(reg)
            }
            Expr::Assign { name, value, .. } => {
                let val = self.emit_expr(value)?;
                if let Some(var_reg) = self.variables.get(name) {
                    write!(self.body, "  store i64 {}, ptr {}\n", val, var_reg).unwrap();
                } else if let Some(gname) = self.globals_map.get(name).cloned() {
                    write!(self.body, "  store i64 {}, ptr @{}\n", val, gname).unwrap();
                } else {
                    return Err(format!("Undefined variable: {}", name));
                }
                Ok(val)
            }
            Expr::UnaryNot(val, ..) => {
                let v = self.emit_expr(val)?;
                let cmp = self.fresh();
                write!(self.body, "  {} = icmp eq i64 {}, 0\n", cmp, v).unwrap();
                let reg = self.fresh();
                write!(self.body, "  {} = zext i1 {} to i64\n", reg, cmp).unwrap();
                Ok(reg)
            }
            Expr::UnaryMinus(val, ..) => {
                let v = self.emit_expr(val)?;
                let reg = self.fresh();
                write!(self.body, "  {} = sub i64 0, {}\n", reg, v).unwrap();
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
                        let mut string_args = Vec::new();
                        for (i, arg_reg) in compiled_args.iter().enumerate() {
                            if self.string_regs.contains(arg_reg) {
                                string_args.push(arg_reg.clone());
                            } else {
                                let buf = self.fresh();
                                write!(self.body, "  {} = alloca i8, i64 32\n", buf).unwrap();
                                let fmt_name = self.global_str("%ld");
                                let fmt_ptr = self.fresh();
                                write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", fmt_ptr, fmt_name).unwrap();
                                let r = self.fresh();
                                write!(self.body, "  {} = call i32 (ptr, i64, ptr, ...) @snprintf(ptr {}, i64 32, ptr {}, i64 {})\n", r, buf, fmt_ptr, arg_reg).unwrap();
                                let ptr_as_int = self.fresh();
                                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_int, buf).unwrap();
                                self.string_regs.insert(ptr_as_int.clone());
                                string_args.push(ptr_as_int);
                            }
                        }
                        if string_args.is_empty() {
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
                        } else {
                            self.declare_extern("str_concat");
                            let mut concat = string_args[0].clone();
                            for arg in &string_args[1..] {
                                let next_concat = self.fresh();
                                write!(self.body, "  {} = call i64 @str_concat(i64 {}, i64 {})\n", next_concat, concat, arg).unwrap();
                                concat = next_concat;
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
                        let reg = self.fresh();
                        write!(self.body, "  {} = add i64 0, 0\n", reg).unwrap();
                        Ok(reg)
                    }
                    "itoa" => {
                        let val = compiled_args.first().ok_or("itoa expects 1 argument")?;
                        let buf = self.fresh();
                        write!(self.body, "  {} = alloca i8, i64 32\n", buf).unwrap();
                        let fmt_name = self.global_str("%ld");
                        let fmt_ptr = self.fresh();
                        write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", fmt_ptr, fmt_name).unwrap();
                        let reg = self.fresh();
                        write!(self.body, "  {} = call i32 (ptr, i64, ptr, ...) @snprintf(ptr {}, i64 32, ptr {}, i64 {})\n", reg, buf, fmt_ptr, val).unwrap();
                        let ptr_as_int = self.fresh();
                        write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_int, buf).unwrap();
                        self.string_regs.insert(ptr_as_int.clone());
                        Ok(ptr_as_int)
                    }
                    _ => {
                        if !self.declare_extern(name) && !self.user_fns.contains(name.as_str()) {
                            return Err(format!("LLVM codegen not supported for interpreter builtin: {}", name));
                        }
                        let args_str = compiled_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                        if matches!(name.as_str(), "setInt" | "strSet" | "writeFile" | "writeAppend" | "writeByte" | "tcp_write" | "tcp_close" | "tls_write" | "tls_close") {
                            write!(self.body, "  call void @{}({})\n", name, args_str).unwrap();
                            let reg = self.fresh();
                            write!(self.body, "  {} = add i64 0, 0\n", reg).unwrap();
                            Ok(reg)
                        } else {
                            let reg = self.fresh();
                            write!(self.body, "  {} = call i64 @{}({})\n", reg, name, args_str).unwrap();
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
                    write!(self.body, "  {} = add i64 0, 0\n", reg).unwrap();
                    return Ok(reg);
                }
                let count = items.len() as u64;
                let arr_ptr = self.fresh();
                write!(self.body, "  {} = alloca i64, i64 {}\n", arr_ptr, count).unwrap();
                for (i, item) in items.iter().enumerate() {
                    let val = self.emit_expr(item)?;
                    let elem_ptr = self.fresh();
                    write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, arr_ptr, i).unwrap();
                    write!(self.body, "  store i64 {}, ptr {}\n", val, elem_ptr).unwrap();
                }
                let ptr_as_i64 = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_i64, arr_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Index { obj, index, .. } => {
                let arr_val = self.emit_expr(obj)?;
                let idx = self.emit_expr(index)?;
                let arr_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", arr_ptr, arr_val).unwrap();
                let elem_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, arr_ptr, idx).unwrap();
                let reg = self.fresh();
                write!(self.body, "  {} = load i64, ptr {}\n", reg, elem_ptr).unwrap();
                Ok(reg)
            }
            Expr::IndexAssign { obj, index, value, .. } => {
                let arr_val = self.emit_expr(obj)?;
                let idx = self.emit_expr(index)?;
                let val = self.emit_expr(value)?;
                let arr_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", arr_ptr, arr_val).unwrap();
                let elem_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, arr_ptr, idx).unwrap();
                write!(self.body, "  store i64 {}, ptr {}\n", val, elem_ptr).unwrap();
                Ok(val)
            }
            Expr::StructLit { struct_name, fields, .. } => {
                let field_count = fields.len() as u64;
                let struct_ptr = self.fresh();
                write!(self.body, "  {} = alloca i64, i64 {}\n", struct_ptr, field_count.max(1)).unwrap();
                for (i, (fname, fexpr)) in fields.iter().enumerate() {
                    let fval = self.emit_expr(fexpr)?;
                    let elem_ptr = self.fresh();
                    write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, struct_ptr, i).unwrap();
                    write!(self.body, "  store i64 {}, ptr {}\n", fval, elem_ptr).unwrap();
                }
                let ptr_as_i64 = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_i64, struct_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Field { obj, field, .. } => {
                let obj_val = self.emit_expr(obj)?;
                let obj_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", obj_ptr, obj_val).unwrap();
                let offset = self.resolve_field_offset(obj, field).unwrap_or(0);
                let elem_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, obj_ptr, offset).unwrap();
                let reg = self.fresh();
                write!(self.body, "  {} = load i64, ptr {}\n", reg, elem_ptr).unwrap();
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
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", obj_ptr, obj_val).unwrap();
                let offset = self.resolve_field_offset(obj, field).unwrap_or(0);
                let elem_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, obj_ptr, offset).unwrap();
                write!(self.body, "  store i64 {}, ptr {}\n", val, elem_ptr).unwrap();
                if let Expr::Ident(var_name, ..) = obj.as_ref() {
                    if let Some(var_reg) = self.variables.get(var_name) {
                        write!(self.body, "  store i64 {}, ptr {}\n", obj_val, var_reg).unwrap();
                    }
                }
                Ok(val)
            }
            Expr::MethodCall { obj, method, args, .. } => {
                let obj_val = self.emit_expr(obj)?;
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
                    let mangled = self.resolve_method(&rt, method);
                    if let Some(mangled_name) = mangled {
                        let mut call_args = vec![obj_val];
                        for a in args {
                            call_args.push(self.emit_expr(a)?);
                        }
                        let args_str = call_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                        let reg = self.fresh();
                        write!(self.body, "  {} = call i64 @{}({})\n", reg, mangled_name, args_str).unwrap();
                        return Ok(reg);
                    }
                }
                Err(format!("LLVM codegen: cannot resolve method {} on receiver", method))
            }
            Expr::EnumCtor { enum_name, variant, args, .. } => {
                let payload_count = args.len() as u64;
                let enum_ptr = self.fresh();
                write!(self.body, "  {} = alloca i64, i64 {}\n", enum_ptr, (payload_count + 1).max(2)).unwrap();
                let tag_id = self.enum_variant_ids.get(&(enum_name.clone(), variant.clone())).copied().unwrap_or(0);
                write!(self.body, "  store i64 {}, ptr {}\n", tag_id, enum_ptr).unwrap();
                for (i, a) in args.iter().enumerate() {
                    let aval = self.emit_expr(a)?;
                    let elem_ptr = self.fresh();
                    write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", elem_ptr, enum_ptr, i + 1).unwrap();
                    write!(self.body, "  store i64 {}, ptr {}\n", aval, elem_ptr).unwrap();
                }
                let ptr_as_i64 = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_i64, enum_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::EnumRef { enum_name, variant, .. } => {
                let enum_ptr = self.fresh();
                write!(self.body, "  {} = alloca i64, i64 2\n", enum_ptr).unwrap();
                let tag_id = self.enum_variant_ids.get(&(enum_name.clone(), variant.clone())).copied().unwrap_or(0);
                write!(self.body, "  store i64 {}, ptr {}\n", tag_id, enum_ptr).unwrap();
                let zero = self.fresh();
                write!(self.body, "  {} = add i64 0, 0\n", zero).unwrap();
                let elem_ptr = self.fresh();
                write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 1\n", elem_ptr, enum_ptr).unwrap();
                write!(self.body, "  store i64 {}, ptr {}\n", zero, elem_ptr).unwrap();
                let ptr_as_i64 = self.fresh();
                write!(self.body, "  {} = ptrtoint ptr {} to i64\n", ptr_as_i64, enum_ptr).unwrap();
                Ok(ptr_as_i64)
            }
            Expr::Match { value, arms, .. } => {
                let scrutinee_val = self.emit_expr(value)?;
                let result_ptr = self.fresh();
                write!(self.body, "  {} = alloca i64, align 8\n", result_ptr).unwrap();
                let default = self.fresh();
                write!(self.body, "  {} = add i64 0, 0\n", default).unwrap();
                write!(self.body, "  store i64 {}, ptr {}\n", default, result_ptr).unwrap();
                let merge_label = self.fresh_label();

                let enum_ptr = self.fresh();
                write!(self.body, "  {} = inttoptr i64 {} to ptr\n", enum_ptr, scrutinee_val).unwrap();
                let tag_reg = self.fresh();
                write!(self.body, "  {} = load i64, ptr {}\n", tag_reg, enum_ptr).unwrap();

                let next_label = self.fresh_label();
                write!(self.body, "  br label %{}\n", next_label).unwrap();
                write!(self.body, "{}:\n", next_label).unwrap();

                for arm in arms {
                    let arm_label = self.fresh_label();
                    let fallthrough_label = self.fresh_label();

                    match &arm.pattern {
                        Pattern::Wildcard => {
                            write!(self.body, "  br label %{}\n", arm_label).unwrap();
                        }
                        Pattern::EnumVariant { enum_name, variant, bindings } => {
                            let expected_tag = self.enum_variant_ids
                                .get(&(enum_name.clone(), variant.clone()))
                                .copied().unwrap_or(-1);
                            let cmp = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, {}\n", cmp, tag_reg, expected_tag).unwrap();
                            write!(self.body, "  br i1 {}, label %{}, label %{}\n", cmp, arm_label, fallthrough_label).unwrap();
                        }
                        Pattern::Int(n) => {
                            let cmp = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, {}\n", cmp, scrutinee_val, n).unwrap();
                            write!(self.body, "  br i1 {}, label %{}, label %{}\n", cmp, arm_label, fallthrough_label).unwrap();
                        }
                        Pattern::String(s) => {
                            let sname = self.global_str(s);
                            let sptr = self.fresh();
                            write!(self.body, "  {} = getelementptr inbounds i8, ptr @{}, i64 0\n", sptr, sname).unwrap();
                            let s_as_i64 = self.fresh();
                            write!(self.body, "  {} = ptrtoint ptr {} to i64\n", s_as_i64, sptr).unwrap();
                            self.declare_extern("strcmp_ajeeb");
                            let cmp_result = self.fresh();
                            write!(self.body, "  {} = call i64 @strcmp_ajeeb(i64 {}, i64 {})\n", cmp_result, scrutinee_val, s_as_i64).unwrap();
                            let cmp_bool = self.fresh();
                            write!(self.body, "  {} = icmp eq i64 {}, 0\n", cmp_bool, cmp_result).unwrap();
                            write!(self.body, "  br i1 {}, label %{}, label %{}\n", cmp_bool, arm_label, fallthrough_label).unwrap();
                        }
                    }

                    write!(self.body, "{}:\n", arm_label).unwrap();
                    if let Pattern::EnumVariant { bindings, .. } = &arm.pattern {
                        for (i, bname) in bindings.iter().enumerate() {
                            let offset = i + 1;
                            let data_ptr = self.fresh();
                            write!(self.body, "  {} = getelementptr inbounds i64, ptr {}, i64 {}\n", data_ptr, enum_ptr, offset).unwrap();
                            let data_val = self.fresh();
                            write!(self.body, "  {} = load i64, ptr {}\n", data_val, data_ptr).unwrap();
                            let binding_alloca = self.fresh();
                            write!(self.body, "  {} = alloca i64, align 8\n", binding_alloca).unwrap();
                            write!(self.body, "  store i64 {}, ptr {}\n", data_val, binding_alloca).unwrap();
                            self.variables.insert(bname.clone(), binding_alloca);
                        }
                    }

                    if let Some(stmts) = &arm.body_block {
                        for s in stmts {
                            self.emit_stmt(s)?;
                        }
                    } else {
                        let arm_result = self.emit_expr(&arm.body)?;
                        write!(self.body, "  store i64 {}, ptr {}\n", arm_result, result_ptr).unwrap();
                    }

                    if let Pattern::EnumVariant { bindings, .. } = &arm.pattern {
                        for bname in bindings {
                            self.variables.remove(bname);
                        }
                    }

                    write!(self.body, "  br label %{}\n", merge_label).unwrap();

                    if !matches!(arm.pattern, Pattern::Wildcard) {
                        write!(self.body, "{}:\n", fallthrough_label).unwrap();
                    }
                }

                let has_wildcard = arms.iter().any(|a| matches!(a.pattern, Pattern::Wildcard));
                if !has_wildcard {
                    write!(self.body, "  br label %{}\n", merge_label).unwrap();
                }
                write!(self.body, "{}:\n", merge_label).unwrap();
                let result = self.fresh();
                write!(self.body, "  {} = load i64, ptr {}\n", result, result_ptr).unwrap();
                Ok(result)
            }
            Expr::GenericCall { name, args, .. } => {
                let mut compiled_args = Vec::new();
                for arg in args {
                    compiled_args.push(self.emit_expr(arg)?);
                }
                if !self.declare_extern(name) && !self.user_fns.contains(name.as_str()) {
                    return Err(format!("LLVM codegen: unknown function {}", name));
                }
                let args_str = compiled_args.iter().map(|a| format!("i64 {}", a)).collect::<Vec<_>>().join(", ");
                let reg = self.fresh();
                write!(self.body, "  {} = call i64 @{}({})\n", reg, name, args_str).unwrap();
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
                write!(self.body, "  {} = call i64 @{}({})\n", reg, mangled, args_str).unwrap();
                Ok(reg)
            }
            _ => Err(format!("Unsupported expression: {:?}", expr)),
        }
    }
}
