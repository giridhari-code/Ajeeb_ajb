use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::os::raw::{c_char, c_void};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

extern "C" {
    fn dlopen(filename: *const c_char, flags: i32) -> *mut c_void;
    fn dlsym(handle: *mut c_void, symbol: *const c_char) -> *mut c_void;
    fn dlclose(handle: *mut c_void) -> i32;
    fn dlerror() -> *mut c_char;
}

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Int(i64),
    Float(f64),
    String(Rc<RefCell<String>>),
    Bool(bool),
    Void,
    Array(Rc<RefCell<Vec<RuntimeValue>>>),
    ClassInstance {
        class_name: String,
        fields: HashMap<String, RuntimeValue>,
    },
    StructInstance {
        name: String,
        fields: HashMap<String, RuntimeValue>,
    },
    EnumVariant {
        enum_name: String,
        variant: String,
        data: Vec<RuntimeValue>,
    },
    Return(Box<RuntimeValue>),
    Break,
    Continue,
}

#[derive(Debug, Clone)]
pub struct FrameInfo {
    pub function_name: String,
    pub line: usize,
    pub col: usize,
}

impl Drop for Evaluator {
    fn drop(&mut self) {
        self.close_files();
    }
}

pub struct Evaluator {
    variable_scopes: Vec<HashMap<String, RuntimeValue>>,
    functions: HashMap<String, (Vec<(String, TypeAnnot)>, Vec<Stmt>, TypeAnnot)>,
    class_fields: HashMap<String, Vec<ClassField>>,
    struct_defs: HashMap<String, Vec<(String, TypeAnnot)>>,
    enum_defs: HashMap<String, Vec<EnumVariantDef>>,
    int_buffers: HashMap<String, Vec<i64>>,
    iteration_count: u64,
    program_args: Vec<String>,
    int_to_string: HashMap<i64, Rc<RefCell<String>>>,
    next_string_ptr: i64,
    outbuf_string: Rc<RefCell<String>>,
    open_files: HashMap<String, std::fs::File>,
    tcp_listeners: HashMap<i64, TcpListener>,
    tcp_clients: HashMap<i64, TcpStream>,
    sqlite_dbs: HashMap<i64, String>,
    next_handle: i64,
    call_stack: Vec<FrameInfo>,
    ffi_registry: crate::interop::FfiRegistry,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            variable_scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            class_fields: HashMap::new(),
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
            int_buffers: HashMap::new(),
            iteration_count: 0,
            program_args: Vec::new(),
            int_to_string: HashMap::new(),
            next_string_ptr: 0x1000,
            outbuf_string: Rc::new(RefCell::new(String::new())),
            open_files: HashMap::new(),
            tcp_listeners: HashMap::new(),
            tcp_clients: HashMap::new(),
            sqlite_dbs: HashMap::new(),
            next_handle: 100,
            call_stack: Vec::new(),
            ffi_registry: crate::interop::FfiRegistry::new(),
        }
    }

    pub fn set_program_args(&mut self, args: Vec<String>) {
        self.program_args = args;
    }

    fn push_scope(&mut self) {
        self.variable_scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.variable_scopes.pop();
    }

    fn lookup_var(&self, name: &str) -> Option<RuntimeValue> {
        for scope in self.variable_scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    fn insert_var(&mut self, name: String, val: RuntimeValue) {
        if let Some(scope) = self.variable_scopes.last_mut() {
            scope.insert(name, val);
        }
    }

    fn remove_var(&mut self, name: &str) {
        if let Some(scope) = self.variable_scopes.last_mut() {
            scope.remove(name);
        }
    }

    pub fn close_files(&mut self) {
        for (_path, mut f) in self.open_files.drain() {
            let _ = f.flush();
        }
    }

    pub fn evaluate_program(&mut self, stmts: &[Stmt]) {
        let mut top_stmts: Vec<Stmt> = Vec::new();
        for stmt in stmts {
            match stmt {
                Stmt::Class {
                    name,
                    fields,
                    methods,
                    ..
                } => {
                    self.class_fields.insert(name.clone(), fields.clone());
                    for m in methods {
                        if let Stmt::FnDef {
                            name: mname,
                            params,
                            body,
                            return_type,
                            ..
                        } = m.clone()
                        {
                            let mangled = format!("{}_{}", name, mname);
                            self.functions.insert(mangled, (params, body, return_type));
                        }
                    }
                }
                Stmt::StructDef { name, fields, .. } => {
                    let ft: Vec<(String, TypeAnnot)> = fields.iter().map(|f| (f.name.clone(), f.type_ann.clone())).collect();
                    self.struct_defs.insert(name.clone(), ft);
                }
                Stmt::EnumDef { name, variants, .. } => {
                    self.enum_defs.insert(name.clone(), variants.clone());
                }
                Stmt::FnDef {
                    name,
                    params,
                    body,
                    return_type,
                    ..
                } => {
                    self.functions.insert(
                        name.clone(),
                        (params.clone(), body.clone(), return_type.clone()),
                    );
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
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, body, return_type, .. } = m.clone() {
                                let mangled = format!("{}_{}_{}", base_type_name, trait_name, mname);
                                self.functions.insert(mangled, (params, body, return_type));
                            }
                        }
                    } else {
                        // Inherent impl: mangled as Type_method
                        for m in methods {
                            if let Stmt::FnDef { name: mname, params, body, return_type, .. } = m.clone() {
                                let mangled = format!("{}_{}", base_type_name, mname);
                                self.functions.insert(mangled, (params, body, return_type));
                            }
                        }
                    }
                }
                Stmt::TraitDef { .. } => {} // Traits are just declarations at runtime
                Stmt::Import(decl) if decl.c_import => {
                    // C library import: @import "lib.so" as alias
                    let lib_path = decl.path.join("/");
                    let alias = decl.alias.as_deref().unwrap_or(&decl.path[decl.path.len().saturating_sub(1)]);
                    match self.ffi_registry.load_library(&lib_path) {
                        Ok(_handle) => {
                            self.ffi_registry.register_alias(alias, &lib_path);
                        }
                        Err(e) => {
                            eprintln!("[FFI] {}", e);
                        }
                    }
                }
                Stmt::Import(_) => {} // Module imports handled elsewhere
                other => {
                    top_stmts.push(other.clone());
                }
            }
        }
        for s in &top_stmts {
            self.exec_stmt(s);
        }
        if self.functions.contains_key("main") {
            self.exec_fn_call("main", &[]);
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> RuntimeValue {
        match stmt {
            Stmt::Set { name, value, .. } | Stmt::Const { name, value, .. } => {
                let val = self.eval_expr(value);
                self.insert_var(name.clone(), val);
                RuntimeValue::Void
            }
            Stmt::Expr(expr, ..) => self.eval_expr(expr),
            Stmt::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    self.eval_expr(expr)
                } else {
                    RuntimeValue::Void
                };
                RuntimeValue::Return(Box::new(val))
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
                ..
            } => {
                if is_truthy(&self.eval_expr(condition)) {
                    for s in then_block {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => return r,
                            RuntimeValue::Continue => return r,
                            _ => {}
                        }
                    }
                } else if let Some(el) = else_block {
                    for s in el {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => return r,
                            RuntimeValue::Continue => return r,
                            _ => {}
                        }
                    }
                }
                RuntimeValue::Void
            }
            Stmt::ForLoop {
                init,
                condition,
                update,
                body,
                ..
            } => {
                self.exec_stmt(init);
                let max_iter: u64 = std::env::var("AJEEB_MAX_ITER")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(u64::MAX);
                let mut _fi = 0u64;
                'for_loop: while is_truthy(&self.eval_expr(condition)) {
                    _fi += 1;
                    if _fi > max_iter {
                        eprintln!("[ABORT] For loop exceeded {} iterations (set AJEEB_MAX_ITER to increase)", max_iter);
                        return RuntimeValue::Void;
                    }
                    for s in body {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => break 'for_loop,
                            RuntimeValue::Continue => break,
                            _ => {}
                        }
                    }
                    self.exec_stmt(update);
                }
                RuntimeValue::Void
            }
            Stmt::While { condition, body, .. } => {
                let max_iter: u64 = std::env::var("AJEEB_MAX_ITER")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(u64::MAX);
                let mut _wi = 0u64;
                'while_loop: while is_truthy(&self.eval_expr(condition)) {
                    _wi += 1;
                    if _wi > max_iter {
                        eprintln!("[ABORT] While loop exceeded {} iterations (set AJEEB_MAX_ITER to increase)", max_iter);
                        return RuntimeValue::Void;
                    }
                    for s in body {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => break 'while_loop,
                            RuntimeValue::Continue => break,
                            _ => {}
                        }
                    }
                }
                RuntimeValue::Void
            }
            Stmt::Break { .. } => RuntimeValue::Break,
            Stmt::Continue { .. } => RuntimeValue::Continue,
            Stmt::Import(..) | Stmt::FnDef { .. } | Stmt::Class { .. } | Stmt::StructDef { .. } | Stmt::EnumDef { .. } | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. } => RuntimeValue::Void,
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> RuntimeValue {
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
                        Eq => RuntimeValue::Bool(enum_name == en2 && variant == v2 && runtime_values_eq(&data, &d2)),
                        Neq => RuntimeValue::Bool(enum_name != en2 || variant != v2 || !runtime_values_eq(&data, &d2)),
                        _ => RuntimeValue::Int(0),
                    },
                    (RuntimeValue::Array(a), RuntimeValue::Array(b)) => match op {
                        Eq => RuntimeValue::Bool(runtime_values_eq(&a.borrow(), &b.borrow())),
                        Neq => RuntimeValue::Bool(!runtime_values_eq(&a.borrow(), &b.borrow())),
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
                self.call_stack.push(FrameInfo {
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
                    // Strip generic type args: "Box[Int]" -> "Box"
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
                        self.call_stack.push(FrameInfo {
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
                        self.call_stack.push(FrameInfo {
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
                // Strip generic type args from name: "Box[Int]" -> "Box"
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
                // Fill default values for any missing fields
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
                    // Strip generic type args for comparison: "Option[Int]" -> "Option"
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

    fn print_stack_trace(&self) {
        eprintln!("Stack trace (most recent call first):");
        for (i, frame) in self.call_stack.iter().rev().enumerate() {
            eprintln!("  {}: {} (line {}, col {})", i, frame.function_name, frame.line, frame.col);
        }
    }

    pub fn exec_fn_call_raw(&mut self, name: &str, arg_vals: &[RuntimeValue]) -> RuntimeValue {
        self.exec_fn_call_body(name, arg_vals)
    }

    pub fn exec_fn_call(&mut self, name: &str, args: &[Expr]) -> RuntimeValue {
        self.exec_fn_call_at(name, args, 0, 0)
    }

    pub fn exec_fn_call_at(&mut self, name: &str, args: &[Expr], line: usize, col: usize) -> RuntimeValue {
        self.iteration_count += 1;
        if self.iteration_count.is_multiple_of(100000) && std::env::var("AJEEB_TRACE").is_ok() {
            eprintln!(
                "[ITER {}] fn: {} args:{}",
                self.iteration_count,
                name,
                args.len()
            );
        }
        let max_iter: u64 = std::env::var("AJEEB_MAX_ITER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(u64::MAX);
        if self.iteration_count > max_iter {
            eprintln!(
                "[ITER {}] ABORT (set AJEEB_MAX_ITER to increase)",
                self.iteration_count
            );
            return RuntimeValue::Int(0);
        }

        let arg_vals: Vec<RuntimeValue> = args.iter().map(|a| self.eval_expr(a)).collect();

        self.call_stack.push(FrameInfo {
            function_name: name.to_string(),
            line,
            col,
        });
        let result = self.exec_fn_call_body(name, &arg_vals);
        self.call_stack.pop();
        result
    }

    fn exec_fn_call_body(&mut self, name: &str, arg_vals: &[RuntimeValue]) -> RuntimeValue {
        match name {
            "print" | "println" => {
                let nl = name == "println";
                for a in arg_vals {
                    match a {
                        RuntimeValue::Int(n) => print!("{}", n),
                        RuntimeValue::Float(f) => print!("{}", f),
                        RuntimeValue::String(s) => {
                            let s_clean: String = s.borrow().chars().take_while(|&c| c != '\0').collect();
                            print!("{}", s_clean);
                        }
                        RuntimeValue::Bool(b) => print!("{}", b),
                        RuntimeValue::Array(arr_rc) => {
                            let arr = arr_rc.borrow();
                            print!("[");
                            for (i, e) in arr.iter().enumerate() {
                                if i > 0 {
                                    print!(", ");
                                }
                                match e {
                                    RuntimeValue::Int(n) => print!("{}", n),
                                    RuntimeValue::String(s) => {
                                        let s_clean: String = s.borrow().chars().take_while(|&c| c != '\0').collect();
                                        print!("\"{}\"", s_clean);
                                    }
                                    RuntimeValue::Bool(b) => print!("{}", b),
                                    RuntimeValue::Array(inner_rc) => {
                                        let inner = inner_rc.borrow();
                                        print!("[");
                                        for (j, ee) in inner.iter().enumerate() {
                                            if j > 0 {
                                                print!(", ");
                                            }
                                            match ee {
                                                RuntimeValue::Int(n) => print!("{}", n),
                                                RuntimeValue::String(s) => {
                                                    print!("\"{}\"", s.borrow())
                                                }
                                                RuntimeValue::Bool(b) => print!("{}", b),
                                                _ => print!("<?>"),
                                            }
                                        }
                                        print!("]");
                                    }
                                    _ => print!("<?>"),
                                }
                            }
                            print!("]");
                        }
                        RuntimeValue::ClassInstance { class_name, .. } => {
                            print!("<{} instance>", class_name)
                        }
                        RuntimeValue::StructInstance { name: sn, .. } => {
                            print!("<{} struct>", sn)
                        }
                        RuntimeValue::EnumVariant { enum_name, variant, data } => {
                            print!("{}::{}", enum_name, variant);
                            if !data.is_empty() {
                                print!("(");
                                for (i, d) in data.iter().enumerate() {
                                    if i > 0 { print!(", "); }
                                    print_value(d);
                                }
                                print!(")");
                            }
                        }
                        RuntimeValue::Void => print!("void"),
                        RuntimeValue::Return(v) => print!("<return {:?}>", v),
                        RuntimeValue::Break => print!("break"),
                        RuntimeValue::Continue => print!("continue"),
                    }
                }
                if nl {
                    println!();
                }
                RuntimeValue::Void
            }
            "itoa" => {
                if let Some(RuntimeValue::Int(n)) = arg_vals.first() {
                    RuntimeValue::String(Rc::new(RefCell::new(n.to_string())))
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new("0".to_string())))
                }
            }
            "len" => {
                if let Some(RuntimeValue::String(s)) = arg_vals.first() {
                    RuntimeValue::Int(s.borrow().len() as i64)
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "arr_len" => {
                if let Some(RuntimeValue::Array(arr)) = arg_vals.first() {
                    RuntimeValue::Int(arr.borrow().len() as i64)
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "charCode" => {
                if arg_vals.len() >= 2 {
                    let s = match &arg_vals[0] {
                        RuntimeValue::String(s) => Some(s.clone()),
                        RuntimeValue::Int(ptr) => self.int_to_string.get(ptr).cloned(),
                        _ => None,
                    };
                    if let (Some(s), RuntimeValue::Int(i)) = (s, &arg_vals[1]) {
                        let idx = *i as usize;
                        let b = s.borrow();
                        if idx < b.len() {
                            let val = b.as_bytes()[idx] as i64;
                            return RuntimeValue::Int(val);
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "strcmp" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let av: &str = &a.borrow();
                        let bv: &str = &b.borrow();
                        return RuntimeValue::Int(if av < bv {
                            -1
                        } else if av > bv {
                            1
                        } else {
                            0
                        });
                    }
                }
                RuntimeValue::Int(0)
            }
            "readFile" => {
                let path = if let Some(RuntimeValue::String(s)) = arg_vals.first() {
                    s.borrow().clone()
                } else {
                    String::new()
                };
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(content)))
            }
            "writeFile" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::String(content)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let bytes: Vec<u8> = content.borrow().bytes().take_while(|&b| b != 0).collect();
                        let _ = std::fs::write(path.borrow().as_str(), &bytes);
                    }
                }
                RuntimeValue::Void
            }
            "writeAppend" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::String(content)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let path_str = path.borrow().clone();
                        let f = self.open_files.entry(path_str).or_insert_with_key(|key| {
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(key.as_str())
                                .expect("writeAppend: failed to open file")
                        });
                        let bytes: Vec<u8> = content.borrow().bytes().take_while(|&b| b != 0).collect();
                        let _ = f.write_all(&bytes);
                    }
                }
                RuntimeValue::Void
            }
            "exec" => {
                if let Some(RuntimeValue::String(cmd)) = arg_vals.first() {
                    let cmd_clean: String = cmd.borrow().chars().take_while(|&c| c != '\0').collect();
                    let exit_code = std::process::Command::new("sh")
                        .args(["-c", &cmd_clean])
                        .status()
                        .map(|s| s.code().unwrap_or(-1))
                        .unwrap_or(-1);
                    return RuntimeValue::Int(exit_code as i64);
                }
                RuntimeValue::Int(0)
            }
            "mkdir" => {
                if let Some(RuntimeValue::String(path)) = arg_vals.first() {
                    let path_clean: String = path.borrow().chars().take_while(|&c| c != '\0').collect();
                    let exit_code = std::process::Command::new("mkdir")
                        .args(["-p", &path_clean])
                        .status()
                        .map(|s| s.code().unwrap_or(-1))
                        .unwrap_or(-1);
                    return RuntimeValue::Int(exit_code as i64);
                }
                RuntimeValue::Int(0)
            }
            "readArg" => {
                let idx = if let Some(RuntimeValue::Int(n)) = arg_vals.first() {
                    *n as usize
                } else {
                    0
                };
                if idx < self.program_args.len() {
                    RuntimeValue::String(Rc::new(RefCell::new(self.program_args[idx].clone())))
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "getStateBuf" => {
                let key = "__state__".to_string();
                self.int_buffers
                    .entry(key.clone())
                    .or_insert_with(|| vec![0i64; 16384]);
                RuntimeValue::String(Rc::new(RefCell::new(key)))
            }
            "getOutbuf" => {
                self.outbuf_string.borrow_mut().clear();
                RuntimeValue::String(self.outbuf_string.clone())
            }
            "rdB" | "getInt" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(buf_name), RuntimeValue::Int(offset)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let idx = (offset / 8) as usize;
                        if let Some(buf) = self.int_buffers.get(buf_name.borrow().as_str()) {
                            if idx < buf.len() {
                                return RuntimeValue::Int(buf[idx]);
                            }
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "wrB" | "setInt" => {
                if arg_vals.len() >= 3 {
                    if let RuntimeValue::String(buf_name) = &arg_vals[0] {
                        if let RuntimeValue::Int(offset) = &arg_vals[1] {
                            let idx = (offset / 8) as usize;
                            let name = buf_name.borrow().clone();
                            let buf = self
                                .int_buffers
                                .entry(name)
                                .or_insert_with(|| vec![0i64; 16384]);
                            if idx < buf.len() {
                                match &arg_vals[2] {
                                    RuntimeValue::Int(v) => buf[idx] = *v,
                                    RuntimeValue::String(s) => {
                                        let ptr = self.next_string_ptr;
                                        self.next_string_ptr += 1;
                                        self.int_to_string.insert(ptr, s.clone());
                                        buf[idx] = ptr;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                RuntimeValue::Void
            }
            "strcpy" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(dst), RuntimeValue::String(src)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        *dst.borrow_mut() = src.borrow().clone();
                    }
                }
                RuntimeValue::Void
            }
            "strSet" => {
                if arg_vals.len() >= 3 {
                    if let (
                        RuntimeValue::String(s),
                        RuntimeValue::Int(idx),
                        RuntimeValue::Int(ch),
                    ) = (&arg_vals[0], &arg_vals[1], &arg_vals[2])
                    {
                        let i = *idx as usize;
                        let mut buf = s.borrow_mut();
                        let blen = buf.len();
                        if i >= blen {
                            buf.extend(std::iter::repeat_n('\0', i + 1 - blen));
                        }
                        buf.replace_range(
                            i..=i,
                            &String::from(char::from_u32(*ch as u32).unwrap_or('\0')),
                        );
                    }
                }
                RuntimeValue::Void
            }
            "writeByte" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(path), RuntimeValue::Int(byte)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let path_str = path.borrow().clone();
                        let f = self.open_files.entry(path_str).or_insert_with_key(|key| {
                            OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(key.as_str())
                                .expect("writeByte: failed to open file")
                        });
                        let _ = f.write_all(&[*byte as u8]);
                    }
                }
                RuntimeValue::Void
            }
            "chr" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(s), RuntimeValue::Int(i)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let idx = *i as usize;
                        let b = s.borrow();
                        if idx < b.len() {
                            let val = b.as_bytes()[idx] as i64;
                            return RuntimeValue::Int(val);
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "chr_str" => {
                if let Some(RuntimeValue::Int(code)) = arg_vals.first() {
                    let c = (*code).max(0).min(255) as u8 as char;
                    RuntimeValue::String(Rc::new(RefCell::new(c.to_string())))
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "getStr" => {
                if let Some(RuntimeValue::Int(ptr)) = arg_vals.first() {
                    if let Some(s) = self.int_to_string.get(ptr) {
                        return RuntimeValue::String(s.clone());
                    }
                }
                RuntimeValue::String(Rc::new(RefCell::new(String::new())))
            }
            "rdPos" => {
                if let Some(RuntimeValue::String(buf_name)) = arg_vals.first() {
                    if let Some(buf) = self.int_buffers.get(buf_name.borrow().as_str()) {
                        if !buf.is_empty() {
                            return RuntimeValue::Int(buf[0]);
                        }
                    }
                }
                RuntimeValue::Int(0)
            }
            "wrPos" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(buf_name), RuntimeValue::Int(v)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let name = buf_name.borrow().clone();
                        let buf = self
                            .int_buffers
                            .entry(name)
                            .or_insert_with(|| vec![0i64; 16384]);
                        if !buf.is_empty() {
                            buf[0] = *v;
                        }
                    }
                }
                RuntimeValue::Void
            }
            "isDigit" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    return RuntimeValue::Int((*c >= 48 && *c <= 57) as i64);
                }
                RuntimeValue::Int(0)
            }
            "isAlpha" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    let uc = *c as u8;
                    return RuntimeValue::Int(
                        ((65..=90).contains(&uc) || (97..=122).contains(&uc) || uc == 95) as i64,
                    );
                }
                RuntimeValue::Int(0)
            }
            "isAlphaNum" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    let uc = *c as u8;
                    return RuntimeValue::Int(
                        ((48..=57).contains(&uc)
                            || (65..=90).contains(&uc)
                            || (97..=122).contains(&uc)
                            || uc == 95) as i64,
                    );
                }
                RuntimeValue::Int(0)
            }
            "isSpace" => {
                if let Some(RuntimeValue::Int(c)) = arg_vals.first() {
                    return RuntimeValue::Int((*c == 32 || *c == 9 || *c == 10 || *c == 13) as i64);
                }
                RuntimeValue::Int(0)
            }
            "strcmp_ajeeb" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let a_trim: String =
                            a.borrow().chars().take_while(|&c| c != '\0').collect();
                        let b_trim: String =
                            b.borrow().chars().take_while(|&c| c != '\0').collect();
                        if a_trim == b_trim {
                            return RuntimeValue::Int(0);
                        }
                        if a_trim < b_trim {
                            return RuntimeValue::Int(-1);
                        }
                        return RuntimeValue::Int(1);
                    }
                }
                RuntimeValue::Int(0)
            }
            "str_concat" => {
                if arg_vals.len() >= 2 {
                    if let (RuntimeValue::String(a), RuntimeValue::String(b)) =
                        (&arg_vals[0], &arg_vals[1])
                    {
                        let result = format!("{}{}", a.borrow(), b.borrow());
                        return RuntimeValue::String(Rc::new(RefCell::new(result)));
                    }
                }
                RuntimeValue::String(Rc::new(RefCell::new(String::new())))
            }
            "substring" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                let start = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::Int(i) = a {
                            Some(*i as usize)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
                let end = arg_vals
                    .get(2)
                    .and_then(|a| {
                        if let RuntimeValue::Int(i) = a {
                            Some(*i as usize)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(s.len());
                let end = end.min(s.len());
                let sub: String = s
                    .chars()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .collect();
                RuntimeValue::String(Rc::new(RefCell::new(sub)))
            }
            "indexOf" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let search: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                if let Some(pos) = s.find(search) {
                    RuntimeValue::Int(pos as i64)
                } else {
                    RuntimeValue::Int(-1)
                }
            }
            "contains" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let search: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.contains(search) { 1 } else { 0 })
            }
            "toUpperCase" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.to_uppercase())))
            }
            "toLowerCase" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.to_lowercase())))
            }
            "trim" => {
                let s_guard: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { s_guard = ss.borrow(); &*s_guard }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.trim().to_string())))
            }
            "split" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let delim: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                let parts: Vec<RuntimeValue> = if delim.is_empty() {
                    s.chars()
                        .map(|c| RuntimeValue::String(Rc::new(RefCell::new(c.to_string()))))
                        .collect()
                } else {
                    s.split(delim)
                        .map(|p| RuntimeValue::String(Rc::new(RefCell::new(p.to_string()))))
                        .collect()
                };
                RuntimeValue::Array(Rc::new(RefCell::new(parts)))
            }
            "replace" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let sg3: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let from: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                let to: &str = match arg_vals.get(2) {
                    Some(RuntimeValue::String(ss)) => { sg3 = ss.borrow(); &*sg3 }
                    _ => "",
                };
                RuntimeValue::String(Rc::new(RefCell::new(s.replace(from, to))))
            }
            "startsWith" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let prefix: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.starts_with(prefix) { 1 } else { 0 })
            }
            "endsWith" => {
                let sg1: std::cell::Ref<String>;
                let sg2: std::cell::Ref<String>;
                let s: &str = match arg_vals.first() {
                    Some(RuntimeValue::String(ss)) => { sg1 = ss.borrow(); &*sg1 }
                    _ => "",
                };
                let suffix: &str = match arg_vals.get(1) {
                    Some(RuntimeValue::String(ss)) => { sg2 = ss.borrow(); &*sg2 }
                    _ => "",
                };
                RuntimeValue::Int(if s.ends_with(suffix) { 1 } else { 0 })
            }
            "tcp_listen" => {
                if let Some(RuntimeValue::Int(port)) = arg_vals.first() {
                    match TcpListener::bind(format!("127.0.0.1:{}", port)) {
                        Ok(listener) => {
                            let fd = self.next_handle;
                            self.next_handle += 1;
                            self.tcp_listeners.insert(fd, listener);
                            RuntimeValue::Int(fd)
                        }
                        Err(e) => {
                            eprintln!("[TCP] listen error on port {}: {}", port, e);
                            RuntimeValue::Int(0)
                        }
                    }
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "tcp_accept" => {
                if let Some(RuntimeValue::Int(fd)) = arg_vals.first() {
                    if let Some(listener) = self.tcp_listeners.get(fd) {
                        match listener.accept() {
                            Ok((stream, _addr)) => {
                                let client_fd = self.next_handle;
                                self.next_handle += 1;
                                stream.set_nonblocking(true).ok();
                                self.tcp_clients.insert(client_fd, stream);
                                RuntimeValue::Int(client_fd)
                            }
                            Err(_) => RuntimeValue::Int(0),
                        }
                    } else {
                        RuntimeValue::Int(0)
                    }
                } else {
                    RuntimeValue::Int(0)
                }
            }
            "tcp_read" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let max = arg_vals.get(1).and_then(|a| if let RuntimeValue::Int(m) = a { Some(*m as usize) } else { None }).unwrap_or(4096);
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let mut buf = vec![0u8; max];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            buf.truncate(n);
                            RuntimeValue::String(Rc::new(RefCell::new(
                                String::from_utf8_lossy(&buf).to_string()
                            )))
                        }
                        _ => RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                    }
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "tcp_write" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let data_guard: std::cell::Ref<String>;
                let data: &[u8] = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => { data_guard = s.borrow(); data_guard.as_bytes() }
                    _ => &[],
                };
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let _ = stream.write_all(data);
                }
                RuntimeValue::Void
            }
            "tcp_close" => {
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                self.tcp_clients.remove(&fd);
                self.tcp_listeners.remove(&fd);
                RuntimeValue::Void
            }
            "tcp_connect" => {
                let host = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                let port = match arg_vals.get(1) {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(stream) => {
                        stream.set_nonblocking(true).ok();
                        let fd = self.next_handle;
                        self.next_handle += 1;
                        self.tcp_clients.insert(fd, stream);
                        RuntimeValue::Int(fd)
                    }
                    Err(e) => {
                        eprintln!("[TCP] connect error to {}:{}: {}", host, port, e);
                        RuntimeValue::Int(0)
                    }
                }
            }
            "dns_lookup" => {
                let hostname = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                };
                match format!("{}:0", hostname).to_socket_addrs() {
                    Ok(mut addrs) => {
                        if let Some(addr) = addrs.next() {
                            RuntimeValue::String(Rc::new(RefCell::new(addr.ip().to_string())))
                        } else {
                            RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                        }
                    }
                    Err(e) => {
                        eprintln!("[DNS] lookup error for '{}': {}", hostname, e);
                        RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                    }
                }
            }
            "tls_connect" => {
                // TLS only available in native mode; fall back to plain TCP in interpreter
                eprintln!("[TLS] tls_connect fallback to plain TCP (use native mode for real TLS)");
                let host = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                let port = match arg_vals.get(1) {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                match TcpStream::connect(format!("{}:{}", host, port)) {
                    Ok(stream) => {
                        stream.set_nonblocking(true).ok();
                        let fd = self.next_handle;
                        self.next_handle += 1;
                        self.tcp_clients.insert(fd, stream);
                        RuntimeValue::Int(fd)
                    }
                    Err(e) => {
                        eprintln!("[TLS] connect error to {}:{}: {}", host, port, e);
                        RuntimeValue::Int(0)
                    }
                }
            }
            "tls_read" => {
                // Fallback to plain tcp_read in interpreter
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let max = arg_vals.get(1).and_then(|a| if let RuntimeValue::Int(m) = a { Some(*m as usize) } else { None }).unwrap_or(4096);
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let mut buf = vec![0u8; max];
                    match stream.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            buf.truncate(n);
                            RuntimeValue::String(Rc::new(RefCell::new(
                                String::from_utf8_lossy(&buf).to_string()
                            )))
                        }
                        _ => RuntimeValue::String(Rc::new(RefCell::new(String::new()))),
                    }
                } else {
                    RuntimeValue::String(Rc::new(RefCell::new(String::new())))
                }
            }
            "tls_write" => {
                // Fallback to plain write in interpreter
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                let data_guard: std::cell::Ref<String>;
                let data: &[u8] = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => { data_guard = s.borrow(); data_guard.as_bytes() }
                    _ => &[],
                };
                if let Some(stream) = self.tcp_clients.get_mut(&fd) {
                    let _ = stream.write_all(data);
                }
                RuntimeValue::Void
            }
            "tls_close" => {
                // Fallback to plain close in interpreter
                let fd = arg_vals.first().and_then(|a| if let RuntimeValue::Int(f) = a { Some(*f) } else { None }).unwrap_or(0);
                self.tcp_clients.remove(&fd);
                self.tcp_listeners.remove(&fd);
                RuntimeValue::Void
            }
            "now_ms" => {
                let ms = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64;
                RuntimeValue::Int(ms)
            }
            "sqlite_open" => {
                let path = arg_vals.first().and_then(|a| if let RuntimeValue::String(s) = a { Some(s.borrow().clone()) } else { None }).unwrap_or_default();
                let handle = self.next_handle;
                self.next_handle += 1;
                self.sqlite_dbs.insert(handle, path);
                RuntimeValue::Int(handle)
            }
            "sqlite_close" => {
                let handle = arg_vals.first().and_then(|a| if let RuntimeValue::Int(h) = a { Some(*h) } else { None }).unwrap_or(0);
                self.sqlite_dbs.remove(&handle);
                RuntimeValue::Void
            }
            "sqlite_exec" => {
                // Stub — real SQLite only in C runtime
                RuntimeValue::Int(0)
            }
            "sqlite_query" => {
                // Stub — real SQLite only in C runtime
                RuntimeValue::Array(Rc::new(RefCell::new(Vec::new())))
            }
            "sqlite_last_error" => {
                RuntimeValue::String(Rc::new(RefCell::new("SQLite available only in native mode".to_string())))
            }
            "assert_eq" => {
                if arg_vals.len() >= 2 {
                    let got = format!("{:?}", arg_vals[0]);
                    let expected = format!("{:?}", arg_vals[1]);
                    if got != expected {
                        eprintln!("FAIL: assert_eq expected '{}' got '{}'", expected, got);
                    }
                }
                RuntimeValue::Void
            }
            "assert_neq" => {
                if arg_vals.len() >= 2 {
                    let got = format!("{:?}", arg_vals[0]);
                    let expected = format!("{:?}", arg_vals[1]);
                    if got == expected {
                        eprintln!("FAIL: assert_neq expected different values, got '{}'", got);
                    }
                }
                RuntimeValue::Void
            }
            "assert_contains" => {
                if let (Some(RuntimeValue::String(s)), Some(RuntimeValue::String(sub))) = (arg_vals.first(), arg_vals.get(1)) {
                    if !s.borrow().contains(&sub.borrow().as_str()) {
                        eprintln!("FAIL: '{}' does not contain '{}'", s.borrow(), sub.borrow());
                    }
                }
                RuntimeValue::Void
            }
            "lib_open" => {
                let path = match arg_vals.first() {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => String::new(),
                };
                if path.is_empty() {
                    return RuntimeValue::Int(-1);
                }
                match CString::new(path.as_str()) {
                    Ok(cpath) => unsafe {
                        let handle = dlopen(cpath.as_ptr(), 1 | 0x001); // RTLD_NOW | RTLD_LOCAL
                        if handle.is_null() {
                            let err = dlerror();
                            if !err.is_null() {
                                let msg = std::ffi::CStr::from_ptr(err).to_string_lossy().to_string();
                                eprintln!("[FFI] dlopen error: {}", msg);
                            }
                            RuntimeValue::Int(-1)
                        } else {
                            RuntimeValue::Int(handle as i64)
                        }
                    },
                    Err(_) => RuntimeValue::Int(-1),
                }
            }
            "lib_sym" => {
                let handle = match arg_vals.first() {
                    Some(RuntimeValue::Int(h)) => *h,
                    _ => return RuntimeValue::Int(0),
                };
                let sym_name = match arg_vals.get(1) {
                    Some(RuntimeValue::String(s)) => s.borrow().clone(),
                    _ => return RuntimeValue::Int(0),
                };
                match CString::new(sym_name.as_str()) {
                    Ok(cname) => unsafe {
                        let ptr = dlsym(handle as *mut c_void, cname.as_ptr());
                        RuntimeValue::Int(ptr as i64)
                    },
                    Err(_) => RuntimeValue::Int(0),
                }
            }
            "lib_call" => {
                let fn_ptr = match arg_vals.first() {
                    Some(RuntimeValue::Int(p)) => *p,
                    _ => return RuntimeValue::Int(0),
                };
                let args_arr = match arg_vals.get(1) {
                    Some(RuntimeValue::Array(a)) => a.borrow().clone(),
                    _ => Vec::new(),
                };
                let _ret_type = match arg_vals.get(2) {
                    Some(RuntimeValue::Int(r)) => *r,
                    _ => 0,
                };
                let c_args: Vec<i64> = args_arr.iter().map(|v| match v {
                    RuntimeValue::Int(n) => *n,
                    _ => 0,
                }).collect();
                unsafe {
                    let result = match c_args.len() {
                        0 => {
                            let f: unsafe extern "C" fn() -> i64 = std::mem::transmute(fn_ptr as usize);
                            f()
                        }
                        1 => {
                            let f: unsafe extern "C" fn(i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0])
                        }
                        2 => {
                            let f: unsafe extern "C" fn(i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1])
                        }
                        3 => {
                            let f: unsafe extern "C" fn(i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2])
                        }
                        4 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3])
                        }
                        5 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3], c_args[4])
                        }
                        6 => {
                            let f: unsafe extern "C" fn(i64, i64, i64, i64, i64, i64) -> i64 = std::mem::transmute(fn_ptr as usize);
                            f(c_args[0], c_args[1], c_args[2], c_args[3], c_args[4], c_args[5])
                        }
                        _ => 0,
                    };
                    RuntimeValue::Int(result)
                }
            }
            "call_fn" => {
                if let Some(RuntimeValue::String(fn_name)) = arg_vals.first() {
                    let name: &str = &fn_name.borrow();
                    return self.exec_fn_call_body(name, &arg_vals[1..]);
                }
                RuntimeValue::Array(Rc::new(RefCell::new(Vec::new())))
            }
            _ => {
                if self.class_fields.contains_key(name) && arg_vals.is_empty() {
                    let mut fields = HashMap::new();
                    if let Some(field_list) = self.class_fields.get(name) {
                        for f in field_list {
                            fields.insert(f.name.clone(), RuntimeValue::Int(0));
                        }
                    }
                    return RuntimeValue::ClassInstance {
                        class_name: name.to_string(),
                        fields,
                    };
                }
                if let Some((params, body, _)) = self.functions.get(name).cloned() {
                    self.push_scope();
                    for (i, (pname, _)) in params.iter().enumerate() {
                        let val = if i < arg_vals.len() {
                            arg_vals[i].clone()
                        } else {
                            RuntimeValue::Int(0)
                        };
                        self.insert_var(pname.clone(), val);
                    }
                    let mut result = RuntimeValue::Void;
                    for s in &body {
                        let r = self.exec_stmt(s);
                        if let RuntimeValue::Return(val) = r {
                            result = *val;
                            break;
                        }
                    }
                    self.pop_scope();
                    return result;
                } else {
                    self.print_stack_trace();
                    eprintln!(
                        "[ERROR] Unknown function '{}' called with {} args",
                        name,
                        arg_vals.len()
                    );
                }
                RuntimeValue::Void
            }
        }
    }
}

fn print_value(val: &RuntimeValue) {
    match val {
        RuntimeValue::Int(n) => print!("{}", n),
        RuntimeValue::Float(f) => print!("{}", f),
        RuntimeValue::String(s) => print!("\"{}\"", s.borrow()),
        RuntimeValue::Bool(b) => print!("{}", b),
        RuntimeValue::EnumVariant { enum_name, variant, data } => {
            print!("{}::{}", enum_name, variant);
            if !data.is_empty() {
                print!("(");
                for (i, d) in data.iter().enumerate() {
                    if i > 0 { print!(", "); }
                    print_value(d);
                }
                print!(")");
            }
        }
        _ => print!("<?>"),
    }
}

fn runtime_values_eq(a: &[RuntimeValue], b: &[RuntimeValue]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (x, y) in a.iter().zip(b.iter()) {
        match (x, y) {
            (RuntimeValue::Int(xn), RuntimeValue::Int(yn)) => { if xn != yn { return false; } }
            (RuntimeValue::Float(xf), RuntimeValue::Float(yf)) => { if xf != yf { return false; } }
            (RuntimeValue::Bool(xb), RuntimeValue::Bool(yb)) => { if xb != yb { return false; } }
            (RuntimeValue::String(xs), RuntimeValue::String(ys)) => { if *xs.borrow() != *ys.borrow() { return false; } }
            (RuntimeValue::EnumVariant { enum_name: xe, variant: xv, data: xd },
             RuntimeValue::EnumVariant { enum_name: ye, variant: yv, data: yd }) => {
                if xe != ye || xv != yv || !runtime_values_eq(xd, yd) { return false; }
            }
            _ => { return false; }
        }
    }
    true
}

fn is_truthy(val: &RuntimeValue) -> bool {
    match val {
        RuntimeValue::Int(n) => *n != 0,
        RuntimeValue::Bool(b) => *b,
        RuntimeValue::String(s) => !s.borrow().is_empty(),
        RuntimeValue::Array(arr) => !arr.borrow().is_empty(),
        RuntimeValue::Return(val) => is_truthy(val),
        _ => true,
    }
}
