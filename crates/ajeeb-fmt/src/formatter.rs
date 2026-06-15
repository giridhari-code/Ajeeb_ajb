use std::fmt::Write;
use ajeeb_compiler::ast::*;
use crate::comment::{Comment, extract_comments};
use crate::config::FormatConfig;

pub struct Formatter {
    config: FormatConfig,
    output: String,
    indent_level: usize,
    comments: Vec<Comment>,
    comment_idx: usize,

}

impl Formatter {
    pub fn new(config: FormatConfig, source: &str) -> Self {
        let comments = extract_comments(source);
        Formatter {
            config,
            output: String::with_capacity(source.len() * 12 / 10),
            indent_level: 0,
            comments,
            comment_idx: 0,
        }
    }

    pub fn format(mut self, stmts: &[Stmt]) -> String {
        for stmt in stmts {
            self.emit_leading_comments_for_stmt(stmt);
            self.format_stmt(stmt);
        }
        self.emit_remaining_comments();
        self.output
    }

    fn indent_str(&self) -> String {
        if self.config.use_tabs {
            "\t".repeat(self.indent_level)
        } else {
            " ".repeat(self.indent_level * self.config.indent_size)
        }
    }

    fn emit_indent(&mut self) {
        write!(self.output, "{}", self.indent_str()).unwrap();
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_line(&mut self, s: &str) {
        self.emit_indent();
        self.output.push_str(s);
        self.nl();
    }

    fn nl(&mut self) {
        self.output.push('\n');
    }

    fn space(&mut self) {
        self.output.push(' ');
    }

    fn emit_comma_space(&mut self) {
        self.output.push_str(", ");
    }

    fn open_brace(&mut self) {
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
    }

    fn close_brace(&mut self) {
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("}");
    }

    fn open_paren(&mut self) {
        self.emit("(");
    }

    fn close_paren(&mut self) {
        self.emit(")");
    }

    fn emit_leading_comments_for_stmt(&mut self, stmt: &Stmt) {
        let (line, col) = stmt_pos(stmt);
        loop {
            if self.comment_idx >= self.comments.len() { break; }
            let c = &self.comments[self.comment_idx];
            if c.is_line_before(line, col) || (c.line == line && c.col < col) {
                let cc = self.comments[self.comment_idx].clone();
                self.comment_idx += 1;
                if cc.is_block {
                    self.emit_comment_block(&cc);
                } else {
                    self.emit_comment_line(&cc);
                }
            } else {
                break;
            }
        }
    }

    fn emit_leading_comments_for_expr(&mut self, expr: &Expr) {
        let (line, col) = expr_pos(expr);
        loop {
            if self.comment_idx >= self.comments.len() { break; }
            let c = &self.comments[self.comment_idx];
            if c.is_line_before(line, col) || (c.line == line && c.col < col) {
                let cc = self.comments[self.comment_idx].clone();
                self.comment_idx += 1;
                if cc.is_block {
                    self.emit_comment_block(&cc);
                } else {
                    self.emit_comment_line(&cc);
                }
            } else {
                break;
            }
        }
    }

    fn emit_comment_line(&mut self, c: &Comment) {
        self.emit_indent();
        self.emit("// ");
        self.emit(&c.text);
        self.nl();
    }

    fn emit_comment_block(&mut self, c: &Comment) {
        self.emit_indent();
        self.emit("/*");
        if c.text.contains('\n') {
            self.nl();
            for line in c.text.lines() {
                self.emit_indent();
                self.emit(" ");
                self.emit(line);
                self.nl();
            }
            self.emit_indent();
            self.emit("*/");
        } else {
            self.emit(" ");
            self.emit(&c.text);
            self.emit(" */");
        }
        self.nl();
    }

    fn emit_remaining_comments(&mut self) {
        while self.comment_idx < self.comments.len() {
            let c = self.comments[self.comment_idx].clone();
            self.comment_idx += 1;
            if c.is_block {
                self.emit_comment_block(&c);
            } else {
                self.emit_comment_line(&c);
            }
        }
    }

    pub fn format_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import(import) => self.format_import(import),
            Stmt::FnDef { name, type_params, params, return_type, body, pub_, .. } => {
                self.format_fn_def(name, type_params, params, return_type, body, *pub_)
            }
            Stmt::Class { name, fields, methods, pub_, .. } => {
                self.format_class(name, fields, methods, *pub_)
            }
            Stmt::StructDef { name, type_params, type_param_bounds, fields, pub_, .. } => {
                self.format_struct_def(name, type_params, type_param_bounds, fields, *pub_)
            }
            Stmt::EnumDef { name, type_params, type_param_bounds, variants, pub_, .. } => {
                self.format_enum_def(name, type_params, type_param_bounds, variants, *pub_)
            }
            Stmt::Let { name, type_ann, value, .. } => {
                self.format_let("let", name, type_ann, value)
            }
            Stmt::Const { name, type_ann, value, .. } => {
                self.format_let("const", name, type_ann, value)
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                self.format_if(condition, then_block, else_block)
            }
            Stmt::While { condition, body, .. } => {
                self.format_while(condition, body)
            }
            Stmt::ForLoop { init, condition, update, body, .. } => {
                self.format_for(init, condition, update, body)
            }
            Stmt::Return { value, .. } => {
                self.format_return(value)
            }
            Stmt::Break { .. } => {
                self.emit_line("break;")
            }
            Stmt::Continue { .. } => {
                self.emit_line("continue;")
            }
            Stmt::Expr(expr, ..) => {
                self.emit_leading_comments_for_expr(expr);
                self.emit_indent();
                self.format_expr(expr);
                self.emit(";");
                self.nl();
            }
            Stmt::TraitDef { name, type_params, type_param_bounds, methods, pub_, .. } => {
                self.format_trait_def(name, type_params, type_param_bounds, methods, *pub_)
            }
            Stmt::ImplBlock { trait_name, trait_type_args, type_params, type_param_bounds, type_name, methods, .. } => {
                self.format_impl_block(trait_name.as_deref(), trait_type_args, type_params, type_param_bounds, type_name, methods)
            }
        }
    }

    // ── Imports ─────────────────────────────────────────────────────

    fn format_import(&mut self, import: &ImportDecl) {
        self.emit_indent();
        self.emit("import ");
        let path_str = import.path.join("::");
        self.emit(&path_str);
        if let Some(alias) = &import.alias {
            self.emit(" as ");
            self.emit(alias);
        }
        self.emit(";");
        self.nl();
    }

    // ── Function Definitions ───────────────────────────────────────

    fn format_fn_def(&mut self, name: &str, type_params: &[String], params: &[(String, TypeAnnot)], return_type: &TypeAnnot, body: &[Stmt], pub_: bool) {
        if pub_ {
            self.emit_indent();
            self.emit("pub ");
        }
        self.emit_indent();
        self.emit("function ");
        self.emit(name);
        if !type_params.is_empty() {
            self.emit("[");
            for (i, tp) in type_params.iter().enumerate() {
                if i > 0 { self.emit_comma_space(); }
                self.emit(tp);
            }
            self.emit("]");
        }
        self.open_paren();
        for (i, (pname, ptype)) in params.iter().enumerate() {
            if i > 0 { self.emit_comma_space(); }
            self.emit(pname);
            self.emit(": ");
            self.format_type(ptype);
        }
        self.close_paren();
        if !matches!(return_type, TypeAnnot::Void) {
            self.emit(": ");
            self.format_type(return_type);
        }
        self.open_brace();
        for stmt in body {
            self.format_stmt(stmt);
        }
        self.close_brace();
        self.nl();
        self.nl();
    }

    // ── Classes ────────────────────────────────────────────────────

    fn format_class(&mut self, name: &str, fields: &[ClassField], methods: &[Stmt], pub_: bool) {
        if pub_ {
            self.emit_indent();
            self.emit("pub ");
        }
        self.emit_indent();
        self.emit("class ");
        self.emit(name);
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
        for field in fields {
            self.emit_indent();
            self.emit(&field.name);
            self.emit(": ");
            self.format_type(&field.type_ann);
            self.emit(";");
            self.nl();
        }
        if !fields.is_empty() && !methods.is_empty() {
            self.nl();
        }
        for method in methods {
            self.format_stmt(method);
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("}");
        self.nl();
        self.nl();
    }

    // ── Structs ────────────────────────────────────────────────────

    fn format_struct_def(&mut self, name: &str, type_params: &[String], type_param_bounds: &[(String, Vec<String>)], fields: &[StructField], pub_: bool) {
        if pub_ {
            self.emit_indent();
            self.emit("pub ");
        }
        self.emit_indent();
        self.emit("struct ");
        self.emit(name);
        if !type_params.is_empty() {
            self.emit("[");
            for (i, tp) in type_params.iter().enumerate() {
                if i > 0 { self.emit_comma_space(); }
                self.emit(tp);
                if let Some((_, bounds)) = type_param_bounds.iter().find(|(n, _)| n == tp) {
                    if !bounds.is_empty() {
                        self.emit(": ");
                        for (j, b) in bounds.iter().enumerate() {
                            if j > 0 { self.emit(" + "); }
                            self.emit(b);
                        }
                    }
                }
            }
            self.emit("]");
        }
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
        for field in fields {
            self.emit_indent();
            self.emit(&field.name);
            self.emit(": ");
            self.format_type(&field.type_ann);
            self.emit(";");
            self.nl();
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("}");
        self.nl();
        self.nl();
    }

    // ── Enums ──────────────────────────────────────────────────────

    fn format_enum_def(&mut self, name: &str, type_params: &[String], type_param_bounds: &[(String, Vec<String>)], variants: &[EnumVariantDef], pub_: bool) {
        if pub_ {
            self.emit_indent();
            self.emit("pub ");
        }
        self.emit_indent();
        self.emit("enum ");
        self.emit(name);
        if !type_params.is_empty() {
            self.emit("[");
            for (i, tp) in type_params.iter().enumerate() {
                if i > 0 { self.emit_comma_space(); }
                self.emit(tp);
                if let Some((_, bounds)) = type_param_bounds.iter().find(|(n, _)| n == tp) {
                    if !bounds.is_empty() {
                        self.emit(": ");
                        for (j, b) in bounds.iter().enumerate() {
                            if j > 0 { self.emit(" + "); }
                            self.emit(b);
                        }
                    }
                }
            }
            self.emit("]");
        }
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
        for (i, variant) in variants.iter().enumerate() {
            if i > 0 { self.emit(","); self.nl(); }
            self.emit_indent();
            self.emit(&variant.name);
            if !variant.fields.is_empty() {
                self.emit("(");
                for (j, ft) in variant.fields.iter().enumerate() {
                    if j > 0 { self.emit_comma_space(); }
                    self.format_type(ft);
                }
                self.emit(")");
            }
        }
        self.nl();
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("};");
        self.nl();
        self.nl();
    }

    // ── Traits ────────────────────────────────────────────────────

    fn format_trait_def(&mut self, name: &str, type_params: &[String], type_param_bounds: &[(String, Vec<String>)], methods: &[TraitMethod], pub_: bool) {
        if pub_ {
            self.emit_indent();
            self.emit("pub ");
        }
        self.emit_indent();
        self.emit("trait ");
        self.emit(name);
        if !type_params.is_empty() {
            self.emit("[");
            for (i, tp) in type_params.iter().enumerate() {
                if i > 0 { self.emit(", "); }
                self.emit(tp);
                if let Some((_, bounds)) = type_param_bounds.iter().find(|(n, _)| n == tp) {
                    if !bounds.is_empty() {
                        self.emit(": ");
                        for (j, b) in bounds.iter().enumerate() {
                            if j > 0 { self.emit(" + "); }
                            self.emit(b);
                        }
                    }
                }
            }
            self.emit("]");
        }
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
        for method in methods {
            self.emit_indent();
            self.emit("function ");
            self.emit(&method.name);
            self.emit("(");
            for (i, (pname, ptype)) in method.params.iter().enumerate() {
                if i > 0 { self.emit_comma_space(); }
                self.emit(pname);
                self.emit(": ");
                self.format_type(ptype);
            }
            self.emit(")");
            if !matches!(method.return_type, TypeAnnot::Void) {
                self.emit(": ");
                self.format_type(&method.return_type);
            }
            self.emit(";");
            self.nl();
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("}");
        self.nl();
        self.nl();
    }

    fn format_impl_block(&mut self, trait_name: Option<&str>, trait_type_args: &[String], type_params: &[String], type_param_bounds: &[(String, Vec<String>)], type_name: &str, methods: &[Stmt]) {
        self.emit_indent();
        self.emit("impl");
        if !type_params.is_empty() {
            self.emit("[");
            for (i, tp) in type_params.iter().enumerate() {
                if i > 0 { self.emit(", "); }
                self.emit(tp);
                // Check if this param has bounds
                if let Some((_, bounds)) = type_param_bounds.iter().find(|(n, _)| n == tp) {
                    if !bounds.is_empty() {
                        self.emit(": ");
                        for (j, b) in bounds.iter().enumerate() {
                            if j > 0 { self.emit(" + "); }
                            self.emit(b);
                        }
                    }
                }
            }
            self.emit("]");
        }
        self.space();
        if let Some(trait_name) = trait_name {
            self.emit(trait_name);
            if !trait_type_args.is_empty() {
                self.emit("[");
                for (i, arg) in trait_type_args.iter().enumerate() {
                    if i > 0 { self.emit(", "); }
                    self.emit(arg);
                }
                self.emit("]");
            }
            self.emit(" for ");
        }
        self.emit(type_name);
        self.space();
        self.emit("{");
        self.nl();
        self.indent_level += 1;
        for method in methods {
            self.format_stmt(method);
        }
        self.indent_level -= 1;
        self.emit_indent();
        self.emit("}");
        self.nl();
        self.nl();
    }

    // ── Let / Const ────────────────────────────────────────────────

    fn format_let(&mut self, kind: &str, name: &str, type_ann: &Option<TypeAnnot>, value: &Expr) {
        self.emit_indent();
        self.emit(kind);
        self.space();
        self.emit(name);
        if let Some(ty) = type_ann {
            self.emit(": ");
            self.format_type(ty);
        }
        self.space();
        self.emit("= ");
        self.format_expr(value);
        self.emit(";");
        self.nl();
    }

    // ── If ─────────────────────────────────────────────────────────

    fn format_if(&mut self, condition: &Expr, then_block: &[Stmt], else_block: &Option<Vec<Stmt>>) {
        self.emit_indent();
        self.emit("if (");
        self.format_expr(condition);
        self.emit(")");
        self.open_brace();
        for s in then_block {
            self.format_stmt(s);
        }
        self.close_brace();
        if let Some(else_stmts) = else_block {
            self.space();
            self.emit("else");
            // Check if the else block starts with an if (else if)
            if else_stmts.len() == 1 {
                if let Stmt::If { .. } = &else_stmts[0] {
                    self.space();
                    self.format_stmt(&else_stmts[0]);
                    return;
                }
            }
            self.open_brace();
            for s in else_stmts {
                self.format_stmt(s);
            }
            self.close_brace();
        }
        self.nl();
    }

    // ── While ──────────────────────────────────────────────────────

    fn format_while(&mut self, condition: &Expr, body: &[Stmt]) {
        self.emit_indent();
        self.emit("while (");
        self.format_expr(condition);
        self.emit(")");
        self.open_brace();
        for s in body {
            self.format_stmt(s);
        }
        self.close_brace();
        self.nl();
    }

    // ── For ────────────────────────────────────────────────────────

    fn format_for(&mut self, init: &Stmt, condition: &Expr, update: &Stmt, body: &[Stmt]) {
        self.emit_indent();
        self.emit("for (");
        // Init
        if let Stmt::Let { name, type_ann, value, .. } = init {
            self.emit("let ");
            self.emit(name);
            if let Some(ty) = type_ann {
                self.emit(": ");
                self.format_type(ty);
            }
            self.emit(" = ");
            self.format_expr(value);
        }
        self.emit("; ");
        // Condition
        self.format_expr(condition);
        self.emit("; ");
        // Update
        if let Stmt::Expr(expr, ..) = update {
            self.format_expr(expr);
        }
        self.emit(")");
        self.open_brace();
        for s in body {
            self.format_stmt(s);
        }
        self.close_brace();
        self.nl();
    }

    // ── Return ─────────────────────────────────────────────────────

    fn format_return(&mut self, value: &Option<Expr>) {
        self.emit_indent();
        self.emit("return");
        if let Some(expr) = value {
            self.space();
            self.format_expr(expr);
        }
        self.emit(";");
        self.nl();
    }

    // ── Expressions ────────────────────────────────────────────────

    fn format_expr(&mut self, expr: &Expr) {
        self.emit_leading_comments_for_expr(expr);
        match expr {
            Expr::Number(n, ..) => { write!(self.output, "{}", n).unwrap(); }
            Expr::FloatLit(f, ..) => { write!(self.output, "{}", f).unwrap(); }
            Expr::StringLit(s, ..) => self.format_string_lit(s),
            Expr::Bool(b, ..) => { self.emit(if *b { "true" } else { "false" }); }
            Expr::Ident(name, ..) => { self.emit(name); }
            Expr::Binary { left, op, right, .. } => self.format_binary(left, op, right),
            Expr::Assign { name, value, .. } => {
                self.emit(name);
                self.space();
                self.emit("= ");
                self.format_expr(value);
            }
            Expr::IndexAssign { obj, index, value, .. } => {
                self.format_expr(obj);
                self.emit("[");
                self.format_expr(index);
                self.emit("] = ");
                self.format_expr(value);
            }
            Expr::FnCall { name, args, .. } => {
                self.emit(name);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
            Expr::MethodCall { obj, method, args, .. } => {
                self.format_expr(obj);
                self.emit(".");
                self.emit(method);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
            Expr::New { class_name, .. } => {
                self.emit("new ");
                self.emit(class_name);
            }
            Expr::ArrayLit(elems, ..) => {
                self.emit("[");
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(elem);
                }
                self.emit("]");
            }
            Expr::Index { obj, index, .. } => {
                self.format_expr(obj);
                self.emit("[");
                self.format_expr(index);
                self.emit("]");
            }
            Expr::Field { obj, field, .. } => {
                self.format_expr(obj);
                self.emit(".");
                self.emit(field);
            }
            Expr::FieldAssign { obj, field, value, .. } => {
                self.format_expr(obj);
                self.emit(".");
                self.emit(field);
                self.space();
                self.emit("= ");
                self.format_expr(value);
            }
            Expr::UnaryMinus(inner, ..) => {
                self.emit("-");
                self.format_expr(inner);
            }
            Expr::UnaryNot(inner, ..) => {
                self.emit("!");
                self.format_expr(inner);
            }
            Expr::Group(inner, ..) => {
                self.emit("(");
                self.format_expr(inner);
                self.emit(")");
            }
            Expr::StructLit { struct_name, fields, .. } => {
                self.emit(struct_name);
                self.space();
                self.emit("{");
                self.space();
                for (i, (fname, fexpr)) in fields.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.emit(fname);
                    self.emit(": ");
                    self.format_expr(fexpr);
                }
                self.space();
                self.emit("}");
            }
            Expr::EnumRef { enum_name, variant, .. } => {
                self.emit(enum_name);
                self.emit("::");
                self.emit(variant);
            }
            Expr::EnumCtor { enum_name, variant, args, .. } => {
                self.emit(enum_name);
                self.emit("::");
                self.emit(variant);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
            Expr::Match { value, arms, .. } => {
                self.emit("match ");
                self.format_expr(value);
                self.space();
                self.emit("{");
                self.nl();
                self.indent_level += 1;
                for arm in arms {
                    self.emit_indent();
                    self.format_pattern(&arm.pattern);
                    self.space();
                    self.emit("=> ");
                    if let Some(body_block) = &arm.body_block {
                        self.emit("{");
                        self.nl();
                        self.indent_level += 1;
                        for s in body_block {
                            self.format_stmt(s);
                        }
                        self.indent_level -= 1;
                        self.emit_indent();
                        self.emit("}");
                    } else {
                        self.format_expr(&arm.body);
                    }
                    self.emit(",");
                    self.nl();
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}");
            }
            Expr::GenericCall { name, type_args, args, .. } => {
                self.emit(name);
                self.emit("[");
                for (i, ta) in type_args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_type(ta);
                }
                self.emit("](");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
            Expr::AssociatedFnCall { type_name, method, args, .. } => {
                self.emit(type_name);
                self.emit("::");
                self.emit(method);
                self.emit("(");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_expr(arg);
                }
                self.emit(")");
            }
        }
    }

    fn format_binary(&mut self, left: &Expr, op: &BinOp, right: &Expr) {
        self.format_expr(left);
        self.space();
        self.emit(match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "==",
            BinOp::Neq => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        });
        self.space();
        self.format_expr(right);
    }

    fn format_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Wildcard => self.emit("_"),
            Pattern::Int(n) => { write!(self.output, "{}", n).unwrap(); }
            Pattern::String(s) => self.format_string_lit_internal(s),
            Pattern::EnumVariant { enum_name, variant, bindings } => {
                self.emit(enum_name);
                self.emit("::");
                self.emit(variant);
                if !bindings.is_empty() {
                    self.emit("(");
                    for (i, b) in bindings.iter().enumerate() {
                        if i > 0 { self.emit_comma_space(); }
                        self.emit(b);
                    }
                    self.emit(")");
                }
            }
        }
    }

    fn format_type(&mut self, ty: &TypeAnnot) {
        match ty {
            TypeAnnot::Int => self.emit("Int"),
            TypeAnnot::Float => self.emit("Float"),
            TypeAnnot::String => self.emit("String"),
            TypeAnnot::Bool => self.emit("Bool"),
            TypeAnnot::Void => self.emit("Void"),
            TypeAnnot::Array(inner) => {
                self.format_type(inner);
                self.emit("[]");
            }
            TypeAnnot::Class(name) => self.emit(name),
            TypeAnnot::Generic(name) => self.emit(name),
            TypeAnnot::Parameterized { base, args } => {
                self.format_type(base);
                self.emit("[");
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { self.emit_comma_space(); }
                    self.format_type(arg);
                }
                self.emit("]");
            }
        }
    }

    fn format_string_lit(&mut self, s: &str) {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\0', "\\0");
        write!(self.output, "\"{}\"", escaped).unwrap();
    }

    fn format_string_lit_internal(&mut self, s: &str) {
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\t', "\\t")
            .replace('\0', "\\0");
        write!(self.output, "\"{}\"", escaped).unwrap();
    }
}

fn stmt_pos(stmt: &Stmt) -> (usize, usize) {
    match stmt {
        Stmt::Let { line, col, .. } => (*line, *col),
        Stmt::Const { line, col, .. } => (*line, *col),
        Stmt::If { line, col, .. } => (*line, *col),
        Stmt::While { line, col, .. } => (*line, *col),
        Stmt::ForLoop { line, col, .. } => (*line, *col),
        Stmt::Break { line, col, .. } => (*line, *col),
        Stmt::Continue { line, col, .. } => (*line, *col),
        Stmt::Return { line, col, .. } => (*line, *col),
        Stmt::Expr(_, line, col) => (*line, *col),
        Stmt::FnDef { line, col, .. } => (*line, *col),
        Stmt::Class { line, col, .. } => (*line, *col),
        Stmt::Import(import) => (import.line, import.col),
        Stmt::StructDef { line, col, .. } => (*line, *col),
        Stmt::EnumDef { line, col, .. } => (*line, *col),
        Stmt::TraitDef { line, col, .. } => (*line, *col),
        Stmt::ImplBlock { line, col, .. } => (*line, *col),
    }
}

fn expr_pos(expr: &Expr) -> (usize, usize) {
    match expr {
        Expr::Number(_, line, col) => (*line, *col),
        Expr::FloatLit(_, line, col) => (*line, *col),
        Expr::StringLit(_, line, col) => (*line, *col),
        Expr::Bool(_, line, col) => (*line, *col),
        Expr::Ident(_, line, col) => (*line, *col),
        Expr::Binary { line, col, .. } => (*line, *col),
        Expr::Assign { line, col, .. } => (*line, *col),
        Expr::IndexAssign { line, col, .. } => (*line, *col),
        Expr::FnCall { line, col, .. } => (*line, *col),
        Expr::MethodCall { line, col, .. } => (*line, *col),
        Expr::New { line, col, .. } => (*line, *col),
        Expr::ArrayLit(_, line, col) => (*line, *col),
        Expr::Index { line, col, .. } => (*line, *col),
        Expr::Field { line, col, .. } => (*line, *col),
        Expr::FieldAssign { line, col, .. } => (*line, *col),
        Expr::UnaryMinus(_, line, col) => (*line, *col),
        Expr::UnaryNot(_, line, col) => (*line, *col),
        Expr::Group(_, line, col) => (*line, *col),
        Expr::StructLit { line, col, .. } => (*line, *col),
        Expr::EnumRef { line, col, .. } => (*line, *col),
        Expr::EnumCtor { line, col, .. } => (*line, *col),
        Expr::AssociatedFnCall { line, col, .. } => (*line, *col),
        Expr::Match { line, col, .. } => (*line, *col),
        Expr::GenericCall { line, col, .. } => (*line, *col),
    }
}

pub fn format_source(config: &FormatConfig, source: &str) -> Result<String, String> {
    let tokens = {
        let mut lexer = ajeeb_compiler::lexer::Lexer::new(source);
        let mut tokens = Vec::new();
        let mut lines = Vec::new();
        let mut cols = Vec::new();
        loop {
            match lexer.next_token_spanned() {
                Ok((ajeeb_compiler::token::Token::Eof, _, _)) => break,
                Ok((tok, line, col)) => {
                    tokens.push(tok);
                    lines.push(line);
                    cols.push(col);
                }
                Err(e) => return Err(format!("Lex error: {}", e)),
            }
        }
        (tokens, lines, cols)
    };

    let (tokens, token_lines, token_cols) = tokens;
    let mut parser = ajeeb_compiler::parser::Parser::with_positions(tokens, token_lines, token_cols);
    let stmts = match parser.parse_program() {
        Ok(stmts) => stmts,
        Err(e) => return Err(format!("Parse error: {}", e)),
    };

    let formatter = Formatter::new(config.clone(), source);
    Ok(formatter.format(&stmts))
}

pub fn format_file(config: &FormatConfig, path: &str) -> Result<String, String> {
    let source = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path, e))?;
    format_source(config, &source)
}
