use super::{Evaluator, RuntimeValue, FrameInfo};
use crate::ast::*;

impl Evaluator {
    pub fn exec_fn_call_raw(&mut self, name: &str, arg_vals: &[RuntimeValue]) -> RuntimeValue {
        self.exec_fn_call_body(name, arg_vals)
    }

    pub fn exec_fn_call(&mut self, name: &str, args: &[Expr]) -> RuntimeValue {
        self.exec_fn_call_at(name, args, 0, 0)
    }

    pub fn exec_fn_call_at(&mut self, name: &str, args: &[Expr], line: usize, col: usize) -> RuntimeValue {
        self.iteration_count += 1;
        if self.iteration_count.is_multiple_of(100000) && std::env::var("AJEEB_TRACE").is_ok() {
            eprintln!(
                "[ITER {}] fn: {} args:{}",
                self.iteration_count,
                name,
                args.len()
            );
        }
        let max_iter: u64 = std::env::var("AJEEB_MAX_ITER")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(u64::MAX);
        if self.iteration_count > max_iter {
            eprintln!(
                "[ITER {}] ABORT (set AJEEB_MAX_ITER to increase)",
                self.iteration_count
            );
            return RuntimeValue::Int(0);
        }

        let arg_vals: Vec<RuntimeValue> = args.iter().map(|a| self.eval_expr(a)).collect();

        self.call_stack.push(FrameInfo {
            function_name: name.to_string(),
            line,
            col,
        });
        let result = self.exec_fn_call_body(name, &arg_vals);
        self.call_stack.pop();
        result
    }
}
