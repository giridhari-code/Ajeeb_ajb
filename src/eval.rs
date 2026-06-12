use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub enum RuntimeValue {
    Int(i64),
    String(Rc<RefCell<String>>),
    Bool(bool),
    Void,
    Array(Vec<RuntimeValue>),
    ClassInstance {
        class_name: String,
        fields: HashMap<String, RuntimeValue>,
    },
    Return(Box<RuntimeValue>),
    Break,
    Continue,
}

pub struct Evaluator {
    variables: HashMap<String, RuntimeValue>,
    functions: HashMap<String, (Vec<(String, TypeAnnot)>, Vec<Stmt>, TypeAnnot)>,
    class_fields: HashMap<String, Vec<ClassField>>,
    int_buffers: HashMap<String, Vec<i64>>,
    iteration_count: u64,
    program_args: Vec<String>,
    int_to_string: HashMap<i64, Rc<RefCell<String>>>,
    next_string_ptr: i64,
    outbuf_string: Rc<RefCell<String>>,
}

impl Evaluator {
    pub fn new() -> Self {
        Evaluator {
            variables: HashMap::new(),
            functions: HashMap::new(),
            class_fields: HashMap::new(),
            int_buffers: HashMap::new(),
            iteration_count: 0,
            program_args: Vec::new(),
            int_to_string: HashMap::new(),
            next_string_ptr: 0x1000,
            outbuf_string: Rc::new(RefCell::new(String::new())),
        }
    }

    pub fn set_program_args(&mut self, args: Vec<String>) {
        self.program_args = args;
    }

    pub fn evaluate_program(&mut self, stmts: &[Stmt]) {
        let mut top_stmts: Vec<Stmt> = Vec::new();
        for stmt in stmts {
            match stmt {
                Stmt::Class {
                    name,
                    fields,
                    methods,
                } => {
                    self.class_fields.insert(name.clone(), fields.clone());
                    for m in methods {
                        if let Stmt::FnDef {
                            name: mname,
                            params,
                            body,
                            return_type,
                        } = m.clone()
                        {
                            let mangled = format!("{}_{}", name, mname);
                            self.functions.insert(mangled, (params, body, return_type));
                        }
                    }
                }
                Stmt::FnDef {
                    name,
                    params,
                    body,
                    return_type,
                } => {
                    self.functions.insert(
                        name.clone(),
                        (params.clone(), body.clone(), return_type.clone()),
                    );
                }
                other => {
                    top_stmts.push(other.clone());
                }
            }
        }
        // Execute top-level statements (globals) before calling main
        for s in &top_stmts {
            self.exec_stmt(s);
        }
        if self.functions.contains_key("main") {
            self.exec_fn_call("main", &[]);
        }
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> RuntimeValue {
        match stmt {
            Stmt::Let { name, value, .. } | Stmt::Const { name, value, .. } => {
                let val = self.eval_expr(value);
                self.variables.insert(name.clone(), val);
                RuntimeValue::Void
            }
            Stmt::Expr(expr) => self.eval_expr(expr),
            Stmt::Return { value } => {
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
            } => {
                if is_truthy(&self.eval_expr(condition)) {
                    for s in then_block {
                        let r = self.exec_stmt(s);
                        if let RuntimeValue::Return(_) = r {
                            return r;
                        }
                    }
                } else if let Some(el) = else_block {
                    for s in el {
                        let r = self.exec_stmt(s);
                        if let RuntimeValue::Return(_) = r {
                            return r;
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
            Stmt::While { condition, body } => {
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
            Stmt::Break => RuntimeValue::Break,
            Stmt::Continue => RuntimeValue::Continue,
            Stmt::FnDef { .. } | Stmt::Class { .. } => RuntimeValue::Void,
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> RuntimeValue {
        match expr {
            Expr::Number(n) => RuntimeValue::Int(*n),
            Expr::StringLit(s) => RuntimeValue::String(Rc::new(RefCell::new(s.clone()))),
            Expr::Bool(b) => RuntimeValue::Bool(*b),
            Expr::Ident(id) => self.variables.get(id).cloned().unwrap_or_else(|| {
                eprintln!("[ERROR] Unknown variable '{}' — treating as 0", id);
                RuntimeValue::Int(0)
            }),
            Expr::Binary { left, op, right } => {
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
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::Assign { name, value } => {
                let val = self.eval_expr(value);
                self.variables.insert(name.clone(), val.clone());
                val
            }
            Expr::FnCall { name, args } => self.exec_fn_call(name, args),
            Expr::New { class_name } => {
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
            Expr::Field { obj, field } => {
                if let RuntimeValue::ClassInstance { fields, .. } = self.eval_expr(obj) {
                    fields.get(field).cloned().unwrap_or(RuntimeValue::Int(0))
                } else {
                    RuntimeValue::Int(0)
                }
            }
            Expr::FieldAssign { obj, field, value } => {
                let val = self.eval_expr(value);
                // Handle obj.field = value through index chains like arr[i].field = val
                match obj.as_ref() {
                    Expr::Ident(var) => {
                        if let RuntimeValue::ClassInstance {
                            class_name,
                            mut fields,
                        } = self.eval_expr(obj)
                        {
                            fields.insert(field.clone(), val.clone());
                            let updated = RuntimeValue::ClassInstance { class_name, fields };
                            self.variables.insert(var.clone(), updated.clone());
                            updated
                        } else {
                            RuntimeValue::Int(0)
                        }
                    }
                    Expr::Index {
                        obj: inner_obj,
                        index,
                    } => {
                        // arr[i].field = val — evaluate array, mutate element, store back
                        let idx_val = self.eval_expr(index);
                        let mut arr_val = self.eval_expr(inner_obj);
                        if let RuntimeValue::Array(ref mut arr) = arr_val {
                            if let RuntimeValue::Int(i) = idx_val {
                                let idx = i as usize;
                                if idx < arr.len() {
                                    if let RuntimeValue::ClassInstance {
                                        class_name,
                                        mut fields,
                                    } = std::mem::replace(&mut arr[idx], RuntimeValue::Int(0))
                                    {
                                        fields.insert(field.clone(), val.clone());
                                        arr[idx] =
                                            RuntimeValue::ClassInstance { class_name, fields };
                                    }
                                }
                            }
                        }
                        if let Expr::Ident(arr_name) = inner_obj.as_ref() {
                            self.variables.insert(arr_name.clone(), arr_val.clone());
                        }
                        val
                    }
                    _ => RuntimeValue::Int(0),
                }
            }
            Expr::UnaryNot(inner) => {
                let val = self.eval_expr(inner);
                RuntimeValue::Bool(!is_truthy(&val))
            }
            Expr::Group(inner) => self.eval_expr(inner),
            Expr::ArrayLit(elems) => {
                let vals: Vec<RuntimeValue> = elems.iter().map(|e| self.eval_expr(e)).collect();
                RuntimeValue::Array(vals)
            }
            Expr::Index { obj, index } => {
                let obj_val = self.eval_expr(obj);
                let idx_val = self.eval_expr(index);
                match (obj_val, idx_val) {
                    (RuntimeValue::Array(arr), RuntimeValue::Int(i)) => {
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
            Expr::IndexAssign { obj, index, value } => {
                let idx_val = self.eval_expr(index);
                let val_val = self.eval_expr(value);
                let mut arr_val = self.eval_expr(obj);
                if let RuntimeValue::Array(ref mut arr) = arr_val {
                    if let RuntimeValue::Int(i) = idx_val {
                        let idx = i as usize;
                        if idx < arr.len() {
                            arr[idx] = val_val.clone();
                        }
                    }
                }
                if let Expr::Ident(name) = obj.as_ref() {
                    self.variables.insert(name.clone(), arr_val.clone());
                }
                val_val
            }
        }
    }

    pub fn exec_fn_call(&mut self, name: &str, args: &[Expr]) -> RuntimeValue {
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

        match name {
            "print" | "println" => {
                let nl = name == "println";
                for a in &arg_vals {
                    match a {
                        RuntimeValue::Int(n) => print!("{}", n),
                        RuntimeValue::String(s) => print!("{}", s.borrow()),
                        RuntimeValue::Bool(b) => print!("{}", b),
                        RuntimeValue::Array(arr) => {
                            print!("[");
                            for (i, e) in arr.iter().enumerate() {
                                if i > 0 {
                                    print!(", ");
                                }
                                match e {
                                    RuntimeValue::Int(n) => print!("{}", n),
                                    RuntimeValue::String(s) => print!("\"{}\"", s.borrow()),
                                    RuntimeValue::Bool(b) => print!("{}", b),
                                    RuntimeValue::Array(inner) => {
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
                        let av = a.borrow().clone();
                        let bv = b.borrow().clone();
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
                        let bytes = content.borrow().as_bytes().to_vec();
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
                        use std::io::Write;
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path.borrow().as_str())
                        {
                            let bytes: Vec<u8> =
                                content.borrow().bytes().filter(|&b| b != 0).collect();
                            let _ = f.write_all(&bytes);
                        }
                    }
                }
                RuntimeValue::Void
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
                        use std::io::Write;
                        if let Ok(mut f) = std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(path.borrow().as_str())
                        {
                            let _ = f.write_all(&[*byte as u8]);
                        }
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
            "substring" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
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
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let search = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                if let Some(pos) = s.find(&search) {
                    RuntimeValue::Int(pos as i64)
                } else {
                    RuntimeValue::Int(-1)
                }
            }
            "contains" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let search = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::Int(if s.contains(&search) { 1 } else { 0 })
            }
            "toUpperCase" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(s.to_uppercase())))
            }
            "toLowerCase" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(s.to_lowercase())))
            }
            "trim" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(s.trim().to_string())))
            }
            "split" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let delim = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let parts: Vec<RuntimeValue> = if delim.is_empty() {
                    s.chars()
                        .map(|c| RuntimeValue::String(Rc::new(RefCell::new(c.to_string()))))
                        .collect()
                } else {
                    s.split(&delim)
                        .map(|p| RuntimeValue::String(Rc::new(RefCell::new(p.to_string()))))
                        .collect()
                };
                RuntimeValue::Array(parts)
            }
            "replace" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let from = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let to = arg_vals
                    .get(2)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::String(Rc::new(RefCell::new(s.replace(&from, &to))))
            }
            "startsWith" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let prefix = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::Int(if s.starts_with(&prefix) { 1 } else { 0 })
            }
            "endsWith" => {
                let s = arg_vals
                    .first()
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let suffix = arg_vals
                    .get(1)
                    .and_then(|a| {
                        if let RuntimeValue::String(ss) = a {
                            Some(ss.borrow().clone())
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                RuntimeValue::Int(if s.ends_with(&suffix) { 1 } else { 0 })
            }
            _ => {
                if self.class_fields.contains_key(name) && args.is_empty() {
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
                    // Clone current scope and overlay parameters onto it
                    // This lets functions access globals while local params shadow them
                    let mut local_scope = self.variables.clone();
                    for (i, (pname, _)) in params.iter().enumerate() {
                        let val = if i < arg_vals.len() {
                            arg_vals[i].clone()
                        } else {
                            RuntimeValue::Int(0)
                        };
                        local_scope.insert(pname.clone(), val);
                    }
                    let saved = std::mem::replace(&mut self.variables, local_scope);
                    let mut result = RuntimeValue::Void;
                    for s in &body {
                        let r = self.exec_stmt(s);
                        if let RuntimeValue::Return(val) = r {
                            result = *val;
                            break;
                        }
                    }
                    self.variables = saved;
                    return result;
                } else {
                    eprintln!(
                        "[ERROR] Unknown function '{}' called with {} args",
                        name,
                        args.len()
                    );
                }
                RuntimeValue::Void
            }
        }
    }
}

fn is_truthy(val: &RuntimeValue) -> bool {
    match val {
        RuntimeValue::Int(n) => *n != 0,
        RuntimeValue::Bool(b) => *b,
        RuntimeValue::String(s) => !s.borrow().is_empty(),
        RuntimeValue::Array(arr) => !arr.is_empty(),
        RuntimeValue::Return(val) => is_truthy(val),
        _ => true,
    }
}
