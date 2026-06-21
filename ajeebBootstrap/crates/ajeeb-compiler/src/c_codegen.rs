use std::fmt::Write;
use std::collections::HashSet;
use crate::mir::{MirBinOp, MirConst, MirOperand, MirProgram, MirRvalue, MirStmt, Terminator};

pub struct CCodegen {
    output: String,
    var_counter: usize,
    label_counter: usize,
    string_vars: HashSet<String>,
}

impl CCodegen {
    pub fn new() -> Self {
        CCodegen {
            output: String::new(),
            var_counter: 0,
            label_counter: 0,
            string_vars: HashSet::new(),
        }
    }

    fn fresh_var(&mut self) -> String {
        let v = self.var_counter;
        self.var_counter += 1;
        format!("v{}", v)
    }

    fn fresh_label(&mut self) -> String {
        let l = self.label_counter;
        self.label_counter += 1;
        format!("L{}", l)
    }

    pub fn compile(&mut self, prog: &MirProgram) -> Result<String, String> {
        self.var_counter = 0;
        self.label_counter = 0;

        // Header
        writeln!(self.output, "#include <stdio.h>").unwrap();
        writeln!(self.output, "#include <stdlib.h>").unwrap();
        writeln!(self.output, "#include <string.h>").unwrap();
        writeln!(self.output, "#include <stdarg.h>").unwrap();
        writeln!(self.output, "").unwrap();

        // Runtime declarations
        writeln!(self.output, "long long getOutbuf();").unwrap();
        writeln!(self.output, "long long getStateBuf();").unwrap();
        writeln!(self.output, "long long len(long long s);").unwrap();
        writeln!(self.output, "long long arr_len(long long a);").unwrap();
        writeln!(self.output, "long long str_concat(long long a, long long b);").unwrap();
        writeln!(self.output, "long long itoa(long long n);").unwrap();
        writeln!(self.output, "long long strcmp_ajeeb(long long a, long long b);").unwrap();
        writeln!(self.output, "void setInt(long long buf, long long idx, long long val);").unwrap();
        writeln!(self.output, "long long getInt(long long buf, long long idx);").unwrap();
        writeln!(self.output, "void strSet(long long s, long long i, long long c);").unwrap();
        writeln!(self.output, "long long charCode(long long s, long long i);").unwrap();
        writeln!(self.output, "long long chr(long long c);").unwrap();
        writeln!(self.output, "long long substring(long long s, long long start, long long end);").unwrap();
        writeln!(self.output, "long long indexOf(long long s, long long sub);").unwrap();
        writeln!(self.output, "long long contains(long long s, long long sub);").unwrap();
        writeln!(self.output, "long long toUpperCase(long long s);").unwrap();
        writeln!(self.output, "long long toLowerCase(long long s);").unwrap();
        writeln!(self.output, "long long trim(long long s);").unwrap();
        writeln!(self.output, "long long startsWith(long long s, long long prefix);").unwrap();
        writeln!(self.output, "long long endsWith(long long s, long long suffix);").unwrap();
        writeln!(self.output, "long long replace(long long s, long long old, long long new);").unwrap();
        writeln!(self.output, "long long readFile(long long path);").unwrap();
        writeln!(self.output, "long long readArg();").unwrap();
        writeln!(self.output, "void writeFile(long long path, long long content);").unwrap();
        writeln!(self.output, "void writeAppend(long long path, long long content);").unwrap();
        writeln!(self.output, "void writeByte(long long path, long long b);").unwrap();
        writeln!(self.output, "long long array_to_string(long long data, long long len);").unwrap();
        writeln!(self.output, "long long allocBuf(long long size);").unwrap();
        writeln!(self.output, "long long exec(long long cmd);").unwrap();
        writeln!(self.output, "long long mkdir(long long path);").unwrap();
        writeln!(self.output, "long long lib_open(long long path);").unwrap();
        writeln!(self.output, "long long lib_sym(long long lib, long long name);").unwrap();
        writeln!(self.output, "long long tcp_connect(long long host, long long port);").unwrap();
        writeln!(self.output, "void tcp_write(long long conn, long long data);").unwrap();
        writeln!(self.output, "long long tcp_read(long long conn, long long buf);").unwrap();
        writeln!(self.output, "void tcp_close(long long conn);").unwrap();
        writeln!(self.output, "long long tcp_listen(long long port);").unwrap();
        writeln!(self.output, "long long tcp_accept(long long listener);").unwrap();
        writeln!(self.output, "long long dns_lookup(long long host);").unwrap();
        writeln!(self.output, "long long tls_connect(long long host, long long port);").unwrap();
        writeln!(self.output, "void tls_write(long long conn, long long data);").unwrap();
        writeln!(self.output, "long long tls_read(long long conn, long long buf);").unwrap();
        writeln!(self.output, "void tls_close(long long conn);").unwrap();
        writeln!(self.output, "").unwrap();

        // Global buffers expected by C runtime
        writeln!(self.output, "char __ajeeb_buf[16384];").unwrap();
        writeln!(self.output, "char __ajeeb_outbuf[65536];").unwrap();
        writeln!(self.output, "").unwrap();

        // Emit functions
        for f in &prog.functions {
            self.emit_fn(f)?;
        }

        Ok(self.output.clone())
    }

    fn emit_fn(&mut self, f: &crate::mir::MirFn) -> Result<(), String> {
        let params_str: Vec<String> = f.params.iter().map(|(name, _)| format!("long long {}", name)).collect();
        writeln!(self.output, "long long {}({}) {{", f.name, params_str.join(", ")).unwrap();

        // Collect all variable names used in this function
        let param_names: HashSet<String> = f.params.iter().map(|(name, _)| name.clone()).collect();
        let local_names: HashSet<String> = f.locals.iter().map(|(name, _)| name.clone()).collect();
        let mut used_vars = HashSet::new();

        // Also track v-temps generated during emission
        let saved_var_counter = self.var_counter;
        self.var_counter = 0;

        // Scan to find all used vars and count v-temps
        for block in &f.blocks {
            for stmt in &block.statements {
                match stmt {
                    MirStmt::Assign { dest, value } => {
                        used_vars.insert(dest.clone());
                        self.collect_rvalue_vars(value, &mut used_vars);
                        // Count temps this would generate
                        self.count_rvalue_temps(value);
                    }
                    MirStmt::Call { dest, func: _, args } => {
                        if let Some(d) = dest {
                            used_vars.insert(d.clone());
                        }
                        for arg in args {
                            self.collect_operand_vars(arg, &mut used_vars);
                        }
                    }
                }
            }
            match &block.terminator {
                Terminator::SwitchInt { cond, .. } => {
                    self.collect_operand_vars(cond, &mut used_vars);
                }
                Terminator::Return(Some(op)) => {
                    self.collect_operand_vars(op, &mut used_vars);
                }
                _ => {}
            }
        }
        let max_v_temp = self.var_counter;
        self.var_counter = saved_var_counter;

        // Declare locals (skip params)
        for (name, _) in &f.locals {
            if !param_names.contains(name) {
                writeln!(self.output, "  long long {};", name).unwrap();
            }
        }

        // Declare any used vars that aren't params or locals (temporaries like t0, t1)
        for var in &used_vars {
            if !param_names.contains(var) && !local_names.contains(var) && var.starts_with('t') {
                writeln!(self.output, "  long long {};", var).unwrap();
            }
        }

        // Declare v-temps
        for i in 0..max_v_temp {
            writeln!(self.output, "  long long v{};", i).unwrap();
        }
        writeln!(self.output, "").unwrap();

        // Emit blocks
        for block in &f.blocks {
            if block.id > 0 {
                writeln!(self.output, "  L{}:", block.id).unwrap();
            }
            for stmt in &block.statements {
                self.emit_stmt(stmt);
            }
            self.emit_terminator(&block.terminator);
        }

        writeln!(self.output, "}}").unwrap();
        writeln!(self.output, "").unwrap();
        Ok(())
    }

    fn count_rvalue_temps(&mut self, rvalue: &MirRvalue) {
        match rvalue {
            MirRvalue::BinaryOp(_, _, _) => {
                self.fresh_var(); // Each binary op uses one temp
            }
            _ => {}
        }
    }

    fn collect_operand_vars(&self, op: &MirOperand, vars: &mut HashSet<String>) {
        if let MirOperand::Var(name) = op {
            vars.insert(name.clone());
        }
    }

    fn collect_rvalue_vars(&self, rvalue: &MirRvalue, vars: &mut HashSet<String>) {
        match rvalue {
            MirRvalue::Use(op) => self.collect_operand_vars(op, vars),
                    MirRvalue::BinaryOp(_, ref left, ref right) => {
                        self.collect_operand_vars(left, vars);
                        self.collect_operand_vars(right, vars);
                    }
            MirRvalue::Const(_) => {}
        }
    }

    fn emit_stmt(&mut self, stmt: &MirStmt) {
        match stmt {
            MirStmt::Assign { dest, value } => {
                // Track string variables
                match value {
                    MirRvalue::Const(MirConst::Str(_)) => {
                        self.string_vars.insert(dest.clone());
                    }
                    MirRvalue::Use(MirOperand::Var(ref name)) => {
                        if self.string_vars.contains(name) {
                            self.string_vars.insert(dest.clone());
                        }
                    }
                    MirRvalue::BinaryOp(MirBinOp::Add, ref left, ref right) => {
                        // str_concat result is a string
                        let left_is_str = match left {
                            MirOperand::Constant(MirConst::Str(_)) => true,
                            MirOperand::Var(n) => self.string_vars.contains(n),
                            _ => false,
                        };
                        let right_is_str = match right {
                            MirOperand::Constant(MirConst::Str(_)) => true,
                            MirOperand::Var(n) => self.string_vars.contains(n),
                            _ => false,
                        };
                        if left_is_str || right_is_str {
                            self.string_vars.insert(dest.clone());
                        }
                    }
                    MirRvalue::BinaryOp(_, ref left, ref right) => {
                        // If either operand is a string, result is not a string (comparison etc)
                    }
                    _ => {}
                }
                let val = self.emit_rvalue(value);
                writeln!(self.output, "  {} = {};", dest, val).unwrap();
            }
            MirStmt::Call { dest, func, args } => {
                let args_str: Vec<String> = args.iter().map(|a| self.emit_operand(a)).collect();
                match func.as_str() {
                    "println" => {
                        if args.is_empty() {
                            writeln!(self.output, "  printf(\"\\n\");").unwrap();
                        } else if args.len() == 1 {
                            let a = &args_str[0];
                            writeln!(self.output, "  puts((char*){});", a).unwrap();
                        } else {
                            // Multi-arg: concatenate then print
                            let mut concat = args_str[0].clone();
                            for a in &args_str[1..] {
                                let tmp = self.fresh_var();
                                writeln!(self.output, "  {} = str_concat({}, {});", tmp, concat, a).unwrap();
                                concat = tmp;
                            }
                            writeln!(self.output, "  puts((char*){});", concat).unwrap();
                        }
                    }
                    "print" => {
                        if args.len() == 1 {
                            let a = &args_str[0];
                            writeln!(self.output, "  printf(\"%s\", (char*){});", a).unwrap();
                        }
                    }
                    "assert_eq" => {
                        if args.len() == 2 {
                            let l = &args_str[0];
                            let r = &args_str[1];
                            let tmp = self.fresh_var();
                            writeln!(self.output, "  {} = ({} == {});", tmp, l, r).unwrap();
                            writeln!(self.output, "  if (!{}) {{ fprintf(stderr, \"assert_eq failed\\n\"); exit(1); }}", tmp).unwrap();
                        }
                    }
                    "setInt" => {
                        if args.len() == 3 {
                            writeln!(self.output, "  setInt({}, {}, {});", args_str[0], args_str[1], args_str[2]).unwrap();
                        }
                    }
                    "strSet" => {
                        if args.len() == 3 {
                            writeln!(self.output, "  strSet({}, {}, {});", args_str[0], args_str[1], args_str[2]).unwrap();
                        }
                    }
                    "writeFile" => {
                        if args.len() == 2 {
                            writeln!(self.output, "  writeFile({}, {});", args_str[0], args_str[1]).unwrap();
                        }
                    }
                    "writeAppend" => {
                        if args.len() == 2 {
                            writeln!(self.output, "  writeAppend({}, {});", args_str[0], args_str[1]).unwrap();
                        }
                    }
                    "writeByte" => {
                        if args.len() == 2 {
                            writeln!(self.output, "  writeByte({}, {});", args_str[0], args_str[1]).unwrap();
                        }
                    }
                    _ => {
                        // Generic function call
                        if let Some(ref dest_name) = dest {
                            writeln!(self.output, "  {} = {}({});", dest_name, func, args_str.join(", ")).unwrap();
                        } else {
                            writeln!(self.output, "  {}({});", func, args_str.join(", ")).unwrap();
                        }
                    }
                }
            }
        }
    }

    fn emit_rvalue(&mut self, rvalue: &MirRvalue) -> String {
        match rvalue {
            MirRvalue::Use(op) => self.emit_operand(op),
            MirRvalue::Const(c) => self.emit_const(c),
            MirRvalue::BinaryOp(op, left, right) => {
                let l = self.emit_operand(left);
                let r = self.emit_operand(right);
                // Check if this is a string concatenation
                if *op == MirBinOp::Add {
                    let left_is_str = match left {
                        MirOperand::Constant(MirConst::Str(_)) => true,
                        MirOperand::Var(n) => self.string_vars.contains(n),
                        _ => false,
                    };
                    let right_is_str = match right {
                        MirOperand::Constant(MirConst::Str(_)) => true,
                        MirOperand::Var(n) => self.string_vars.contains(n),
                        _ => false,
                    };
                    if left_is_str || right_is_str {
                        let tmp = self.fresh_var();
                        writeln!(self.output, "  {} = str_concat({}, {});", tmp, l, r).unwrap();
                        return tmp;
                    }
                }
                let tmp = self.fresh_var();
                match op {
                    MirBinOp::Add => writeln!(self.output, "  {} = {} + {};", tmp, l, r).unwrap(),
                    MirBinOp::Sub => writeln!(self.output, "  {} = {} - {};", tmp, l, r).unwrap(),
                    MirBinOp::Mul => writeln!(self.output, "  {} = {} * {};", tmp, l, r).unwrap(),
                    MirBinOp::Div => {
                        writeln!(self.output, "  {} = ({} == 0) ? 0 : ({} / {});", tmp, r, l, r).unwrap();
                    }
                    MirBinOp::Eq => writeln!(self.output, "  {} = ({} == {});", tmp, l, r).unwrap(),
                    MirBinOp::Neq => writeln!(self.output, "  {} = ({} != {});", tmp, l, r).unwrap(),
                    MirBinOp::Lt => writeln!(self.output, "  {} = ({} < {});", tmp, l, r).unwrap(),
                    MirBinOp::Gt => writeln!(self.output, "  {} = ({} > {});", tmp, l, r).unwrap(),
                    MirBinOp::Le => writeln!(self.output, "  {} = ({} <= {});", tmp, l, r).unwrap(),
                    MirBinOp::Ge => writeln!(self.output, "  {} = ({} >= {});", tmp, l, r).unwrap(),
                    MirBinOp::And => writeln!(self.output, "  {} = {} && {};", tmp, l, r).unwrap(),
                    MirBinOp::Or => writeln!(self.output, "  {} = {} || {};", tmp, l, r).unwrap(),
                }
                tmp
            }
        }
    }

    fn emit_operand(&mut self, op: &MirOperand) -> String {
        match op {
            MirOperand::Var(name) => name.clone(),
            MirOperand::Constant(c) => self.emit_const(c),
        }
    }

    fn emit_const(&mut self, c: &MirConst) -> String {
        match c {
            MirConst::Int(n) => format!("{}", n),
            MirConst::Float(f) => {
                let bits = f.to_bits() as i64;
                format!("*((long long*)&({}))", bits)
            }
            MirConst::Str(s) => {
                let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
                format!("((long long)(char*)\"{}\")", escaped)
            }
            MirConst::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        }
    }

    fn emit_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Goto(target) => {
                writeln!(self.output, "  goto L{};", target).unwrap();
            }
            Terminator::SwitchInt { cond, targets, default } => {
                let c = self.emit_operand(cond);
                if targets.is_empty() {
                    writeln!(self.output, "  if ({} != 0) goto L{};", c, default).unwrap();
                } else {
                    let (val, target) = &targets[0];
                    writeln!(self.output, "  if ({} == {}) goto L{}; else goto L{};", c, val, target, default).unwrap();
                }
            }
            Terminator::Return(Some(op)) => {
                let val = self.emit_operand(op);
                writeln!(self.output, "  return {};", val).unwrap();
            }
            Terminator::Return(None) => {
                writeln!(self.output, "  return 0;").unwrap();
            }
            Terminator::Unreachable => {
                writeln!(self.output, "  /* unreachable */").unwrap();
            }
        }
    }
}
