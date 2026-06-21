use crate::hir::*;
use crate::mir::*;
use std::collections::HashMap;

pub struct MirBuilder {
    current_blocks: Vec<BasicBlock>,
    current_stmts: Vec<MirStmt>,
    temp_counter: usize,
    loop_stack: Vec<(usize, usize)>, // (continue_target, break_target) — 0 = placeholder
    break_patches: Vec<usize>,       // block indices whose Goto target should become exit_block
    continue_patches: Vec<usize>,    // block indices whose Goto target should become update/header_block
    method_mangled_names: HashMap<String, Vec<String>>, // type_method -> [mangled1, mangled2, ...]
    closure_counter: usize,
    pending_closures: Vec<HirFn>,
}

impl MirBuilder {
    pub fn new() -> Self {
        MirBuilder {
            current_blocks: Vec::new(),
            current_stmts: Vec::new(),
            temp_counter: 0,
            loop_stack: Vec::new(),
            break_patches: Vec::new(),
            continue_patches: Vec::new(),
            method_mangled_names: HashMap::new(),
            closure_counter: 0,
            pending_closures: Vec::new(),
        }
    }

    fn fresh_temp(&mut self) -> String {
        let n = self.temp_counter;
        self.temp_counter += 1;
        format!("t{}", n)
    }

    fn push_stmt(&mut self, stmt: MirStmt) {
        self.current_stmts.push(stmt);
    }

    fn finish_block(&mut self, terminator: Terminator) {
        let id = self.current_blocks.len();
        let stmts = std::mem::take(&mut self.current_stmts);
        self.current_blocks.push(BasicBlock {
            id,
            statements: stmts,
            terminator,
        });
    }

    fn start_block(&mut self) -> usize {
        // If there are pending stmts, flush them into a new block
        if !self.current_stmts.is_empty() {
            let next_id = self.current_blocks.len() + 1;
            self.finish_block(Terminator::Goto(next_id));
        }
        // Return the index where the next block will be created
        self.current_blocks.len()
    }

    pub fn build_program(&mut self, hir: &HirProgram) -> MirProgram {
        let mut functions = Vec::new();

        // Pre-register all method mangled names before building any function
        // (so MethodCall inside main() can resolve trait methods)
        for imp in &hir.impls {
            for m in &imp.methods {
                let mangled = if let Some(ref trait_name) = imp.trait_name {
                    format!("{}_{}_{}", imp.type_name, trait_name, m.name)
                } else {
                    format!("{}_{}", imp.type_name, m.name)
                };
                let key = format!("{}_{}", imp.type_name, m.name);
                self.method_mangled_names.entry(key)
                    .or_insert_with(Vec::new)
                    .push(mangled);
            }
        }

        for f in &hir.functions {
            functions.push(self.build_fn(f));
        }

        // Build impl methods as separate functions
        for imp in &hir.impls {
            for m in &imp.methods {
                let mangled = if let Some(ref trait_name) = imp.trait_name {
                    format!("{}_{}_{}", imp.type_name, trait_name, m.name)
                } else {
                    format!("{}_{}", imp.type_name, m.name)
                };
                let mut method = self.build_fn(m);
                method.name = mangled;
                functions.push(method);
            }
        }

        let structs: Vec<_> = hir.structs.iter()
            .map(|s| (s.name.clone(), s.fields.clone()))
            .collect();
        let enums: Vec<_> = hir.enums.iter()
            .map(|e| (e.name.clone(), e.variants.clone()))
            .collect();

        // Process pending closures (HirFn → MirFn)
        let pending: Vec<HirFn> = self.pending_closures.drain(..).collect();
        let mut closure_fns = Vec::new();
        for hir_fn in &pending {
            closure_fns.push(self.build_fn(hir_fn));
        }
        // Append closure functions after regular functions
        let all_fns: Vec<MirFn> = functions.into_iter().chain(closure_fns).collect();

        MirProgram { functions: all_fns, structs, enums }
    }

    pub fn build_fn(&mut self, f: &HirFn) -> MirFn {
        self.current_blocks.clear();
        self.current_stmts.clear();
        self.temp_counter = 0;
        self.loop_stack.clear();
        self.break_patches.clear();
        self.continue_patches.clear();

        let mut locals: Vec<(String, HirType)> = f.params.iter()
            .map(|(n, t)| (n.clone(), t.clone()))
            .collect();

        // Start with block 0
        let _entry = self.start_block();

        for stmt in &f.body {
            self.lower_stmt(stmt, &mut locals);
        }

        // Flush any pending stmts into a final block.
        // Always terminate — even if current_stmts is empty, the last
        // while/for/if may have called start_block() leaving an unterminated
        // exit block. Without this, the codegen emits `unreachable` which
        // tells LLVM the loop-exit is dead, producing an infinite loop.
        self.finish_block(Terminator::Return(None));

        // Ensure the function has at least one block.
        if self.current_blocks.is_empty() {
            self.finish_block(Terminator::Return(None));
        }

        MirFn {
            name: f.name.clone(),
            params: f.params.clone(),
            return_type: f.return_type.clone(),
            blocks: self.current_blocks.clone(),
            locals,
        }
    }

    fn lower_stmt(&mut self, stmt: &HirStmt, locals: &mut Vec<(String, HirType)>) {
        match stmt {
            HirStmt::Set { name, ty, value } => {
                locals.push((name.clone(), ty.clone()));
                let operand = self.lower_expr(value);
                self.push_stmt(MirStmt::Assign {
                    dest: name.clone(),
                    value: MirRvalue::Use(operand),
                });
            }
            HirStmt::Return(expr) => {
                let operand = self.lower_expr(expr);
                self.finish_block(Terminator::Return(Some(operand)));
                // Start unreachable block after return
                self.start_block();
            }
            HirStmt::If { cond, then, else_ } => {
                self.lower_if(cond, then, else_, locals);
            }
            HirStmt::While { cond, body } => {
                self.lower_while(cond, body, locals);
            }
            HirStmt::For { init, cond, update, body } => {
                self.lower_for(init, cond, update, body, locals);
            }
            HirStmt::Expr(expr) => {
                self.lower_expr(expr);
            }
            HirStmt::Break => {
                if self.loop_stack.last().is_some() {
                    let block_id = self.current_blocks.len();
                    self.finish_block(Terminator::Goto(0)); // placeholder
                    self.break_patches.push(block_id);
                }
            }
            HirStmt::Continue => {
                if self.loop_stack.last().is_some() {
                    let block_id = self.current_blocks.len();
                    self.finish_block(Terminator::Goto(0)); // placeholder
                    self.continue_patches.push(block_id);
                }
            }
        }
    }

    fn lower_if(
        &mut self,
        cond: &HirExpr,
        then: &[HirStmt],
        else_: &[HirStmt],
        locals: &mut Vec<(String, HirType)>,
    ) {
        let cond_operand = self.lower_expr(cond);

        // Push SwitchInt with placeholder targets, patch later
        let switch_idx = self.current_blocks.len();
        self.finish_block(Terminator::SwitchInt {
            cond: cond_operand,
            targets: vec![(1, 0)], // then_block placeholder
            default: 0,            // else/merge placeholder
        });

        // Then block
        let then_block = self.start_block();
        let saved_continue = self.continue_patches.len();
        let saved_break = self.break_patches.len();
        for s in then {
            self.lower_stmt(s, locals);
        }
        self.finish_block(Terminator::Goto(0)); // merge placeholder

        // Else block (if present)
        let else_block = if !else_.is_empty() {
            let eb = self.start_block();
            for s in else_ {
                self.lower_stmt(s, locals);
            }
            self.finish_block(Terminator::Goto(0)); // merge placeholder
            eb
        } else {
            0
        };

        // Merge block
        let merge_block = self.start_block();

        // Patch SwitchInt targets
        let else_or_merge = if else_.is_empty() { merge_block } else { else_block };
        if let Terminator::SwitchInt { ref mut targets, ref mut default, .. } = self.current_blocks[switch_idx].terminator {
            targets[0].1 = then_block;
            *default = else_or_merge;
        }

        // Patch ALL Goto(0) placeholders in then/else blocks → merge_block
        for i in (switch_idx + 1)..merge_block {
            if let Terminator::Goto(ref mut target) = self.current_blocks[i].terminator {
                if *target == 0 { *target = merge_block; }
            }
        }
    }

    fn lower_while(
        &mut self,
        cond: &HirExpr,
        body: &[HirStmt],
        locals: &mut Vec<(String, HirType)>,
    ) {
        // Finish current block with goto to the header
        let next = self.current_blocks.len() + 1;
        self.finish_block(Terminator::Goto(next));

        // Header block: evaluate condition
        let header_block = self.start_block();
        let cond_operand = self.lower_expr(cond);

        let body_block = self.current_blocks.len() + 1;

        self.finish_block(Terminator::SwitchInt {
            cond: cond_operand,
            targets: vec![(1, body_block)],
            default: 0, // exit_block patched later
        });

        // Body block
        let saved_continue = self.continue_patches.len();
        let saved_break = self.break_patches.len();
        self.loop_stack.push((header_block, 0)); // break target patched later
        let _ = self.start_block();
        for s in body {
            self.lower_stmt(s, locals);
        }
        // Always emit loop-back edge. When body ends with if-without-else,
        // lower_if creates a merge block via start_block() which empties
        // current_stmts, causing is_terminated() to return true and skipping
        // the loop-back. Unconditionally emitting it is safe: if the body
        // already ended with return/break, the new block is unreachable
        // but harmless.
        self.finish_block(Terminator::Goto(header_block));
        self.loop_stack.pop();

        // Patch break targets to exit_block
        let exit_block = self.current_blocks.len();
        for i in saved_break..self.break_patches.len() {
            let bid = self.break_patches[i];
            self.current_blocks[bid].terminator = Terminator::Goto(exit_block);
        }
        self.break_patches.truncate(saved_break);

        // Patch continue targets to header_block
        for i in saved_continue..self.continue_patches.len() {
            let bid = self.continue_patches[i];
            self.current_blocks[bid].terminator = Terminator::Goto(header_block);
        }
        self.continue_patches.truncate(saved_continue);

        // Patch SwitchInt default → exit_block
        let switch_idx = body_block - 1;
        if let Terminator::SwitchInt { ref mut default, .. } = self.current_blocks[switch_idx].terminator {
            *default = exit_block;
        }

        // Exit block
        let _ = self.start_block();
    }

    fn lower_for(
        &mut self,
        init: &HirStmt,
        cond: &HirExpr,
        update: &HirStmt,
        body: &[HirStmt],
        locals: &mut Vec<(String, HirType)>,
    ) {
        // Init block
        self.lower_stmt(init, locals);

        let next = self.current_blocks.len() + 1;
        self.finish_block(Terminator::Goto(next));

        // Header: condition check
        let header_block = self.start_block();
        let cond_operand = self.lower_expr(cond);

        let body_block = self.current_blocks.len() + 1;

        self.finish_block(Terminator::SwitchInt {
            cond: cond_operand,
            targets: vec![(1, body_block)],
            default: 0, // exit_block patched later
        });

        // Body block — continue goes to update, break goes to exit (patched later)
        let saved_continue = self.continue_patches.len();
        let saved_break = self.break_patches.len();
        self.loop_stack.push((0, 0)); // both patched later
        let _ = self.start_block();
        for s in body {
            self.lower_stmt(s, locals);
        }
        // Always emit goto to update_block (patched later). Same reasoning as lower_while.
        self.finish_block(Terminator::Goto(0)); // update_block patched later
        self.loop_stack.pop();

        // Update block
        let update_block = self.start_block();
        self.lower_stmt(update, locals);
        if !self.is_terminated() {
            self.finish_block(Terminator::Goto(header_block));
        }

        // Exit block
        let exit_block = self.start_block();

        // Patch all break targets → exit_block
        for i in saved_break..self.break_patches.len() {
            let bid = self.break_patches[i];
            self.current_blocks[bid].terminator = Terminator::Goto(exit_block);
        }
        self.break_patches.truncate(saved_break);

        // Patch all continue targets → update_block
        for i in saved_continue..self.continue_patches.len() {
            let bid = self.continue_patches[i];
            self.current_blocks[bid].terminator = Terminator::Goto(update_block);
        }
        self.continue_patches.truncate(saved_continue);

        // Patch the SwitchInt default → exit_block
        // The SwitchInt block is body_block - 1
        let switch_idx = body_block - 1;
        if let Terminator::SwitchInt { ref mut default, .. } = self.current_blocks[switch_idx].terminator {
            *default = exit_block;
        }

        // Patch the body's final Goto(0) → update_block
        // The body's last block is the one before the update block
        // It might be the SwitchInt block or a later block
        // Find the block that has Goto(0) — it's the last block before update_block
        let body_last = update_block - 1;
        if body_last > switch_idx {
            if let Terminator::Goto(ref mut target) = self.current_blocks[body_last].terminator {
                if *target == 0 {
                    *target = update_block;
                }
            }
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr) -> MirOperand {
        match expr {
            HirExpr::Int(n) => MirOperand::Constant(MirConst::Int(*n)),
            HirExpr::Float(f) => MirOperand::Constant(MirConst::Float(*f)),
            HirExpr::Str(s) => MirOperand::Constant(MirConst::Str(s.clone())),
            HirExpr::Bool(b) => MirOperand::Constant(MirConst::Bool(*b)),
            HirExpr::Var { name, .. } => MirOperand::Var(name.clone()),
            HirExpr::BinOp { op, left, right, .. } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                let mir_op = match op {
                    HirBinOp::Add => MirBinOp::Add,
                    HirBinOp::Sub => MirBinOp::Sub,
                    HirBinOp::Mul => MirBinOp::Mul,
                    HirBinOp::Div => MirBinOp::Div,
                    HirBinOp::Eq => MirBinOp::Eq,
                    HirBinOp::Neq => MirBinOp::Neq,
                    HirBinOp::Lt => MirBinOp::Lt,
                    HirBinOp::Gt => MirBinOp::Gt,
                    HirBinOp::Le => MirBinOp::Le,
                    HirBinOp::Ge => MirBinOp::Ge,
                    HirBinOp::And => MirBinOp::And,
                    HirBinOp::Or => MirBinOp::Or,
                };
                // Try constant folding at MIR level
                if let (MirOperand::Constant(ref lc), MirOperand::Constant(ref rc)) = (&l, &r) {
                    if let Some(result) = const_fold_binop(mir_op, lc, rc) {
                        return MirOperand::Constant(result);
                    }
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Assign {
                    dest: temp.clone(),
                    value: MirRvalue::BinaryOp(mir_op, l, r),
                });
                MirOperand::Var(temp)
            }
            HirExpr::Call { name, args, .. } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expr(arg));
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: name.clone(),
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
            HirExpr::MethodCall { receiver, method, args, .. } => {
                let recv = self.lower_expr(receiver);
                let mut mir_args = vec![recv];
                for arg in args {
                    mir_args.push(self.lower_expr(arg));
                }
                // Determine mangled name: try inherent first, then trait
                let mangled = match receiver.ty() {
                    HirType::Named(type_name) => {
                        let key = format!("{}_{}", type_name, method);
                        if let Some(list) = self.method_mangled_names.get(&key) {
                            list[0].clone()
                        } else {
                            format!("{}_{}", type_name, method)
                        }
                    }
                    _ => method.clone(),
                };
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: mangled,
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
            HirExpr::StructLit { name, fields, .. } => {
                let mut mir_args = Vec::new();
                for (_, val) in fields {
                    mir_args.push(self.lower_expr(val));
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: format!("__struct_{}", name),
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
            HirExpr::FieldAccess { obj, field, .. } => {
                let obj_op = self.lower_expr(obj);
                let temp = self.fresh_temp();
                let struct_name = match obj.ty() {
                    HirType::Named(name) => name.clone(),
                    _ => String::new(),
                };
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: format!("__struct_get_{}_{}", struct_name, field),
                    args: vec![obj_op],
                });
                MirOperand::Var(temp)
            }
            HirExpr::FieldAssign { obj, field, value, .. } => {
                let obj_op = self.lower_expr(obj);
                let val_op = self.lower_expr(value);
                let temp = self.fresh_temp();
                let struct_name = match obj.ty() {
                    HirType::Named(name) => name.clone(),
                    _ => String::new(),
                };
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: format!("__struct_set_{}_{}", struct_name, field),
                    args: vec![obj_op, val_op],
                });
                MirOperand::Var(temp)
            }
            HirExpr::ArrayLit { elems, .. } => {
                let mut mir_args = Vec::new();
                for elem in elems {
                    mir_args.push(self.lower_expr(elem));
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: "__array_lit".to_string(),
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
            HirExpr::Index { obj, idx, .. } => {
                let obj_op = self.lower_expr(obj);
                let idx_op = self.lower_expr(idx);
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: "__index".to_string(),
                    args: vec![obj_op, idx_op],
                });
                MirOperand::Var(temp)
            }
            HirExpr::IndexAssign { obj, idx, value, .. } => {
                let obj_op = self.lower_expr(obj);
                let idx_op = self.lower_expr(idx);
                let val_op = self.lower_expr(value);
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: "__index_assign".to_string(),
                    args: vec![obj_op, idx_op, val_op],
                });
                MirOperand::Var(temp)
            }
            HirExpr::EnumCtor { enum_name, variant, args, .. } => {
                let mut mir_args = Vec::new();
                for arg in args {
                    mir_args.push(self.lower_expr(arg));
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: format!("{}_{}", enum_name, variant),
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
            HirExpr::UnaryMinus(inner, _) => {
                let inner_op = self.lower_expr(inner);
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Assign {
                    dest: temp.clone(),
                    value: MirRvalue::BinaryOp(
                        MirBinOp::Sub,
                        MirOperand::Constant(MirConst::Int(0)),
                        inner_op,
                    ),
                });
                MirOperand::Var(temp)
            }
            HirExpr::UnaryNot(inner, _) => {
                let inner_op = self.lower_expr(inner);
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Assign {
                    dest: temp.clone(),
                    value: MirRvalue::BinaryOp(
                        MirBinOp::Eq,
                        inner_op,
                        MirOperand::Constant(MirConst::Int(0)),
                    ),
                });
                MirOperand::Var(temp)
            }
            HirExpr::Assign { name, value, .. } => {
                let val_op = self.lower_expr(value);
                self.push_stmt(MirStmt::Assign {
                    dest: name.clone(),
                    value: MirRvalue::Use(val_op),
                });
                MirOperand::Var(name.clone())
            }
            HirExpr::Closure { params, body, return_type, .. } => {
                let closure_id = self.closure_counter;
                self.closure_counter += 1;
                let closure_name = format!("__closure_{}", closure_id);
                // Store the HirFn for later processing (after all regular fns are built)
                let closure_hir_fn = HirFn {
                    name: closure_name.clone(),
                    params: params.clone(),
                    return_type: *return_type.clone(),
                    body: body.clone(),
                    is_generic: false,
                    type_params: Vec::new(),
                };
                self.pending_closures.push(closure_hir_fn);
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: "__closure_create".to_string(),
                    args: vec![MirOperand::Constant(MirConst::Int(closure_id as i64))],
                });
                MirOperand::Var(temp)
            }
            HirExpr::ClosureCall { callee, args, .. } => {
                let callee_op = self.lower_expr(callee);
                let mut mir_args = Vec::new();
                mir_args.push(callee_op);
                for arg in args {
                    mir_args.push(self.lower_expr(arg));
                }
                let temp = self.fresh_temp();
                self.push_stmt(MirStmt::Call {
                    dest: Some(temp.clone()),
                    func: "__closure_call".to_string(),
                    args: mir_args,
                });
                MirOperand::Var(temp)
            }
        }
    }

    fn is_terminated(&self) -> bool {
        // A block is terminated if current_stmts is empty and we have blocks
        // (meaning the last block already received a terminator via finish_block)
        self.current_stmts.is_empty() && !self.current_blocks.is_empty()
    }
}

fn const_fold_binop(op: MirBinOp, l: &MirConst, r: &MirConst) -> Option<MirConst> {
    match (op, l, r) {
        (MirBinOp::Add, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a + b)),
        (MirBinOp::Sub, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a - b)),
        (MirBinOp::Mul, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a * b)),
        (MirBinOp::Div, MirConst::Int(a), MirConst::Int(b)) => {
            if *b == 0 { None } else { Some(MirConst::Int(a / b)) }
        }
        (MirBinOp::Eq, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a == b)),
        (MirBinOp::Neq, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a != b)),
        (MirBinOp::Lt, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a < b)),
        (MirBinOp::Gt, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a > b)),
        (MirBinOp::Le, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a <= b)),
        (MirBinOp::Ge, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a >= b)),
        (MirBinOp::Add, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Float(a + b)),
        (MirBinOp::Sub, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Float(a - b)),
        (MirBinOp::Mul, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Float(a * b)),
        (MirBinOp::Div, MirConst::Float(a), MirConst::Float(b)) => {
            if *b == 0.0 { None } else { Some(MirConst::Float(a / b)) }
        }
        (MirBinOp::Eq, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a == b)),
        (MirBinOp::Neq, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a != b)),
        (MirBinOp::Lt, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a < b)),
        (MirBinOp::Gt, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a > b)),
        (MirBinOp::Le, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a <= b)),
        (MirBinOp::Ge, MirConst::Float(a), MirConst::Float(b)) => Some(MirConst::Bool(a >= b)),
        (MirBinOp::And, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(*a && *b)),
        (MirBinOp::Or, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(*a || *b)),
        (MirBinOp::Eq, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(a == b)),
        (MirBinOp::Neq, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(a != b)),
        _ => None,
    }
}
