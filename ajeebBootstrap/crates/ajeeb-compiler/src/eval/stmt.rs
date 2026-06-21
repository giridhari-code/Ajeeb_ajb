use super::{Evaluator, RuntimeValue, is_truthy};
use crate::ast::*;

impl Evaluator {
    pub(super) fn exec_stmt(&mut self, stmt: &Stmt) -> RuntimeValue {
        match stmt {
            Stmt::Set { name, value, .. } | Stmt::Const { name, value, .. } => {
                let val = self.eval_expr(value);
                self.insert_var(name.clone(), val);
                RuntimeValue::Void
            }
            Stmt::Expr(expr, ..) => self.eval_expr(expr),
            Stmt::Return { value, .. } => {
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
                ..
            } => {
                if is_truthy(&self.eval_expr(condition)) {
                    for s in then_block {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => return r,
                            RuntimeValue::Continue => return r,
                            _ => {}
                        }
                    }
                } else if let Some(el) = else_block {
                    for s in el {
                        let r = self.exec_stmt(s);
                        match r {
                            RuntimeValue::Return(_) => return r,
                            RuntimeValue::Break => return r,
                            RuntimeValue::Continue => return r,
                            _ => {}
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
                ..
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
            Stmt::While { condition, body, .. } => {
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
            Stmt::Break { .. } => RuntimeValue::Break,
            Stmt::Continue { .. } => RuntimeValue::Continue,
            Stmt::Import(..) | Stmt::FnDef { .. } | Stmt::Class { .. } | Stmt::StructDef { .. } | Stmt::EnumDef { .. } | Stmt::TraitDef { .. } | Stmt::ImplBlock { .. } => RuntimeValue::Void,
        }
    }
}
