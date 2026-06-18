use std::collections::HashMap;
use crate::ast::{Expr, Stmt, TypeAnnot};
use super::Codegen;

impl Codegen {
    pub(super) fn subst_type_ann(t: &TypeAnnot, subst: &HashMap<String, TypeAnnot>) -> TypeAnnot {
        match t {
            TypeAnnot::Generic(name) => subst.get(name).cloned().unwrap_or(t.clone()),
            TypeAnnot::Array(inner) => TypeAnnot::Array(Box::new(Self::subst_type_ann(inner, subst))),
            TypeAnnot::Class(name) => TypeAnnot::Class(name.clone()),
            other => other.clone(),
        }
    }

    pub(super) fn subst_expr(e: &Expr, subst: &HashMap<String, TypeAnnot>) -> Expr {
        match e {
            Expr::Ident(name, line, col) => Expr::Ident(name.clone(), *line, *col),
            Expr::MethodCall { obj, method, args, line, col } => {
                Expr::MethodCall {
                    obj: Box::new(Self::subst_expr(obj, subst)),
                    method: method.clone(),
                    args: args.iter().map(|a| Self::subst_expr(a, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            Expr::FnCall { name, args, line, col } => {
                Expr::FnCall {
                    name: name.clone(),
                    args: args.iter().map(|a| Self::subst_expr(a, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            Expr::StringLit(s, line, col) => Expr::StringLit(s.clone(), *line, *col),
            Expr::Number(n, line, col) => Expr::Number(*n, *line, *col),
            Expr::Bool(b, line, col) => Expr::Bool(*b, *line, *col),
            Expr::FloatLit(f, line, col) => Expr::FloatLit(*f, *line, *col),
            Expr::Binary { left, op, right, line, col } => {
                Expr::Binary {
                    left: Box::new(Self::subst_expr(left, subst)),
                    op: op.clone(),
                    right: Box::new(Self::subst_expr(right, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Expr::UnaryNot(inner, line, col) => {
                Expr::UnaryNot(Box::new(Self::subst_expr(inner, subst)), *line, *col)
            }
            Expr::Assign { name, value, line, col } => {
                Expr::Assign {
                    name: name.clone(),
                    value: Box::new(Self::subst_expr(value, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Expr::ArrayLit(items, line, col) => {
                Expr::ArrayLit(
                    items.iter().map(|i| Self::subst_expr(i, subst)).collect(),
                    *line,
                    *col,
                )
            }
            other => other.clone(),
        }
    }

    pub(super) fn subst_stmt(s: &Stmt, subst: &HashMap<String, TypeAnnot>) -> Stmt {
        match s {
            Stmt::Set { name, type_ann, value, pub_, line, col } => {
                Stmt::Set {
                    name: name.clone(),
                    type_ann: type_ann.as_ref().map(|t| Self::subst_type_ann(t, subst)),
                    value: Self::subst_expr(value, subst),
                    pub_: *pub_,
                    line: *line,
                    col: *col,
                }
            }
            Stmt::Const { name, type_ann, value, pub_, line, col } => {
                Stmt::Const {
                    name: name.clone(),
                    type_ann: type_ann.as_ref().map(|t| Self::subst_type_ann(t, subst)),
                    value: Self::subst_expr(value, subst),
                    pub_: *pub_,
                    line: *line,
                    col: *col,
                }
            }
            Stmt::Expr(expr, line, col) => {
                Stmt::Expr(Self::subst_expr(expr, subst), *line, *col)
            }
            Stmt::Return { value, line, col } => {
                Stmt::Return {
                    value: value.as_ref().map(|v| Self::subst_expr(v, subst)),
                    line: *line,
                    col: *col,
                }
            }
            Stmt::If { condition, then_block, else_block, line, col } => {
                Stmt::If {
                    condition: Self::subst_expr(condition, subst),
                    then_block: then_block.iter().map(|s| Self::subst_stmt(s, subst)).collect(),
                    else_block: else_block.as_ref().map(|eb| eb.iter().map(|s| Self::subst_stmt(s, subst)).collect()),
                    line: *line,
                    col: *col,
                }
            }
            Stmt::ForLoop { init, condition, update, body, line, col } => {
                Stmt::ForLoop {
                    init: Box::new(Self::subst_stmt(init, subst)),
                    condition: Self::subst_expr(condition, subst),
                    update: Box::new(Self::subst_stmt(update, subst)),
                    body: body.iter().map(|s| Self::subst_stmt(s, subst)).collect(),
                    line: *line,
                    col: *col,
                }
            }
            other => other.clone(),
        }
    }
}
