use crate::ast::{self, BinOp, Stmt, Expr, TypeAnnot, LambdaBody};
use crate::hir::*;
use std::collections::HashMap;

pub struct HirLowering {
    fn_signatures: HashMap<String, (Vec<(String, HirType)>, HirType)>,
    struct_fields: HashMap<String, Vec<(String, HirType)>>,
    enum_variants: HashMap<String, Vec<(String, Vec<HirType>)>>,
    type_env: HashMap<String, HirType>,
    generic_params: HashMap<String, Vec<String>>,
    in_loop: bool,
    tmp_counter: u32,
}

impl HirLowering {
    pub fn new() -> Self {
        let mut lowering = HirLowering {
            fn_signatures: HashMap::new(),
            struct_fields: HashMap::new(),
            enum_variants: HashMap::new(),
            type_env: HashMap::new(),
            generic_params: HashMap::new(),
            in_loop: false,
            tmp_counter: 0,
        };
        lowering.register_builtins();
        lowering
    }

    fn register_builtins(&mut self) {
        let builtins: Vec<(&str, Vec<(String, HirType)>, HirType)> = vec![
            ("println", vec![("s".to_string(), HirType::Str)], HirType::Void),
            ("print", vec![("s".to_string(), HirType::Str)], HirType::Void),
            ("itoa", vec![("n".to_string(), HirType::Int)], HirType::Str),
            ("len", vec![("s".to_string(), HirType::Str)], HirType::Int),
            ("str_concat", vec![("a".to_string(), HirType::Str), ("b".to_string(), HirType::Str)], HirType::Str),
            ("readArg", vec![("n".to_string(), HirType::Int)], HirType::Str),
            ("exit", vec![("code".to_string(), HirType::Int)], HirType::Void),
            ("getStr", vec![("ptr".to_string(), HirType::Int)], HirType::Str),
            ("exec", vec![("cmd".to_string(), HirType::Str)], HirType::Int),
            ("mkdir", vec![("path".to_string(), HirType::Str)], HirType::Int),
        ];
        for (name, params, ret) in builtins {
            self.fn_signatures.insert(name.to_string(), (params, ret));
        }
    }

    pub fn lower_program(&mut self, stmts: &[Stmt]) -> HirProgram {
        // First pass: collect all signatures
        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, return_type, type_params, .. } => {
                    let hir_params: Vec<(String, HirType)> = params.iter()
                        .map(|(n, t)| (n.clone(), self.resolve_type(t)))
                        .collect();
                    let hir_ret = self.resolve_type(return_type);
                    self.fn_signatures.insert(name.clone(), (hir_params, hir_ret));
                    if !type_params.is_empty() {
                        self.generic_params.insert(name.clone(), type_params.clone());
                    }
                }
                Stmt::Class { name, fields, methods, .. } => {
                    let hir_fields: Vec<(String, HirType)> = fields.iter()
                        .map(|f| (f.name.clone(), self.resolve_type(&f.type_ann)))
                        .collect();
                    self.struct_fields.insert(name.clone(), hir_fields);
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, return_type, type_params, .. } = m {
                            let hir_params: Vec<(String, HirType)> = params.iter()
                                .map(|(n, t)| (n.clone(), self.resolve_type(t)))
                                .collect();
                            let hir_ret = self.resolve_type(return_type);
                            let mangled = format!("{}_{}", name, mname);
                            self.fn_signatures.insert(mangled.clone(), (hir_params, hir_ret));
                            if !type_params.is_empty() {
                                self.generic_params.insert(mangled, type_params.clone());
                            }
                        }
                    }
                }
                Stmt::StructDef { name, fields, type_params, .. } => {
                    let hir_fields: Vec<(String, HirType)> = fields.iter()
                        .map(|f| (f.name.clone(), self.resolve_type(&f.type_ann)))
                        .collect();
                    self.struct_fields.insert(name.clone(), hir_fields);
                }
                Stmt::EnumDef { name, variants, type_params, .. } => {
                    let hir_variants: Vec<(String, Vec<HirType>)> = variants.iter()
                        .map(|v| {
                            let tys = v.fields.iter().map(|t| self.resolve_type(t)).collect();
                            (v.name.clone(), tys)
                        })
                        .collect();
                    self.enum_variants.insert(name.clone(), hir_variants);
                }
                _ => {}
            }
        }

        // Second pass: lower everything
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut traits = Vec::new();
        let mut impls = Vec::new();

        for stmt in stmts {
            match stmt {
                Stmt::FnDef { name, params, return_type, body, type_params, type_param_bounds, .. } => {
                    if name == "main" || !type_params.is_empty() {
                        // Lower main and generic functions
                        let hir_fn = self.lower_fn(name, params, return_type, body, type_params, type_param_bounds);
                        functions.push(hir_fn);
                    } else {
                        let hir_fn = self.lower_fn(name, params, return_type, body, type_params, type_param_bounds);
                        functions.push(hir_fn);
                    }
                }
                Stmt::Class { name, fields, methods, .. } => {
                    let hir_fields: Vec<(String, HirType)> = fields.iter()
                        .map(|f| (f.name.clone(), self.resolve_type(&f.type_ann)))
                        .collect();
                    structs.push(HirStructDef {
                        name: name.clone(),
                        fields: hir_fields,
                        type_params: Vec::new(),
                    });
                    for m in methods {
                        if let Stmt::FnDef { name: mname, params, return_type, body, type_params, type_param_bounds, .. } = m {
                            let mangled = format!("{}_{}", name, mname);
                            let hir_fn = self.lower_fn(&mangled, params, return_type, body, type_params, type_param_bounds);
                            functions.push(hir_fn);
                        }
                    }
                }
                Stmt::StructDef { name, fields, type_params, .. } => {
                    let hir_fields: Vec<(String, HirType)> = fields.iter()
                        .map(|f| (f.name.clone(), self.resolve_type(&f.type_ann)))
                        .collect();
                    structs.push(HirStructDef {
                        name: name.clone(),
                        fields: hir_fields,
                        type_params: type_params.clone(),
                    });
                }
                Stmt::EnumDef { name, variants, type_params, .. } => {
                    let hir_variants: Vec<(String, Vec<HirType>)> = variants.iter()
                        .map(|v| {
                            let tys = v.fields.iter().map(|t| self.resolve_type(t)).collect();
                            (v.name.clone(), tys)
                        })
                        .collect();
                    enums.push(HirEnumDef {
                        name: name.clone(),
                        variants: hir_variants,
                        type_params: type_params.clone(),
                    });
                }
                Stmt::TraitDef { name, methods, type_params, .. } => {
                    let hir_methods: Vec<HirTraitMethod> = methods.iter()
                        .map(|m| HirTraitMethod {
                            name: m.name.clone(),
                            params: m.params.iter().map(|(n, t)| (n.clone(), self.resolve_type(t))).collect(),
                            return_type: self.resolve_type(&m.return_type),
                        })
                        .collect();
                    traits.push(HirTraitDef {
                        name: name.clone(),
                        methods: hir_methods,
                        type_params: type_params.clone(),
                    });
                }
                Stmt::ImplBlock { trait_name, type_name, methods, type_params, .. } => {
                    let hir_methods: Vec<HirFn> = methods.iter().filter_map(|m| {
                        if let Stmt::FnDef { name, params, return_type, body, type_params: mtp, type_param_bounds, .. } = m {
                            Some(self.lower_fn(name, params, return_type, body, mtp, type_param_bounds))
                        } else {
                            None
                        }
                    }).collect();
                    impls.push(HirImplBlock {
                        trait_name: trait_name.clone(),
                        type_name: type_name.clone(),
                        methods: hir_methods,
                        type_params: type_params.clone(),
                    });
                }
                _ => {}
            }
        }

        HirProgram { functions, structs, enums, traits, impls }
    }

    fn lower_fn(
        &mut self,
        name: &str,
        params: &[(String, TypeAnnot)],
        return_type: &TypeAnnot,
        body: &[Stmt],
        type_params: &[String],
        _type_param_bounds: &[(String, Vec<String>)],
    ) -> HirFn {
        let saved_env = self.type_env.clone();

        let hir_params: Vec<(String, HirType)> = params.iter()
            .map(|(n, t)| {
                let ht = self.resolve_type(t);
                self.type_env.insert(n.clone(), ht.clone());
                (n.clone(), ht)
            })
            .collect();

        let hir_ret = self.resolve_type(return_type);
        let hir_body: Vec<HirStmt> = body.iter().flat_map(|s| self.lower_stmt_vec(s)).collect();

        self.type_env = saved_env;

        HirFn {
            name: name.to_string(),
            params: hir_params,
            return_type: hir_ret,
            body: hir_body,
            is_generic: !type_params.is_empty(),
            type_params: type_params.to_vec(),
        }
    }

    fn lower_stmt_vec(&mut self, s: &Stmt) -> Vec<HirStmt> {
        match s {
            Stmt::Expr(expr, ..) => {
                if let Expr::Match { value, arms, .. } = expr {
                    self.lower_match_stmt(value, arms)
                } else {
                    vec![self.lower_stmt(s)]
                }
            }
            _ => vec![self.lower_stmt(s)],
        }
    }

    fn lower_match_stmt(&mut self, value: &Expr, arms: &[ast::MatchArm]) -> Vec<HirStmt> {
        let hir_val = self.lower_expr(value);
        let match_ty = hir_val.ty().clone();
        let tmp_name = format!("__match_{}", self.tmp_counter);
        self.tmp_counter += 1;
        self.type_env.insert(tmp_name.clone(), match_ty.clone());
        let tmp_var = HirExpr::Var { name: tmp_name.clone(), ty: match_ty.clone() };
        // Build nested if-else chain from last arm to first
        let mut chain: Vec<HirStmt> = Vec::new();
        for arm in arms.iter().rev() {
            let body_hir = self.lower_expr(&arm.body);
            let body_stmt = HirStmt::Expr(body_hir);
            match &arm.pattern {
                ast::Pattern::Wildcard => {
                    chain = vec![body_stmt];
                }
                ast::Pattern::Int(n) => {
                    let cond = HirExpr::BinOp {
                        op: HirBinOp::Eq,
                        left: Box::new(tmp_var.clone()),
                        right: Box::new(HirExpr::Int(*n)),
                        ty: HirType::Bool,
                    };
                    chain = vec![HirStmt::If {
                        cond,
                        then: vec![body_stmt],
                        else_: chain,
                    }];
                }
                _ => {
                    chain = vec![body_stmt];
                }
            }
        }
        let mut stmts: Vec<HirStmt> = vec![HirStmt::Set {
            name: tmp_name.clone(),
            ty: match_ty,
            value: hir_val,
        }];
        stmts.extend(chain);
        stmts
    }

    fn lower_stmt(&mut self, s: &Stmt) -> HirStmt {
        match s {
            Stmt::Set { name, type_ann, value, .. } => {
                let hir_val = self.lower_expr(value);
                let ty = if let Some(ann) = type_ann {
                    self.resolve_type(ann)
                } else {
                    hir_val.ty().clone()
                };
                self.type_env.insert(name.clone(), ty.clone());
                HirStmt::Set {
                    name: name.clone(),
                    ty,
                    value: hir_val,
                }
            }
            Stmt::Const { name, type_ann, value, .. } => {
                let hir_val = self.lower_expr(value);
                let ty = if let Some(ann) = type_ann {
                    self.resolve_type(ann)
                } else {
                    hir_val.ty().clone()
                };
                self.type_env.insert(name.clone(), ty.clone());
                HirStmt::Set {
                    name: name.clone(),
                    ty,
                    value: hir_val,
                }
            }
            Stmt::Return { value, .. } => {
                let hir_val = value.as_ref()
                    .map(|e| self.lower_expr(e))
                    .unwrap_or(HirExpr::Int(0));
                HirStmt::Return(hir_val)
            }
            Stmt::If { condition, then_block, else_block, .. } => {
                let cond = self.lower_expr(condition);
                let then: Vec<HirStmt> = then_block.iter().map(|s| self.lower_stmt(s)).collect();
                let else_: Vec<HirStmt> = else_block.as_ref()
                    .map(|eb| eb.iter().map(|s| self.lower_stmt(s)).collect())
                    .unwrap_or_default();
                HirStmt::If { cond, then, else_ }
            }
            Stmt::While { condition, body, .. } => {
                let cond = self.lower_expr(condition);
                let saved = self.in_loop;
                self.in_loop = true;
                let body: Vec<HirStmt> = body.iter().map(|s| self.lower_stmt(s)).collect();
                self.in_loop = saved;
                HirStmt::While { cond, body }
            }
            Stmt::ForLoop { init, condition, update, body, .. } => {
                let init_stmt = Box::new(self.lower_stmt(init));
                let cond = self.lower_expr(condition);
                let update_stmt = Box::new(self.lower_stmt(update));
                let saved = self.in_loop;
                self.in_loop = true;
                let body: Vec<HirStmt> = body.iter().map(|s| self.lower_stmt(s)).collect();
                self.in_loop = saved;
                HirStmt::For {
                    init: init_stmt,
                    cond,
                    update: update_stmt,
                    body,
                }
            }
            Stmt::Break { .. } => HirStmt::Break,
            Stmt::Continue { .. } => HirStmt::Continue,
            Stmt::Expr(expr, ..) => HirStmt::Expr(self.lower_expr(expr)),
            Stmt::FnDef { .. } | Stmt::Class { .. } | Stmt::StructDef { .. }
            | Stmt::EnumDef { .. } | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. }
            | Stmt::Import(_) => {
                // Top-level definitions are handled in lower_program
                HirStmt::Expr(HirExpr::Int(0))
            }
        }
    }

    fn lower_expr(&mut self, e: &Expr) -> HirExpr {
        match e {
            Expr::Number(n, ..) => HirExpr::Int(*n),
            Expr::FloatLit(f, ..) => HirExpr::Float(*f),
            Expr::StringLit(s, ..) => HirExpr::Str(s.clone()),
            Expr::Bool(b, ..) => HirExpr::Bool(*b),
            Expr::Ident(name, ..) => {
                let ty = self.type_env.get(name)
                    .cloned()
                    .unwrap_or(HirType::Unknown);
                HirExpr::Var { name: name.clone(), ty }
            }
            Expr::Binary { left, op, right, .. } => {
                let l = self.lower_expr(left);
                let r = self.lower_expr(right);
                let op = self.lower_binop(op);
                let ty = self.infer_binop_type(&op, l.ty(), r.ty());
                HirExpr::BinOp {
                    op,
                    left: Box::new(l),
                    right: Box::new(r),
                    ty,
                }
            }
            Expr::UnaryMinus(inner, ..) => {
                let hir = self.lower_expr(inner);
                let ty = hir.ty().clone();
                HirExpr::UnaryMinus(Box::new(hir), ty)
            }
            Expr::UnaryNot(inner, ..) => {
                let hir = self.lower_expr(inner);
                HirExpr::UnaryNot(Box::new(hir), HirType::Bool)
            }
            Expr::Group(inner, ..) => self.lower_expr(inner),
            Expr::Assign { name, value, .. } => {
                let hir_val = self.lower_expr(value);
                let ty = self.type_env.get(name).cloned().unwrap_or_else(|| hir_val.ty().clone());
                HirExpr::Assign {
                    name: name.clone(),
                    value: Box::new(hir_val),
                    ty,
                }
            }
            Expr::FnCall { name, args, .. } => {
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                let ty = self.fn_signatures.get(name)
                    .map(|(_, ret)| ret.clone())
                    .unwrap_or(HirType::Unknown);
                HirExpr::Call {
                    name: name.clone(),
                    args: hir_args,
                    ty,
                }
            }
            Expr::GenericCall { name, type_args, args, .. } => {
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                let hir_type_args: Vec<HirType> = type_args.iter().map(|t| self.resolve_type(t)).collect();
                let generic_params = self.generic_params.get(name).cloned().unwrap_or_default();
                let ty = self.fn_signatures.get(name)
                    .map(|(_, ret)| {
                        if generic_params.is_empty() {
                            ret.clone()
                        } else {
                            substitute_type(ret, &generic_params, &hir_type_args)
                        }
                    })
                    .unwrap_or(HirType::Unknown);
                HirExpr::Call {
                    name: name.clone(),
                    args: hir_args,
                    ty,
                }
            }
            Expr::MethodCall { obj, method, args, .. } => {
                let receiver = self.lower_expr(obj);
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                // Try to resolve method return type
                let ty = self.resolve_method_return_type(receiver.ty(), method);
                HirExpr::MethodCall {
                    receiver: Box::new(receiver),
                    method: method.clone(),
                    args: hir_args,
                    ty,
                }
            }
            Expr::Field { obj, field, .. } => {
                let obj_hir = self.lower_expr(obj);
                let ty = self.resolve_field_type(obj_hir.ty(), field);
                HirExpr::FieldAccess {
                    obj: Box::new(obj_hir),
                    field: field.clone(),
                    ty,
                }
            }
            Expr::FieldAssign { obj, field, value, .. } => {
                let obj_hir = self.lower_expr(obj);
                let val_hir = self.lower_expr(value);
                let ty = self.resolve_field_type(obj_hir.ty(), field);
                HirExpr::FieldAssign {
                    obj: Box::new(obj_hir),
                    field: field.clone(),
                    value: Box::new(val_hir),
                    ty,
                }
            }
            Expr::ArrayLit(elems, ..) => {
                let hir_elems: Vec<HirExpr> = elems.iter().map(|e| self.lower_expr(e)).collect();
                let inner_ty = hir_elems.first()
                    .map(|e| e.ty().clone())
                    .unwrap_or(HirType::Unknown);
                HirExpr::ArrayLit {
                    elems: hir_elems,
                    ty: HirType::Array(Box::new(inner_ty)),
                }
            }
            Expr::Index { obj, index, .. } => {
                let obj_hir = self.lower_expr(obj);
                let idx_hir = self.lower_expr(index);
                let ty = match obj_hir.ty() {
                    HirType::Array(inner) => inner.as_ref().clone(),
                    HirType::Str => HirType::Int,
                    _ => HirType::Unknown,
                };
                HirExpr::Index {
                    obj: Box::new(obj_hir),
                    idx: Box::new(idx_hir),
                    ty,
                }
            }
            Expr::IndexAssign { obj, index, value, .. } => {
                let obj_hir = self.lower_expr(obj);
                let idx_hir = self.lower_expr(index);
                let val_hir = self.lower_expr(value);
                HirExpr::IndexAssign {
                    obj: Box::new(obj_hir),
                    idx: Box::new(idx_hir),
                    value: Box::new(val_hir),
                    ty: HirType::Void,
                }
            }
            Expr::StructLit { struct_name, fields, .. } => {
                let hir_fields: Vec<(String, HirExpr)> = fields.iter()
                    .map(|(n, e)| (n.clone(), self.lower_expr(e)))
                    .collect();
                let ty = HirType::Named(struct_name.clone());
                HirExpr::StructLit {
                    name: struct_name.clone(),
                    fields: hir_fields,
                    ty,
                }
            }
            Expr::EnumCtor { enum_name, variant, args, .. } => {
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                let ty = HirType::Named(enum_name.clone());
                HirExpr::EnumCtor {
                    enum_name: enum_name.clone(),
                    variant: variant.clone(),
                    args: hir_args,
                    ty,
                }
            }
            Expr::EnumRef { enum_name, variant: _, .. } => {
                HirExpr::Var {
                    name: format!("{}::{}", enum_name, enum_name),
                    ty: HirType::Named(enum_name.clone()),
                }
            }
            Expr::AssociatedFnCall { type_name, method, args, .. } => {
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                let fn_name = format!("{}_{}", type_name, method);
                let ty = self.fn_signatures.get(&fn_name)
                    .map(|(_, ret)| ret.clone())
                    .unwrap_or(HirType::Unknown);
                HirExpr::Call {
                    name: fn_name,
                    args: hir_args,
                    ty,
                }
            }
            Expr::New { class_name, .. } => {
                HirExpr::StructLit {
                    name: class_name.clone(),
                    fields: Vec::new(),
                    ty: HirType::Named(class_name.clone()),
                }
            }
            Expr::Match { value, arms, .. } => {
                let _hir_val = self.lower_expr(value);
                if let Some(arm) = arms.first() {
                    if let Some(body_block) = &arm.body_block {
                        if let Some(last) = body_block.last() {
                            match last {
                                Stmt::Expr(e, ..) => return self.lower_expr(e),
                                _ => {
                                    let _ = self.lower_stmt(last);
                                }
                            }
                        }
                    }
                    return self.lower_expr(&arm.body);
                }
                HirExpr::Int(0)
            }
            Expr::Lambda { params, return_type, body, .. } => {
                let hir_params: Vec<(String, HirType)> = params.iter()
                    .map(|(n, t)| (n.clone(), self.resolve_type(t)))
                    .collect();
                let param_types: Vec<HirType> = params.iter()
                    .map(|(_, t)| self.resolve_type(t))
                    .collect();
                let hir_ret = return_type.as_ref()
                    .map(|t| self.resolve_type(t))
                    .unwrap_or(HirType::Unknown);
                let prev_env = self.type_env.clone();
                self.type_env.clear();
                for (pname, pty) in &hir_params {
                    self.type_env.insert(pname.clone(), pty.clone());
                }
                let hir_body: Vec<HirStmt> = match body {
                    LambdaBody::Expr(e) => {
                        let hir_e = self.lower_expr(e);
                        vec![HirStmt::Return(hir_e)]
                    }
                    LambdaBody::Block(stmts) => {
                        stmts.iter().map(|s| self.lower_stmt(s)).collect()
                    }
                };
                let captures: Vec<String> = prev_env.keys()
                    .filter(|k| self.type_env.contains_key(*k))
                    .filter(|k| !hir_params.iter().any(|(n, _)| n == *k))
                    .cloned()
                    .collect();
                self.type_env = prev_env;
                HirExpr::Closure {
                    params: hir_params,
                    return_type: Box::new(hir_ret.clone()),
                    body: hir_body,
                    captures,
                    ty: HirType::Fn {
                        params: param_types,
                        ret: Box::new(hir_ret),
                    },
                }
            }
            Expr::ClosureCall { callee, args, .. } => {
                let hir_callee = self.lower_expr(callee);
                let hir_args: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                let ret_ty = match hir_callee.ty() {
                    HirType::Fn { ret, .. } => ret.as_ref().clone(),
                    _ => HirType::Unknown,
                };
                HirExpr::ClosureCall {
                    callee: Box::new(hir_callee),
                    args: hir_args,
                    ty: ret_ty,
                }
            }
        }
    }

    fn lower_binop(&self, op: &BinOp) -> HirBinOp {
        match op {
            BinOp::Add => HirBinOp::Add,
            BinOp::Sub => HirBinOp::Sub,
            BinOp::Mul => HirBinOp::Mul,
            BinOp::Div => HirBinOp::Div,
            BinOp::Eq => HirBinOp::Eq,
            BinOp::Neq => HirBinOp::Neq,
            BinOp::Lt => HirBinOp::Lt,
            BinOp::Gt => HirBinOp::Gt,
            BinOp::Le => HirBinOp::Le,
            BinOp::Ge => HirBinOp::Ge,
            BinOp::And => HirBinOp::And,
            BinOp::Or => HirBinOp::Or,
        }
    }

    fn infer_binop_type(&self, op: &HirBinOp, left: &HirType, right: &HirType) -> HirType {
        match op {
            HirBinOp::Add => {
                if left == &HirType::Str || right == &HirType::Str {
                    HirType::Str
                } else if left == &HirType::Float || right == &HirType::Float {
                    HirType::Float
                } else {
                    HirType::Int
                }
            }
            HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Div => {
                if left == &HirType::Float || right == &HirType::Float {
                    HirType::Float
                } else {
                    HirType::Int
                }
            }
            HirBinOp::Eq | HirBinOp::Neq | HirBinOp::Lt | HirBinOp::Gt
            | HirBinOp::Le | HirBinOp::Ge => HirType::Bool,
            HirBinOp::And | HirBinOp::Or => HirType::Bool,
        }
    }

    fn resolve_type(&self, t: &TypeAnnot) -> HirType {
        match t {
            TypeAnnot::Int => HirType::Int,
            TypeAnnot::Float => HirType::Float,
            TypeAnnot::String => HirType::Str,
            TypeAnnot::Bool => HirType::Bool,
            TypeAnnot::Void => HirType::Void,
            TypeAnnot::Array(inner) => HirType::Array(Box::new(self.resolve_type(inner))),
            TypeAnnot::Class(name) => HirType::Named(name.clone()),
            TypeAnnot::Generic(name) => HirType::Named(name.clone()),
            TypeAnnot::Parameterized { base, args } => {
                let base_name = match base.as_ref() {
                    TypeAnnot::Class(n) => n.clone(),
                    _ => format!("{:?}", base),
                };
                let hir_args: Vec<HirType> = args.iter().map(|a| self.resolve_type(a)).collect();
                HirType::Generic(base_name, hir_args)
            }
            TypeAnnot::Function { param_types, return_type } => {
                HirType::Fn {
                    params: param_types.iter().map(|p| self.resolve_type(p)).collect(),
                    ret: Box::new(self.resolve_type(return_type)),
                }
            }
        }
    }

    fn resolve_method_return_type(&self, receiver: &HirType, method: &str) -> HirType {
        let type_name = match receiver {
            HirType::Named(n) => Some(n.as_str()),
            _ => None,
        };
        if let Some(tn) = type_name {
            // Check inherent methods: Type_method
            let mangled = format!("{}_{}", tn, method);
            if let Some((_, ret)) = self.fn_signatures.get(&mangled) {
                return ret.clone();
            }
            // Check struct field access
            if let Some(fields) = self.struct_fields.get(tn) {
                for (fname, fty) in fields {
                    if fname == method {
                        return fty.clone();
                    }
                }
            }
        }
        HirType::Unknown
    }

    fn resolve_field_type(&self, obj: &HirType, field: &str) -> HirType {
        let type_name = match obj {
            HirType::Named(n) => Some(n.as_str()),
            _ => None,
        };
        if let Some(tn) = type_name {
            if let Some(fields) = self.struct_fields.get(tn) {
                for (fname, fty) in fields {
                    if fname == field {
                        return fty.clone();
                    }
                }
            }
        }
        HirType::Unknown
    }

    pub fn infer_type(&self, e: &HirExpr) -> HirType {
        e.ty().clone()
    }
}

fn substitute_type(ty: &HirType, generic_params: &[String], type_args: &[HirType]) -> HirType {
    match ty {
        HirType::Named(name) => {
            if let Some(pos) = generic_params.iter().position(|p| p == name) {
                type_args.get(pos).cloned().unwrap_or(ty.clone())
            } else {
                ty.clone()
            }
        }
        HirType::Generic(base, args) => {
            let new_args: Vec<HirType> = args.iter()
                .map(|a| substitute_type(a, generic_params, type_args))
                .collect();
            HirType::Generic(base.clone(), new_args)
        }
        HirType::Array(inner) => {
            HirType::Array(Box::new(substitute_type(inner, generic_params, type_args)))
        }
        _ => ty.clone(),
    }
}
