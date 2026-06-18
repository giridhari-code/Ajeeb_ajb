pub mod expr;
pub mod stmt;
pub mod functions;
pub mod builtins;
pub mod traits;
pub mod modules;

use crate::ast::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;

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

    fn print_stack_trace(&self) {
        eprintln!("Stack trace (most recent call first):");
        for (i, frame) in self.call_stack.iter().rev().enumerate() {
            eprintln!("  {}: {} (line {}, col {})", i, frame.function_name, frame.line, frame.col);
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
