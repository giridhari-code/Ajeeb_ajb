use super::{Evaluator, RuntimeValue, is_truthy};
use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

impl Evaluator {
    pub(super) fn eval_expr(&mut self, expr: &Expr) -> RuntimeValue {
        match expr {
            Expr::Number(n, ..) => RuntimeValue::Int(*n),
            Expr::FloatLit(f, ..) => RuntimeValue::Float(*f),
            Expr::StringLit(s, ..) => RuntimeValue::String(Rc::new(RefCell::new(s.clone()))),
            Expr::Bool(b, ..) => RuntimeValue::Bool(*b),
            Expr::Ident(id, line, col) => self.lookup_var(id).unwrap_or_else(|| {
                self.print_stack_trace();
                eprintln!("[ERROR] Unknown variable '{}' at line {}, col {} — treating as 0", id, line, col);
                RuntimeValue::Int(0)
            }),
            Expr::Lambda { .. } => RuntimeValue::Int(0),
            Expr::ClosureCall { .. } => RuntimeValue::Int(0),
            Expr::Binary { left, op, right, .. } => {
                let l = self.eval_expr(left);
                let r = self.eval_expr(right);
                use BinOp::*;
                match (l, r) {
                    (RuntimeValue::Int(a), RuntimeValue::Int(b)) => RuntimeValue::Int(match op {
                        Add => a + b,
                        Sub => a - b,
                        Mul => a * b,
                        Div => {
                            if b == 0 {
                                self.print_stack_trace();
                                eprintln!("[ERROR] Division by zero — returning 0");
                                0
                            } else {
                                a / b
                            }
                        }
                        Eq => (a == b) as i64,
                        Neq => (a != b) as i64,
                        Lt => (a < b) as i64,
                        Gt => (a > b) as i64,
                        Le => (a <= b) as i64,
                        Ge => (a >= b) as i64,
                        And => (a != 0 && b != 0) as i64,
                        Or => (a != 0 || b != 0) as i64,
                    }),
                    (RuntimeValue::Float(a), RuntimeValue::Float(b)) => RuntimeValue::Float(match op {
                        Add => a + b,
                        Sub => a - b,
                        Mul => a * b,
                        Div => a / b,
                        Eq => (a == b) as i64 as f64,
                        Neq => (a != b) as i64 as f64,
                        Lt => (a < b) as i64 as f64,
                        Gt => (a > b) as i64 as f64,
                        Le => (a <= b) as i64 as f64,
                        Ge => (a >= b) as i64 as f64,
                        _ => 0.0,
                    }),
                    (RuntimeValue::Int(a), RuntimeValue::Float(b)) => RuntimeValue::Float(match op {
                        Add => a as f64 + b,
                        Sub => a as f64 - b,
                        Mul => a as f64 * b,
                        Div => a as f64 / b,
                        Eq => ((a as f64) == b) as i64 as f64,
                        Neq => ((a as f64) != b) as i64 as f64,
                        Lt => ((a as f64) < b) as i64 as f64,
                        Gt => ((a as f64) > b) as i64 as f64,
                        Le => ((a as f64) <= b) as i64 as f64,
                        Ge => ((a as f64) >= b) as i64 as f64,
                        _ => 0.0,
                    }),
                    (RuntimeValue::Float(a), RuntimeValue::Int(b)) => RuntimeValue::Float(match op {
                        Add => a + b as f64,
                        Sub => a - b as f64,
                        Mul => a * b as f64,
                        Div => a / b as f64,
                        Eq => (a == b as f64) as i64 as f64,
                        Neq => (a != b as f64) as i64 as f64,
                        Lt => (a < b as f64) as i64 as f64,
                        Gt => (a > b as f64) as i64 as f64,
                        Le => (a <= b as f64) as i64 as f64,
                        Ge => (a >= b as f64) as i64 as f64,
                        _ => 0.0,
                    }),
                    (RuntimeValue::String(a), RuntimeValue::String(b)) => match op {
                        Add => RuntimeValue::String(Rc::new(RefCell::new(
                            a.borrow().clone() + &b.borrow(),
                        ))),
                        Eq => {
                            let a_trim: String =
                                a.borrow().chars().take_while(|&c| c != '\0').collect();
                            let b_trim: String =
                                b.borrow().chars().take_while(|&c| c != '\0').collect();
                            RuntimeValue::Bool(a_trim == b_trim)
                        }
                        Neq => {
                            let a_trim: String =
                                a.borrow().chars().take_while(|&c| c != '\0').collect();
                            let b_trim: String =
                                b.borrow().chars().take_while(|&c| c != '\0').collect();
                            RuntimeValue::Bool(a_trim != b_trim)
                        }
                        _ => RuntimeValue::Int(0),
                    },
                    (RuntimeValue::Bool(a), RuntimeValue::Bool(b)) => match op {
                        Eq => RuntimeValue::Bool(a == b),
                        Neq => RuntimeValue::Bool(a != b),
                        And => RuntimeValue::Bool(a && b),
                        Or => RuntimeValue::Bool(a || b),
                        _ => RuntimeValue::Int(0),
                    },
                    (RuntimeValue::EnumVariant { enum_name, variant, data }, RuntimeValue::EnumVariant { enum_name: en2, variant: v2, data: d2 }) => match op {
                        Eq => RuntimeValue::Bool(enum_name == en2 && variant == v2 && super::runtime_values_eq(&data, &d2)),
                        Neq => RuntimeValue::Bool(enum_name != en2 || variant != v2 || !super::runtime_values_eq(&data, &d2)),
                        _ => RuntimeValue::Int(0),
                    },
                    (RuntimeValue::Array(a), RuntimeValue::Array(b)) => match op {
                        Eq => RuntimeValue::Bool(super::runtime_values_eq(&a.borrow(), &b.borrow())),
                        Neq => RuntimeValue::Bool(!super::runtime_values_eq(&a.borrow(), &b.borrow())),
                        _ => RuntimeValue::Int(0),
                    },
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::Assign { name, value, .. } => {
                let val = self.eval_expr(value);
                self.insert_var(name.clone(), val.clone());
                val
            }
            Expr::FnCall { name, args, line, col } => self.exec_fn_call_at(name, args, *line, *col),
            Expr::AssociatedFnCall { type_name, method, args, line, col } => {
                let base_name = if let Some(bracket_pos) = type_name.find('[') {
                    &type_name[..bracket_pos]
                } else {
                    type_name.as_str()
                };
                let mangled = format!("{}_{}", base_name, method);
                let mut call_args = Vec::new();
                for a in args {
                    call_args.push(self.eval_expr(a));
                }
                self.call_stack.push(super::FrameInfo {
                    function_name: mangled.clone(),
                    line: *line,
                    col: *col,
                });
                let result = self.exec_fn_call_raw(&mangled, &call_args);
                self.call_stack.pop();
                result
            }
            Expr::MethodCall { obj, method, args, line, col } => {
                let obj_val = self.eval_expr(obj);
                let type_name = match &obj_val {
                    RuntimeValue::ClassInstance { class_name, .. } => Some(class_name.clone()),
                    RuntimeValue::StructInstance { name, .. } => Some(name.clone()),
                    RuntimeValue::EnumVariant { enum_name, .. } => Some(enum_name.clone()),
                    _ => None,
                };
                if let Some(tn) = &type_name {
                    let base_tn = if let Some(bracket_pos) = tn.find('[') {
                        &tn[..bracket_pos]
                    } else {
                        tn.as_str()
                    };
                    let mangled = format!("{}_{}", base_tn, method);
                    if self.functions.contains_key(&mangled) {
                        let mut call_args = vec![obj_val];
                        for a in args {
                            call_args.push(self.eval_expr(a));
                        }
                        self.call_stack.push(super::FrameInfo {
                            function_name: mangled.clone(),
                            line: *line,
                            col: *col,
                        });
                        let result = self.exec_fn_call_raw(&mangled, &call_args);
                        self.call_stack.pop();
                        return result;
                    }
                    let prefix = format!("{}_", base_tn);
                    let suffix = format!("_{}", method);
                    let matching_key: Option<String> = self.functions.keys()
                        .find(|k| k.starts_with(&prefix) && k.ends_with(&suffix))
                        .cloned();
                    if let Some(key) = matching_key {
                        let mut call_args = vec![obj_val];
                        for a in args {
                            call_args.push(self.eval_expr(a));
                        }
                        self.call_stack.push(super::FrameInfo {
                            function_name: key.clone(),
                            line: *line,
                            col: *col,
                        });
                        let result = self.exec_fn_call_raw(&key, &call_args);
                        self.call_stack.pop();
                        return result;
                    }
                    self.print_stack_trace();
                    eprintln!("[ERROR] No method '{}' found for type '{}' at line {}, col {}", method, tn, line, col);
                } else {
                    self.print_stack_trace();
                    eprintln!("[ERROR] Method call on non-object type at line {}, col {}", line, col);
                }
                RuntimeValue::Int(0)
            }
            Expr::New { class_name, .. } => {
                let mut fields = HashMap::new();
                if let Some(field_list) = self.class_fields.get(class_name) {
                    for f in field_list {
                        fields.insert(f.name.clone(), RuntimeValue::Int(0));
                    }
                }
                RuntimeValue::ClassInstance {
                    class_name: class_name.clone(),
                    fields,
                }
            }
            Expr::Field { obj, field, .. } => {
                let obj_val = self.eval_expr(obj);
                match &obj_val {
                    RuntimeValue::ClassInstance { fields, .. } => {
                        fields.get(field).cloned().unwrap_or(RuntimeValue::Int(0))
                    }
                    RuntimeValue::StructInstance { fields, .. } => {
                        fields.get(field).cloned().unwrap_or(RuntimeValue::Int(0))
                    }
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::FieldAssign { obj, field, value, .. } => {
                let val = self.eval_expr(value);
                match obj.as_ref() {
                    Expr::Ident(var, ..) => {
                        let mut obj_val = self.eval_expr(obj);
                        match &mut obj_val {
                            RuntimeValue::ClassInstance { fields, .. } => {
                                fields.insert(field.clone(), val.clone());
                            }
                            RuntimeValue::StructInstance { fields, .. } => {
                                fields.insert(field.clone(), val.clone());
                            }
                            _ => {}
                        }
                        self.insert_var(var.clone(), obj_val);
                        val
                    }
                    Expr::Index {
                        obj: inner_obj,
                        index,
                        ..
                    } => {
                        let idx_val = self.eval_expr(index);
                        let arr_val = self.eval_expr(inner_obj);
                        if let RuntimeValue::Array(arr_rc) = &arr_val {
                            let mut arr = arr_rc.borrow_mut();
                            if let RuntimeValue::Int(i) = idx_val {
                                let idx = i as usize;
                                if idx < arr.len() {
                                    if let RuntimeValue::StructInstance { name: sn, fields: mut fs } =
                                        std::mem::replace(&mut arr[idx], RuntimeValue::Int(0))
                                    {
                                        fs.insert(field.clone(), val.clone());
                                        arr[idx] = RuntimeValue::StructInstance { name: sn, fields: fs };
                                    }
                                }
                            }
                        }
                        if let Expr::Ident(arr_name, ..) = inner_obj.as_ref() {
                            self.insert_var(arr_name.clone(), arr_val.clone());
                        }
                        val
                    }
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::UnaryMinus(inner, ..) => {
                let val = self.eval_expr(inner);
                match val {
                    RuntimeValue::Int(n) => RuntimeValue::Int(-n),
                    RuntimeValue::Float(f) => RuntimeValue::Float(-f),
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::UnaryNot(inner, ..) => {
                let val = self.eval_expr(inner);
                RuntimeValue::Bool(!is_truthy(&val))
            }
            Expr::Group(inner, ..) => self.eval_expr(inner),
            Expr::ArrayLit(elems, ..) => {
                let vals: Vec<RuntimeValue> = elems.iter().map(|e| self.eval_expr(e)).collect();
                RuntimeValue::Array(Rc::new(RefCell::new(vals)))
            }
            Expr::Index { obj, index, .. } => {
                let obj_val = self.eval_expr(obj);
                let idx_val = self.eval_expr(index);
                match (obj_val, idx_val) {
                    (RuntimeValue::Array(arr), RuntimeValue::Int(i)) => {
                        let arr = arr.borrow();
                        let idx = i as usize;
                        if idx < arr.len() {
                            arr[idx].clone()
                        } else {
                            RuntimeValue::Int(0)
                        }
                    }
                    (RuntimeValue::String(s), RuntimeValue::Int(i)) => {
                        let idx = i as usize;
                        let b = s.borrow();
                        if idx < b.len() {
                            RuntimeValue::Int(b.as_bytes()[idx] as i64)
                        } else {
                            RuntimeValue::Int(0)
                        }
                    }
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::IndexAssign { obj, index, value, .. } => {
                let idx_val = self.eval_expr(index);
                let val_val = self.eval_expr(value);
                let arr_val = self.eval_expr(obj);
                if let RuntimeValue::Array(arr_rc) = &arr_val {
                    let mut arr = arr_rc.borrow_mut();
                    if let RuntimeValue::Int(i) = idx_val {
                        let idx = i as usize;
                        if idx < arr.len() {
                            arr[idx] = val_val.clone();
                        } else {
                            while arr.len() <= idx {
                                arr.push(RuntimeValue::Int(0));
                            }
                            arr[idx] = val_val.clone();
                        }
                    }
                }
                if let Expr::Ident(name, ..) = obj.as_ref() {
                    self.insert_var(name.clone(), arr_val.clone());
                }
                val_val
            }
            Expr::StructLit { struct_name, fields, .. } => {
                let base_name = if let Some(bracket_pos) = struct_name.find('[') {
                    &struct_name[..bracket_pos]
                } else {
                    struct_name.as_str()
                };
                let def_fields = self.struct_defs.get(base_name).cloned().unwrap_or_default();
                let mut field_map = HashMap::new();
                for (fname, fexpr) in fields {
                    let val = self.eval_expr(fexpr);
                    field_map.insert(fname.clone(), val);
                }
                for (fname, _fty) in &def_fields {
                    field_map.entry(fname.clone()).or_insert(RuntimeValue::Int(0));
                }
                RuntimeValue::StructInstance {
                    name: base_name.to_string(),
                    fields: field_map,
                }
            }
            Expr::EnumRef { enum_name, variant, .. } => {
                RuntimeValue::EnumVariant {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    data: Vec::new(),
                }
            }
            Expr::EnumCtor { enum_name, variant, args, .. } => {
                let data: Vec<RuntimeValue> = args.iter().map(|a| self.eval_expr(a)).collect();
                RuntimeValue::EnumVariant {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    data,
                }
            }
            Expr::Match { value, arms, .. } => {
                let val = self.eval_expr(value);
                for arm in arms {
                    if self.pattern_matches(&arm.pattern, &val) {
                        self.bind_pattern(&arm.pattern, &val);
                        let result = if let Some(stmts) = &arm.body_block {
                            let mut r = RuntimeValue::Void;
                            for s in stmts {
                                r = self.exec_stmt(s);
                                if matches!(r, RuntimeValue::Return(_)) { break; }
                            }
                            r
                        } else {
                            self.eval_expr(&arm.body)
                        };
                        self.unbind_pattern(&arm.pattern);
                        return result;
                    }
                }
                RuntimeValue::Int(0)
            }
            Expr::GenericCall { name, args, line, col, .. } => {
                self.exec_fn_call_at(name, args, *line, *col)
            }
        }
    }

    fn pattern_matches(&self, pattern: &Pattern, value: &RuntimeValue) -> bool {
        match pattern {
            Pattern::Wildcard => true,
            Pattern::EnumVariant { enum_name, variant, bindings: _ } => {
                if let RuntimeValue::EnumVariant { enum_name: en, variant: v, data: _ } = value {
                    let base_en = if let Some(bracket_pos) = en.find('[') {
                        &en[..bracket_pos]
                    } else {
                        en.as_str()
                    };
                    enum_name == base_en && variant == v
                } else {
                    false
                }
            }
            Pattern::Int(n) => {
                if let RuntimeValue::Int(v) = value {
                    *n == *v
                } else {
                    false
                }
            }
            Pattern::String(s) => {
                if let RuntimeValue::String(v) = value {
                    *s == *v.borrow()
                } else {
                    false
                }
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value: &RuntimeValue) {
        if let Pattern::EnumVariant { enum_name: _, variant: _, bindings } = pattern {
            if let RuntimeValue::EnumVariant { data, .. } = value {
                for (i, bname) in bindings.iter().enumerate() {
                    if i < data.len() {
                        self.insert_var(bname.clone(), data[i].clone());
                    }
                }
            }
        }
    }

    fn unbind_pattern(&mut self, pattern: &Pattern) {
        if let Pattern::EnumVariant { bindings, .. } = pattern {
            for bname in bindings {
                self.remove_var(bname);
            }
        }
    }
}
