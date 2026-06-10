use crate::ast::*;
use crate::error::CompileError;

pub struct CCodeGen {
    pub output_buffer: String,
}

impl CCodeGen {
    pub fn new() -> Self {
        CCodeGen { output_buffer: String::new() }
    }

    pub fn generate_c_source(&mut self, stmts: &[Stmt]) -> Result<String, CompileError> {
        self.output_buffer.push_str("// ==========================================\n");
        self.output_buffer.push_str("//  Ajeeb Self-Hosted C Transpiler Engine   \n");
        self.output_buffer.push_str("// ==========================================\n\n");
        self.output_buffer.push_str("#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n#include <stdbool.h>\n#include <stdint.h>\n\n");

        self.output_buffer.push_str("char __ajeeb_buf[16384];\nchar __ajeeb_outbuf[65536];\n\n");

        for stmt in stmts {
            self.emit_c_statement(stmt)?;
        }
        Ok(self.output_buffer.clone())
    }

    fn emit_c_statement(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::FnDef { name, params, body, return_type } => {
                if name == "main" {
                    self.output_buffer.push_str("int main(int argc, char** argv) {\n");
                } else {
                    let rtype = type_annot_to_c(return_type);
                    self.output_buffer.push_str(&format!("{} fn_{}(", rtype, name));
                    let mut first = true;
                    for (pname, ptype) in params {
                        if !first { self.output_buffer.push_str(", "); }
                        first = false;
                        self.output_buffer.push_str(&format!("{} {}", type_annot_to_c(ptype), pname));
                    }
                    self.output_buffer.push_str(") {\n");
                }
                for s in body {
                    self.emit_c_statement(s)?;
                }
                if return_type != &TypeAnnot::Void && !body.iter().any(|s| matches!(s, Stmt::Return { .. })) {
                    self.output_buffer.push_str("    return 0;\n");
                }
                self.output_buffer.push_str("}\n\n");
            }
            Stmt::Let { name, value, type_ann } => {
                match value {
                    Expr::New { class_name } => {
                        // Allocate class instance on stack as struct
                        let ctype = type_ann.as_ref()
                            .map(type_annot_to_c)
                            .unwrap_or_else(|| format!("{}", class_name));
                        self.output_buffer.push_str(&format!("    {} {} = {{0}};\n", ctype, name));
                    }
                    _ => {
                        let ctype = type_ann.as_ref()
                            .map(type_annot_to_c)
                            .unwrap_or_else(|| "int".to_string());
                        self.output_buffer.push_str(&format!("    {} {} = {};\n", ctype, name, self.emit_c_expr(value)));
                    }
                }
            }
            Stmt::Const { name, value, type_ann } => {
                let ctype = type_ann.as_ref()
                    .map(|t| format!("const {}", type_annot_to_c(t)))
                    .unwrap_or_else(|| "const int".to_string());
                self.output_buffer.push_str(&format!("    {} {} = {};\n", ctype, name, self.emit_c_expr(value)));
            }
            Stmt::If { condition, then_block, else_block } => {
                self.output_buffer.push_str(&format!("    if ({}) {{\n", self.emit_c_expr(condition)));
                for s in then_block {
                    self.emit_c_statement(s)?;
                }
                if let Some(eblock) = else_block {
                    self.output_buffer.push_str("    } else {\n");
                    for s in eblock {
                        self.emit_c_statement(s)?;
                    }
                }
                self.output_buffer.push_str("    }\n");
            }
            Stmt::While { condition, body } => {
                self.output_buffer.push_str(&format!("    while ({}) {{\n", self.emit_c_expr(condition)));
                for s in body {
                    self.emit_c_statement(s)?;
                }
                self.output_buffer.push_str("    }\n");
            }
            Stmt::Return { value } => {
                if let Some(expr) = value {
                    self.output_buffer.push_str(&format!("    return {};\n", self.emit_c_expr(expr)));
                } else {
                    self.output_buffer.push_str("    return;\n");
                }
            }
            Stmt::Expr(expr) => {
                self.output_buffer.push_str(&format!("    {};\n", self.emit_c_expr(expr)));
            }
            Stmt::Class { name, fields, methods } => {
                self.output_buffer.push_str(&format!("typedef struct {{\n"));
                for f in fields {
                    let ctype = type_annot_to_c(&f.type_ann);
                    self.output_buffer.push_str(&format!("    {} {};\n", ctype, f.name));
                }
                self.output_buffer.push_str(&format!("}} {};\n\n", name));
                for m in methods {
                    if let Stmt::FnDef { name: mname, params, body, return_type } = m {
                        let rtype = type_annot_to_c(return_type);
                        self.output_buffer.push_str(&format!("{} {}_{}({}* self", rtype, name, mname, name));
                        for (pname, ptype) in params {
                            self.output_buffer.push_str(&format!(", {} {}", type_annot_to_c(ptype), pname));
                        }
                        self.output_buffer.push_str(") {\n");
                        for s in body {
                            self.emit_c_statement(s)?;
                        }
                        if return_type != &TypeAnnot::Void && !body.iter().any(|s| matches!(s, Stmt::Return { .. })) {
                            self.output_buffer.push_str("    return 0;\n");
                        }
                        self.output_buffer.push_str("}\n\n");
                    }
                }
            }
        }
        Ok(())
    }

    fn emit_c_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => n.to_string(),
            Expr::StringLit(s) => {
                let mut escaped = String::with_capacity(s.len() + 2);
                escaped.push('"');
                for c in s.chars() {
                    match c {
                        '\n' => escaped.push_str("\\n"),
                        '\t' => escaped.push_str("\\t"),
                        '\\' => escaped.push_str("\\\\"),
                        '"' => escaped.push_str("\\\""),
                        _ => escaped.push(c),
                    }
                }
                escaped.push('"');
                format!("(intptr_t){}", escaped)
            }
            Expr::Bool(b) => (if *b { "1" } else { "0" }).to_string(),
            Expr::Ident(id) => id.clone(),
            Expr::Binary { left, op, right } => {
                let op_str = match op {
                    BinOp::Add => " + ",
                    BinOp::Sub => " - ",
                    BinOp::Mul => " * ",
                    BinOp::Div => " / ",
                    BinOp::Eq => " == ",
                    BinOp::Neq => " != ",
                    BinOp::Lt => " < ",
                    BinOp::Gt => " > ",
                    BinOp::Le => " <= ",
                    BinOp::Ge => " >= ",
                    BinOp::And => " && ",
                    BinOp::Or => " || ",
                };
                format!("({}{}{})", self.emit_c_expr(left), op_str, self.emit_c_expr(right))
            }
            Expr::Assign { name, value } => {
                format!("({} = {})", name, self.emit_c_expr(value))
            }
            Expr::FnCall { name, args } => {
                match name.as_str() {
                    "print" | "println" => {
                        let term = if *name == "println" { "\\n" } else { "" };
                        if let Some(first) = args.first() {
                            match first {
                                Expr::StringLit(s) => {
                                    let mut esc = String::new();
                                    for c in s.chars() {
                                        match c {
                                            '\n' => esc.push_str("\\n"),
                                            '\t' => esc.push_str("\\t"),
                                            '\\' => esc.push_str("\\\\"),
                                            '"' => esc.push_str("\\\""),
                                            _ => esc.push(c),
                                        }
                                    }
                                    format!("printf(\"{}{}\")", esc, term)
                                }
                                _ => format!("printf(\"%d{}\", (int){})", term, self.emit_c_expr(first)),
                            }
                        } else {
                            "printf(\"\\n\")".to_string()
                        }
                    }
                    "len" if args.len() == 1 => {
                        format!("(intptr_t)strlen((char*){})", self.emit_c_expr(&args[0]))
                    }
                    "charCode" if args.len() == 2 => {
                        format!("(unsigned char)((char*){})[{}]", self.emit_c_expr(&args[0]), self.emit_c_expr(&args[1]))
                    }
                    "readFile" if args.len() == 1 => format!("(intptr_t)__ajeeb_buf"),
                    "writeFile" if args.len() == 2 => "(intptr_t)0".to_string(),
                    "itoa" if args.len() == 1 => "(intptr_t)__ajeeb_buf".to_string(),
                    "strcmp" if args.len() == 2 => {
                        format!("strcmp((char*){}, (char*){})", self.emit_c_expr(&args[0]), self.emit_c_expr(&args[1]))
                    }
                    "strcpy" if args.len() == 2 => {
                        format!("(intptr_t)strcpy((char*){}, (char*){})", self.emit_c_expr(&args[0]), self.emit_c_expr(&args[1]))
                    }
                    "writeAppend" if args.len() == 2 => "(intptr_t)0".to_string(),
                    "strSet" if args.len() == 3 => {
                        format!("((char*){})[{}] = (char)({})",
                            self.emit_c_expr(&args[0]), self.emit_c_expr(&args[1]),
                            self.emit_c_expr(&args[2]))
                    }
                    "getInt" if args.len() == 2 => "(intptr_t)0".to_string(),
                    "setInt" if args.len() == 3 => "(intptr_t)0".to_string(),
                    "getOutbuf" => "(intptr_t)__ajeeb_outbuf".to_string(),
                    "readArg" if args.len() == 1 => {
                        format!("(intptr_t)argv[({}) + 1]", self.emit_c_expr(&args[0]))
                    }
                    _ => {
                        let parsed_args: Vec<String> = args.iter().map(|a| self.emit_c_expr(a)).collect();
                        if name.contains('_') && !parsed_args.is_empty() {
                            let us = name.find('_').unwrap();
                            let cn = &name[..us];
                            let mn = &name[us+1..];
                            let rest: Vec<&str> = parsed_args[1..].iter().map(|s| s.as_str()).collect();
                            if rest.is_empty() {
                                format!("{}_{}(&{})", cn, mn, parsed_args[0])
                            } else {
                                format!("{}_{}(&{}, {})", cn, mn, parsed_args[0], rest.join(", "))
                            }
                        } else {
                            format!("fn_{}({})", name, parsed_args.join(", "))
                        }
                    }
                }
            }
            Expr::New { class_name } => {
                format!("({}){{0}}", class_name)
            }
            Expr::ArrayLit(elems) => {
                let parsed: Vec<String> = elems.iter().map(|e| self.emit_c_expr(e)).collect();
                format!("{{ {} }}", parsed.join(", "))
            }
            Expr::Index { obj, index } => {
                format!("({})[{}]", self.emit_c_expr(obj), self.emit_c_expr(index))
            }
            Expr::IndexAssign { obj, index, value } => {
                format!("(({})[{}] = ({}))", self.emit_c_expr(obj), self.emit_c_expr(index), self.emit_c_expr(value))
            }
            Expr::Field { obj, field, class_name: _ } => {
                let obj_s = self.emit_c_expr(obj);
                if obj_s == "self" {
                    format!("self->{}", field)
                } else {
                    format!("({}).{}", obj_s, field)
                }
            }
            Expr::FieldAssign { obj, field, value, class_name: _ } => {
                let obj_s = self.emit_c_expr(obj);
                if obj_s == "self" {
                    format!("(self->{} = ({}))", field, self.emit_c_expr(value))
                } else {
                    format!("(({}).{} = ({}))", obj_s, field, self.emit_c_expr(value))
                }
            }
            Expr::Group(inner) => format!("({})", self.emit_c_expr(inner)),
        }
    }
}

fn type_annot_to_c(t: &TypeAnnot) -> String {
    match t {
        // Ajeeb uses a unified type: int holds both integers and pointers
        TypeAnnot::Int => "intptr_t".to_string(),
        TypeAnnot::String => "intptr_t".to_string(),
        TypeAnnot::Bool => "intptr_t".to_string(),
        TypeAnnot::Void => "void".to_string(),
        TypeAnnot::Array(_) => "intptr_t".to_string(),
        // Class types use struct name for stack allocation in let/const
        TypeAnnot::Class(name) => name.clone(),
    }
}
