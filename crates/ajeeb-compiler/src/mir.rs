use crate::hir::HirType;

#[derive(Debug, Clone)]
pub struct MirProgram {
    pub functions: Vec<MirFn>,
    pub structs: Vec<(String, Vec<(String, HirType)>)>,
    pub enums: Vec<(String, Vec<(String, Vec<HirType>)>)>,
}

#[derive(Debug, Clone)]
pub struct MirFn {
    pub name: String,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
    pub blocks: Vec<BasicBlock>,
    pub locals: Vec<(String, HirType)>,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: usize,
    pub statements: Vec<MirStmt>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone)]
pub enum MirStmt {
    Assign {
        dest: String,
        value: MirRvalue,
    },
    Call {
        dest: Option<String>,
        func: String,
        args: Vec<MirOperand>,
    },
}

#[derive(Debug, Clone)]
pub enum MirRvalue {
    Use(MirOperand),
    BinaryOp(MirBinOp, MirOperand, MirOperand),
    Const(MirConst),
}

#[derive(Debug, Clone)]
pub enum MirOperand {
    Var(String),
    Constant(MirConst),
}

#[derive(Debug, Clone)]
pub enum MirConst {
    Int(i64),
    Str(String),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MirBinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Goto(usize),
    SwitchInt {
        cond: MirOperand,
        targets: Vec<(i64, usize)>,
        default: usize,
    },
    Return(Option<MirOperand>),
    Unreachable,
}

// ── MIR Optimizations ──────────────────────────────────────────────

pub fn optimize_mir(prog: &mut MirProgram) {
    for f in &mut prog.functions {
        constant_fold(f);
        dead_block_elim(f);
    }
}

fn constant_fold(f: &mut MirFn) {
    for block in &mut f.blocks {
        let mut folded = Vec::new();
        for stmt in block.statements.drain(..) {
            match &stmt {
                MirStmt::Assign { dest, value: MirRvalue::BinaryOp(op, left, right) } => {
                    if let (MirOperand::Constant(lc), MirOperand::Constant(rc)) = (left, right) {
                        if let Some(result) = eval_const_binop(*op, lc, rc) {
                            folded.push(MirStmt::Assign {
                                dest: dest.clone(),
                                value: MirRvalue::Const(result),
                            });
                            continue;
                        }
                    }
                    folded.push(stmt);
                }
                _ => folded.push(stmt),
            }
        }
        block.statements = folded;
    }
}

fn eval_const_binop(op: MirBinOp, l: &MirConst, r: &MirConst) -> Option<MirConst> {
    match (op, l, r) {
        (MirBinOp::Add, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a + b)),
        (MirBinOp::Sub, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a - b)),
        (MirBinOp::Mul, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a * b)),
        (MirBinOp::Div, MirConst::Int(_), MirConst::Int(0)) => None,
        (MirBinOp::Div, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Int(a / b)),
        (MirBinOp::Eq, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a == b)),
        (MirBinOp::Neq, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a != b)),
        (MirBinOp::Lt, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a < b)),
        (MirBinOp::Gt, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a > b)),
        (MirBinOp::Le, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a <= b)),
        (MirBinOp::Ge, MirConst::Int(a), MirConst::Int(b)) => Some(MirConst::Bool(a >= b)),
        (MirBinOp::And, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(*a && *b)),
        (MirBinOp::Or, MirConst::Bool(a), MirConst::Bool(b)) => Some(MirConst::Bool(*a || *b)),
        _ => None,
    }
}

fn dead_block_elim(f: &mut MirFn) {
    if f.blocks.is_empty() { return; }

    let block_count = f.blocks.len();

    // First, clamp all terminator targets to valid ranges
    for block in &mut f.blocks {
        match &mut block.terminator {
            Terminator::Goto(target) => {
                if *target >= block_count { *target = block_count - 1; }
            }
            Terminator::SwitchInt { targets, default, .. } => {
                for (_, t) in targets.iter_mut() {
                    if *t >= block_count { *t = block_count - 1; }
                }
                if *default >= block_count { *default = block_count - 1; }
            }
            _ => {}
        }
    }

    // Mark reachable blocks from block 0
    let mut reachable = vec![false; f.blocks.len()];
    let mut stack = vec![0usize];
    while let Some(id) = stack.pop() {
        if id >= f.blocks.len() || reachable[id] { continue; }
        reachable[id] = true;
        match &f.blocks[id].terminator {
            Terminator::Goto(target) => {
                if !reachable[*target] { stack.push(*target); }
            }
            Terminator::SwitchInt { targets, default, .. } => {
                for (_, t) in targets {
                    if !reachable[*t] { stack.push(*t); }
                }
                if !reachable[*default] { stack.push(*default); }
            }
            _ => {}
        }
    }

    // Remove unreachable blocks (except keep at least block 0)
    if reachable.iter().all(|r| *r) { return; }

    // Build index remapping
    let mut remap = vec![0usize; f.blocks.len()];
    let mut new_blocks = Vec::new();
    for (i, block) in f.blocks.iter().enumerate() {
        if reachable[i] {
            remap[i] = new_blocks.len();
            new_blocks.push(block.clone());
        }
    }

    // Remap block IDs in terminators and update block.id
    for block in &mut new_blocks {
        match &mut block.terminator {
            Terminator::Goto(target) => { *target = remap[*target]; }
            Terminator::SwitchInt { targets, default, .. } => {
                for (_, t) in targets.iter_mut() { *t = remap[*t]; }
                *default = remap[*default];
            }
            _ => {}
        }
    }

    // Update block IDs to match new indices
    for (i, block) in new_blocks.iter_mut().enumerate() {
        block.id = i;
    }

    f.blocks = new_blocks;
}
