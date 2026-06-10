use std::collections::HashMap;
use crate::ast::*;
use crate::error::CompileError;
use crate::token::Token;

struct FnInfo {
    label: String,
}

#[allow(dead_code)]
struct ClassLayout {
    _fields: Vec<String>,
    field_offsets: HashMap<String, i32>,
    _size: i32,
}

pub struct AsmGen {
    asm: String,
    data: String,
    label_counter: usize,
    var_map: HashMap<String, i32>,
    fn_map: HashMap<String, FnInfo>,
    class_map: HashMap<String, ClassLayout>,
    _field_access_tmp: HashMap<String, String>,
    current_offset: i32,
    class_field_scope: Option<String>,
}

impl AsmGen {
    pub fn new() -> Self {
        AsmGen {
            asm: String::new(),
            data: String::new(),
            label_counter: 0,
            var_map: HashMap::new(),
            fn_map: HashMap::new(),
            class_map: HashMap::new(),
            _field_access_tmp: HashMap::new(),
            current_offset: 0,
            class_field_scope: None,
        }
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!(".L{}_{}", prefix, id)
    }

    fn get_var_offset(&self, name: &str) -> i32 {
        *self.var_map.get(name).unwrap_or_else(|| panic!("Variable '{}' declare nahi hui!", name))
    }

    fn alloc_var(&mut self, name: &str) -> i32 {
        self.current_offset -= 8;
        self.var_map.insert(name.to_string(), self.current_offset);
        self.current_offset
    }

    fn alloc_var_sized(&mut self, name: &str, slots: usize) -> i32 {
        for _ in 0..slots { self.current_offset -= 8; }
        let base = self.current_offset + 8;
        self.var_map.insert(name.to_string(), self.current_offset + 8 * slots as i32);
        base
    }

    pub fn generate(&mut self, stmts: &[Stmt]) -> Result<String, CompileError> {
        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, .. } => {
                    let label = format!("fn_{}", name);
                    self.fn_map.insert(name.clone(), FnInfo { label });
                }
                Stmt::Class { name, fields, methods } => {
                    let mut offsets = HashMap::new();
                    let mut fnames = Vec::new();
                    let mut off: i32 = 0;
                    for f in fields {
                        offsets.insert(f.name.clone(), off);
                        fnames.push(f.name.clone());
                        off += 8;
                    }
                    self.class_map.insert(name.clone(), ClassLayout {
                        _fields: fnames,
                        field_offsets: offsets,
                        _size: off,
                    });
                    for m in methods {
                        if let Stmt::FnDef { name: mname, .. } = m {
                            let label = format!("fn_{}_{}", name, mname);
                            self.fn_map.insert(format!("{}_{}", name, mname), FnInfo { label });
                        }
                    }
                }
                _ => {}
            }
        }

        self.asm.push_str(".global _start\n");
        self.asm.push_str(".text\n");

        let has_main = stmts.iter().any(|s| matches!(s, Stmt::FnDef { name, .. } if name == "main"));

        if has_main {
            self.asm.push_str("_start:\n");
            self.asm.push_str("    bl fn_main\n");
            self.asm.push_str("    mov x8, #93\n");
            self.asm.push_str("    svc #0\n");
        } else {
            self.asm.push_str("_start:\n");
            self.asm.push_str("    mov x29, sp\n");
        }

        for stmt in stmts {
            self.emit_stmt(stmt)?;
        }

        if !has_main {
            self.asm.push_str("    mov x0, #0\n");
            self.asm.push_str("    mov x8, #93\n");
            self.asm.push_str("    svc #0\n");
        }

        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, body, .. } => {
                    self.emit_fn_def(&name, &params, &body)?;
                }
                Stmt::Class { name, methods, .. } => {
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, body, return_type: _ } = m {
                            let mut all_params = vec![("self".to_string(), TypeAnnot::Int)];
                            all_params.extend(params.iter().cloned());
                            self.class_field_scope = Some(name.clone());
                            let mangled_name = format!("{}_{}", name, mname);
                            self.emit_fn_def(&mangled_name, &all_params, &body)?;
                            self.class_field_scope = None;
                        }
                    }
                }
                _ => {}
            }
        }

        self.emit_data_section();
        let bss = ".section .bss\n.align 4\n__ajeeb_buf: .space 16384\n__ajeeb_itoa_buf: .space 32\n";
        Ok(format!("{}\n{}{}", self.asm, self.data, bss))
    }

    fn emit_data_section(&mut self) {
        if !self.data.is_empty() {
            self.data = format!(".section .rodata\n{}", self.data);
        }
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let { name, value, .. } | Stmt::Const { name, value, .. } => {
                match value {
                    Expr::ArrayLit(elems) => {
                        let base = self.alloc_var_sized(name, elems.len() + 1);
                        let ptr_slot = self.get_var_offset(name);
                        for (i, elem) in elems.iter().enumerate() {
                            let elem_addr = base + (i as i32 * 8);
                            self.emit_expr(elem)?;
                            self.asm.push_str(&format!("    str x0, [x29, {}]\n", elem_addr));
                        }
                        self.asm.push_str(&format!("    add x0, x29, #{}\n", base));
                        self.asm.push_str(&format!("    str x0, [x29, {}]\n", ptr_slot));
                    }
                    _ => {
                        self.alloc_var(name);
                        self.emit_expr(value)?;
                        let offset = self.get_var_offset(name);
                        self.asm.push_str(&format!("    str x0, [x29, {}]\n", offset));
                    }
                }
                Ok(())
            }
            Stmt::If { condition, then_block, else_block } => {
                let else_label = self.fresh_label("else");
                let end_label = self.fresh_label("endif");
                self.emit_condition(condition, &else_label)?;
                for s in then_block { self.emit_stmt(s)?; }
                self.asm.push_str(&format!("    b {}\n", end_label));
                self.asm.push_str(&format!("{}:\n", else_label));
                if let Some(eblock) = else_block {
                    for s in eblock { self.emit_stmt(s)?; }
                }
                self.asm.push_str(&format!("{}:\n", end_label));
                Ok(())
            }
            Stmt::While { condition, body } => {
                let begin_label = self.fresh_label("while_begin");
                let end_label = self.fresh_label("while_end");
                self.asm.push_str(&format!("{}:\n", begin_label));
                self.emit_expr(condition)?;
                self.asm.push_str("    cmp x0, #0\n");
                self.asm.push_str(&format!("    b.eq {}\n", end_label));
                for s in body { self.emit_stmt(s)?; }
                self.asm.push_str(&format!("    b {}\n", begin_label));
                self.asm.push_str(&format!("{}:\n", end_label));
                Ok(())
            }
            Stmt::Return { value } => {
                if let Some(expr) = value {
                    self.emit_expr(expr)?;
                }
                self.emit_fn_epilogue();
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.emit_expr(expr)?;
                Ok(())
            }
            Stmt::FnDef { .. } => Ok(()),
            Stmt::Class { .. } => Ok(()),
        }
    }

    fn emit_condition(&mut self, condition: &Expr, false_label: &str) -> Result<(), CompileError> {
        self.emit_expr(condition)?;
        self.asm.push_str("    cmp x0, #0\n");
        self.asm.push_str(&format!("    b.eq {}\n", false_label));
        Ok(())
    }

    fn emit_print(&mut self, expr: &Expr) -> Result<(), CompileError> {
        if let Expr::StringLit(s) = expr {
            let lbl = self.fresh_label("str");
            let len = s.len();
            self.data.push_str(&format!("{}: .asciz \"", lbl));
            for c in s.chars() {
                match c {
                    '\n' => self.data.push_str("\\n"),
                    '\t' => self.data.push_str("\\t"),
                    '\\' => self.data.push_str("\\\\"),
                    '"' => self.data.push_str("\"\""),
                    _ => self.data.push(c),
                }
            }
            self.data.push_str("\"\n");
            self.asm.push_str("    mov x0, #1\n");
            self.asm.push_str(&format!("    adrp x1, {}\n", lbl));
            self.asm.push_str(&format!("    add x1, x1, :lo12:{}\n", lbl));
            self.asm.push_str(&format!("    mov x2, #{}\n", len));
            self.asm.push_str("    mov x8, #64\n");
            self.asm.push_str("    svc #0\n");
        } else {
            self.emit_expr(expr)?;
            self.asm.push_str("    mov x1, x0\n");
            self.asm.push_str("    mov x0, #1\n");
            self.asm.push_str("    mov x2, #8\n");
            self.asm.push_str("    mov x8, #64\n");
            self.asm.push_str("    svc #0\n");
        }
        Ok(())
    }

    fn emit_println(&mut self, expr: &Expr) -> Result<(), CompileError> {
        self.emit_print(expr)?;
        let nl_lbl = self.fresh_label("nl");
        self.data.push_str(&format!("{}: .asciz \"\\n\"\n", nl_lbl));
        self.asm.push_str("    mov x0, #1\n");
        self.asm.push_str(&format!("    adrp x1, {}\n", nl_lbl));
        self.asm.push_str(&format!("    add x1, x1, :lo12:{}\n", nl_lbl));
        self.asm.push_str("    mov x2, #1\n");
        self.asm.push_str("    mov x8, #64\n");
        self.asm.push_str("    svc #0\n");
        Ok(())
    }

    fn emit_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr {
            Expr::Number(n) => {
                self.asm.push_str(&format!("    mov x0, #{}\n", n));
            }
            Expr::StringLit(s) => {
                let lbl = self.fresh_label("str");
                self.data.push_str(&format!("{}: .asciz \"", lbl));
                for c in s.chars() {
                    match c {
                        '\n' => self.data.push_str("\\n"),
                        '\t' => self.data.push_str("\\t"),
                        '\\' => self.data.push_str("\\\\"),
                        '"' => self.data.push_str("\"\""),
                        _ => self.data.push(c),
                    }
                }
                self.data.push_str("\"\n");
                self.asm.push_str(&format!("    adrp x0, {}\n", lbl));
                self.asm.push_str(&format!("    add x0, x0, :lo12:{}\n", lbl));
            }
            Expr::Bool(b) => {
                self.asm.push_str(&format!("    mov x0, #{}\n", if *b { 1 } else { 0 }));
            }
            Expr::Ident(name) => {
                let offset = self.get_var_offset(name);
                self.asm.push_str(&format!("    ldr x0, [x29, {}]\n", offset));
            }
            Expr::Binary { left, op, right } => {
                self.emit_expr(left)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n");
                self.emit_expr(right)?;
                self.asm.push_str("    mov x1, x0\n");
                self.asm.push_str("    ldr x0, [sp], #16\n");
                match op {
                    BinOp::Add => self.asm.push_str("    add x0, x0, x1\n"),
                    BinOp::Sub => self.asm.push_str("    sub x0, x0, x1\n"),
                    BinOp::Mul => self.asm.push_str("    mul x0, x0, x1\n"),
                    BinOp::Div => self.asm.push_str("    sdiv x0, x0, x1\n"),
                    BinOp::Eq => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, eq\n");
                    }
                    BinOp::Neq => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, ne\n");
                    }
                    BinOp::Lt => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, lt\n");
                    }
                    BinOp::Gt => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, gt\n");
                    }
                    BinOp::Le => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, le\n");
                    }
                    BinOp::Ge => {
                        self.asm.push_str("    cmp x0, x1\n");
                        self.asm.push_str("    cset x0, ge\n");
                    }
                }
            }
            Expr::Assign { name, value } => {
                self.emit_expr(value)?;
                let offset = self.get_var_offset(name);
                self.asm.push_str(&format!("    str x0, [x29, {}]\n", offset));
            }
            Expr::IndexAssign { obj, index, value } => {
                self.emit_expr(obj)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n");
                self.emit_expr(index)?;
                self.asm.push_str("    mov x1, x0\n");
                self.asm.push_str("    ldr x0, [sp], #16\n");
                self.asm.push_str("    add x0, x0, x1, lsl #3\n");
                self.asm.push_str("    str x0, [sp, #-16]!\n");
                self.emit_expr(value)?;
                self.asm.push_str("    ldr x1, [sp], #16\n");
                self.asm.push_str("    str x0, [x1]\n");
            }
            Expr::FnCall { name, args } => {
                if name == "print" {
                    if args.len() == 1 {
                        return self.emit_print(&args[0]);
                    }
                }
                if name == "println" {
                    if args.len() == 1 {
                        return self.emit_println(&args[0]);
                    }
                }
                if name == "len" && args.len() == 1 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x1, x0\n");
                    self.asm.push_str("    mov x0, #0\n");
                    let sl_lbl = self.fresh_label("strlen_loop");
                    self.asm.push_str(&format!("{}:\n", sl_lbl));
                    let selbl = self.fresh_label("strlen_end");
                    self.asm.push_str("    ldrb w2, [x1, x0]\n");
                    self.asm.push_str(&format!("    cbz w2, {}\n", selbl));
                    self.asm.push_str("    add x0, x0, #1\n");
                    self.asm.push_str(&format!("    b {}\n", sl_lbl));
                    self.asm.push_str(&format!("{}:\n", selbl));
                    return Ok(());
                }
                if name == "charCode" && args.len() == 2 {
                    self.emit_expr(&args[1])?;
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    ldr x1, [sp], #16\n");
                    self.asm.push_str("    add x0, x0, x1\n");
                    self.asm.push_str("    ldrb w0, [x0]\n");
                    return Ok(());
                }
                if name == "readFile" && args.len() == 1 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x1, x0\n");
                    self.asm.push_str("    mov x0, #-100\n");
                    self.asm.push_str("    mov x2, #0\n");
                    self.asm.push_str("    mov x8, #56\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x19, x0\n");
                    self.asm.push_str("    adrp x1, __ajeeb_buf\n");
                    self.asm.push_str("    add x1, x1, :lo12:__ajeeb_buf\n");
                    self.asm.push_str("    mov x2, #16384\n");
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x8, #63\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x20, x0\n");
                    self.asm.push_str("    adrp x1, __ajeeb_buf\n");
                    self.asm.push_str("    add x1, x1, :lo12:__ajeeb_buf\n");
                    self.asm.push_str("    strb wzr, [x1, x20]\n");
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x8, #57\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    adrp x0, __ajeeb_buf\n");
                    self.asm.push_str("    add x0, x0, :lo12:__ajeeb_buf\n");
                    return Ok(());
                }
                if name == "writeFile" && args.len() == 2 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x19, x0\n");
                    self.emit_expr(&args[1])?;
                    self.asm.push_str("    mov x20, x0\n");
                    self.asm.push_str("    mov x2, #0\n");
                    let wlbl = self.fresh_label("wstrlen");
                    let welbl = self.fresh_label("wstrlen_end");
                    self.asm.push_str(&format!("{}:\n", wlbl));
                    self.asm.push_str("    ldrb w3, [x20, x2]\n");
                    self.asm.push_str(&format!("    cbz w3, {}\n", welbl));
                    self.asm.push_str("    add x2, x2, #1\n");
                    self.asm.push_str(&format!("    b {}\n", wlbl));
                    self.asm.push_str(&format!("{}:\n", welbl));
                    self.asm.push_str("    mov x21, x2\n");
                    self.asm.push_str("    mov x1, x19\n");
                    self.asm.push_str("    mov x0, #-100\n");
                    self.asm.push_str("    mov x2, #577\n");
                    self.asm.push_str("    mov x3, #420\n");
                    self.asm.push_str("    mov x8, #56\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x19, x0\n");
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x1, x20\n");
                    self.asm.push_str("    mov x2, x21\n");
                    self.asm.push_str("    mov x8, #64\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x0, x19\n");
                    self.asm.push_str("    mov x8, #57\n");
                    self.asm.push_str("    svc #0\n");
                    self.asm.push_str("    mov x0, #0\n");
                    return Ok(());
                }
                if name == "itoa" && args.len() == 1 {
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    adrp x1, __ajeeb_itoa_buf\n");
                    self.asm.push_str("    add x1, x1, :lo12:__ajeeb_itoa_buf\n");
                    self.asm.push_str("    add x2, x1, #31\n");
                    self.asm.push_str("    mov w3, #0\n");
                    self.asm.push_str("    strb w3, [x2]\n");
                    let iz_lbl = self.fresh_label("itoa_zero");
                    let ip_lbl = self.fresh_label("itoa_pos");
                    let il_lbl = self.fresh_label("itoa_loop");
                    let id_lbl = self.fresh_label("itoa_done");
                    let ie_lbl = self.fresh_label("itoa_end");
                    self.asm.push_str(&format!("    cmp x0, #0\n"));
                    self.asm.push_str(&format!("    b.ne {}\n", iz_lbl));
                    self.asm.push_str("    mov w3, #'0'\n");
                    self.asm.push_str("    sub x2, x2, #1\n");
                    self.asm.push_str("    strb w3, [x2]\n");
                    self.asm.push_str(&format!("    mov x0, x2\n"));
                    self.asm.push_str(&format!("    b {}\n", ie_lbl));
                    self.asm.push_str(&format!("{}:\n", iz_lbl));
                    self.asm.push_str("    mov x4, #0\n");
                    self.asm.push_str(&format!("    cmp x0, #0\n"));
                    self.asm.push_str(&format!("    b.ge {}\n", ip_lbl));
                    self.asm.push_str("    mov x4, #1\n");
                    self.asm.push_str("    neg x0, x0\n");
                    self.asm.push_str(&format!("{}:\n", ip_lbl));
                    self.asm.push_str("    mov x7, #10\n");
                    self.asm.push_str(&format!("{}:\n", il_lbl));
                    self.asm.push_str("    udiv x5, x0, x7\n");
                    self.asm.push_str("    msub x6, x5, x7, x0\n");
                    self.asm.push_str("    add x6, x6, #'0'\n");
                    self.asm.push_str("    sub x2, x2, #1\n");
                    self.asm.push_str("    strb w6, [x2]\n");
                    self.asm.push_str("    mov x0, x5\n");
                    self.asm.push_str(&format!("    cbnz x0, {}\n", il_lbl));
                    self.asm.push_str(&format!("{}:\n", id_lbl));
                    self.asm.push_str("    cmp x4, #1\n");
                    self.asm.push_str(&format!("    b.ne {}\n", ie_lbl));
                    self.asm.push_str("    sub x2, x2, #1\n");
                    self.asm.push_str("    mov w6, #'-'\n");
                    self.asm.push_str("    strb w6, [x2]\n");
                    self.asm.push_str(&format!("{}:\n", ie_lbl));
                    self.asm.push_str("    mov x0, x2\n");
                    return Ok(());
                }
                if name == "strcmp" && args.len() == 2 {
                    self.emit_expr(&args[1])?;
                    self.asm.push_str("    mov x1, x0\n");
                    self.emit_expr(&args[0])?;
                    let scl_lbl = self.fresh_label("strcmp_loop");
                    let scd_lbl = self.fresh_label("strcmp_diff");
                    self.asm.push_str(&format!("{}:\n", scl_lbl));
                    self.asm.push_str("    ldrb w2, [x0]\n");
                    self.asm.push_str("    ldrb w3, [x1]\n");
                    self.asm.push_str("    cmp w2, w3\n");
                    self.asm.push_str(&format!("    b.ne {}\n", scd_lbl));
                    let seq_lbl = self.fresh_label("strcmp_eq");
                    self.asm.push_str(&format!("    cbz w2, {}\n", seq_lbl));
                    self.asm.push_str("    add x0, x0, #1\n");
                    self.asm.push_str("    add x1, x1, #1\n");
                    self.asm.push_str(&format!("    b {}\n", scl_lbl));
                    self.asm.push_str(&format!("{}:\n", seq_lbl));
                    self.asm.push_str("    mov x0, #0\n");
                    let scx_lbl = self.fresh_label("strcmp_exit");
                    self.asm.push_str(&format!("    b {}\n", scx_lbl));
                    self.asm.push_str(&format!("{}:\n", scd_lbl));
                    self.asm.push_str("    sub x0, x2, x3\n");
                    self.asm.push_str(&format!("{}:\n", scx_lbl));
                    return Ok(());
                }
                if name == "strcpy" && args.len() == 2 {
                    self.emit_expr(&args[1])?;
                    self.asm.push_str("    mov x1, x0\n");
                    self.emit_expr(&args[0])?;
                    self.asm.push_str("    mov x2, x0\n");
                    let syp_lbl = self.fresh_label("strcpy_loop");
                    self.asm.push_str(&format!("{}:\n", syp_lbl));
                    self.asm.push_str("    ldrb w3, [x1]\n");
                    self.asm.push_str("    strb w3, [x2]\n");
                    let sye_lbl = self.fresh_label("strcpy_done");
                    self.asm.push_str(&format!("    cbz w3, {}\n", sye_lbl));
                    self.asm.push_str("    add x1, x1, #1\n");
                    self.asm.push_str("    add x2, x2, #1\n");
                    self.asm.push_str(&format!("    b {}\n", syp_lbl));
                    self.asm.push_str(&format!("{}:\n", sye_lbl));
                    return Ok(());
                }
                let label = {
                    let info = self.fn_map.get(name)
                        .ok_or_else(|| CompileError::new(0, 0, format!("Function '{}' define nahi hui!", name)))?;
                    info.label.clone()
                };
                if args.len() > 8 {
                    return Err(CompileError::new(0, 0, "Zyaada arguments! Sirf 8 arguments allowed hain.".to_string()));
                }
                for arg in args.iter() {
                    self.emit_expr(arg)?;
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                }
                let arg_regs = ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"];
                for (i, _) in args.iter().enumerate().rev() {
                    self.asm.push_str(&format!("    ldr {}, [sp], #16\n", arg_regs[i]));
                }
                self.asm.push_str(&format!("    bl {}\n", label));
            }
            Expr::ArrayLit(elems) => {
                self.asm.push_str("    mov x0, sp\n");
                for elem in elems.iter() {
                    self.emit_expr(elem)?;
                    self.asm.push_str("    str x0, [sp, #-16]!\n");
                }
                self.asm.push_str("    mov x0, sp\n");
            }
            Expr::Index { obj, index } => {
                self.emit_expr(obj)?;
                self.asm.push_str("    str x0, [sp, #-16]!\n");
                self.emit_expr(index)?;
                self.asm.push_str("    mov x1, x0\n");
                self.asm.push_str("    ldr x0, [sp], #16\n");
                self.asm.push_str("    add x0, x0, x1, lsl #3\n");
                self.asm.push_str("    ldr x0, [x0]\n");
            }
            Expr::Field { obj, field } => {
                let class_name = self.class_field_scope.as_ref().cloned();
                if let Some(ref cn) = class_name {
                    if let Some(off) = self.class_map.get(cn)
                        .and_then(|layout| layout.field_offsets.get(field))
                        .cloned()
                    {
                        self.emit_expr(obj)?;
                        self.asm.push_str(&format!("    ldr x0, [x0, #{}]\n", off));
                    } else {
                        return Err(CompileError::new(0, 0, format!("Field '{}' class '{}' me exist nahi karta!", field, cn)));
                    }
                } else {
                    return Err(CompileError::new(0, 0, "Field access sirf class methods me kaam karta hai.".to_string()));
                }
            }
            Expr::Group(inner) => self.emit_expr(inner)?,
        }
        Ok(())
    }

    fn emit_fn_def(&mut self, name: &str, params: &[(String, TypeAnnot)], body: &[Stmt]) -> Result<(), CompileError> {
        let info = self.fn_map.get(name).unwrap();
        self.asm.push_str(&format!("{}:\n", info.label));
        self.asm.push_str("    stp x29, x30, [sp, #-16]!\n");
        self.asm.push_str("    stp x19, x20, [sp, #-16]!\n");
        self.asm.push_str("    str x21, [sp, #-16]!\n");
        self.asm.push_str("    mov x29, sp\n");

        let old_var_map = self.var_map.clone();
        let old_offset = self.current_offset;
        self.var_map.clear();
        self.current_offset = 0;

        let param_regs = ["x0", "x1", "x2", "x3", "x4", "x5", "x6", "x7"];
        for (i, (pname, _)) in params.iter().enumerate() {
            let offset = self.alloc_var(pname);
            self.asm.push_str(&format!("    str {}, [x29, {}]\n", param_regs[i], offset));
        }

        let mut local_var_count = 0;
        for stmt in body {
            self.count_local_vars(stmt, &mut local_var_count);
        }
        let total_stack = params.len() + local_var_count;
        if total_stack > 0 {
            let size = ((total_stack * 8) + 15) & !15;
            self.asm.push_str(&format!("    sub sp, sp, #{}\n", size));
        }

        for stmt in body {
            self.emit_stmt(stmt)?;
        }

        let has_return = body.iter().any(|s| matches!(s, Stmt::Return { .. }));
        if !has_return {
            self.emit_fn_epilogue();
        }

        self.var_map = old_var_map;
        self.current_offset = old_offset;

        Ok(())
    }

    fn count_local_vars(&mut self, stmt: &Stmt, count: &mut usize) {
        match stmt {
            Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
                if let Expr::ArrayLit(elems) = value {
                    *count += elems.len() + 1;
                } else {
                    *count += 1;
                }
            }
            Stmt::If { then_block, else_block, .. } => {
                for s in then_block { self.count_local_vars(s, count); }
                if let Some(eb) = else_block {
                    for s in eb { self.count_local_vars(s, count); }
                }
            }
            Stmt::While { body, .. } => {
                for s in body { self.count_local_vars(s, count); }
            }
            _ => {}
        }
    }

    fn emit_fn_epilogue(&mut self) {
        self.asm.push_str("    mov sp, x29\n");
        self.asm.push_str("    ldr x21, [sp], #16\n");
        self.asm.push_str("    ldp x19, x20, [sp], #16\n");
        self.asm.push_str("    ldp x29, x30, [sp], #16\n");
        self.asm.push_str("    ret\n");
    }
}
