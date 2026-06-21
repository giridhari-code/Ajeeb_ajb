use crate::ast::*;
use std::collections::HashMap;
use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub(super) fn count_type_args_in_name(name: &str) -> usize {
        if let Some(start) = name.find('[') {
            let rest = &name[start + 1..];
            if let Some(end) = rest.find(']') {
                let inner = &rest[..end];
                if inner.is_empty() {
                    0
                } else {
                    let mut depth = 0usize;
                    let mut count = 1usize;
                    for ch in inner.chars() {
                        match ch {
                            '[' => depth += 1,
                            ']' => depth = depth.saturating_sub(1),
                            ',' if depth == 0 => count += 1,
                            _ => {}
                        }
                    }
                    count
                }
            } else {
                0
            }
        } else {
            0
        }
    }

    pub(super) fn substitute_type_params(ty: &TypeAnnot, subst: &HashMap<String, String>) -> TypeAnnot {
        match ty {
            TypeAnnot::Generic(name) => {
                if let Some(replacement) = subst.get(name) {
                    TypeAnnot::Class(replacement.clone())
                } else {
                    ty.clone()
                }
            }
            TypeAnnot::Parameterized { base, args } => {
                let new_base = Box::new(Self::substitute_type_params(base, subst));
                let new_args = args.iter().map(|a| Self::substitute_type_params(a, subst)).collect();
                TypeAnnot::Parameterized { base: new_base, args: new_args }
            }
            TypeAnnot::Array(inner) => {
                TypeAnnot::Array(Box::new(Self::substitute_type_params(inner, subst)))
            }
            _ => ty.clone(),
        }
    }

    pub(super) fn infer_fn_call_generic(&mut self, name: &str, type_args: &[TypeAnnot], args: &[Expr], line: usize, col: usize) -> TypeAnnot {
        if let Some((params, return_ty)) = self.functions.get(name).cloned() {
            let type_params: Vec<String> = {
                let mut seen = std::collections::HashSet::new();
                params.iter()
                    .filter_map(|(_, t)| if let TypeAnnot::Generic(s) = t {
                        if seen.insert(s.clone()) { Some(s.clone()) } else { None }
                    } else { None })
                    .collect()
            };
            let expected = type_params.len();
            let got = type_args.len();
            if expected == 0 {
                if got > 0 {
                    self.errors.push(self.err(
                        line, col,
                        format!("Function '{}' does not accept type arguments", name),
                    ));
                }
            } else if got == 0 {
                self.errors.push(self.err(
                    line, col,
                    format!("Generic function '{}' requires {} type argument(s) but got 0", name, expected),
                ));
            } else if got != expected {
                self.errors.push(self.err(
                    line, col,
                    format!("Generic function '{}' expects {} type argument(s) but got {}", name, expected, got),
                ));
            }
            let mut subst = std::collections::HashMap::new();
            for (i, tp) in type_params.iter().enumerate() {
                if i < type_args.len() {
                    subst.insert(tp.clone(), type_args[i].clone());
                }
            }
            if let Some(fn_bounds) = self.fn_generic_bounds.get(name) {
                for (param_name, bounds) in fn_bounds {
                    if let Some(concrete_ty) = subst.get(param_name) {
                        let concrete_name = match concrete_ty {
                            TypeAnnot::Class(n) => Some(n.as_str()),
                            _ => None,
                        };
                        for bound_trait in bounds {
                            if !self.type_implements_trait(concrete_name, bound_trait) {
                                self.errors.push(self.err(
                                    line, col,
                                    format!(
                                        "Type argument does not satisfy bound '{}' for parameter '{}'",
                                        bound_trait, param_name
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
            let pcount = params.len();
            if pcount != args.len() {
                self.errors.push(self.err(
                    line, col,
                    format!("Function '{}' expects {} arguments but got {}", name, pcount, args.len()),
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                let arg_ty = self.infer_expr_type(arg);
                if i < pcount {
                    let param_ty = &params[i].1;
                    if !self.types_match(param_ty, &arg_ty) {
                        self.errors.push(self.err(
                            line, col,
                            format!("Type mismatch: argument {} of '{}' expected {:?} but got {:?}", i + 1, name, param_ty, arg_ty),
                        ));
                    }
                }
            }
            return_ty
        } else {
            self.infer_fn_call(name, args, line, col)
        }
    }

    pub(super) fn extract_type_args_from_name(encoded: &str) -> Vec<String> {
        if let Some(bracket_pos) = encoded.find('[') {
            let end = encoded.rfind(']').unwrap_or(encoded.len());
            let inner = &encoded[bracket_pos+1..end];
            if inner.is_empty() { return vec![]; }
            let mut args = Vec::new();
            let mut depth = 0;
            let mut current = String::new();
            for ch in inner.chars() {
                match ch {
                    '[' => { depth += 1; current.push(ch); }
                    ']' => { depth -= 1; current.push(ch); }
                    ',' if depth == 0 => { args.push(current.trim().to_string()); current = String::new(); }
                    _ => current.push(ch),
                }
            }
            if !current.trim().is_empty() { args.push(current.trim().to_string()); }
            args
        } else {
            vec![]
        }
    }

    pub(super) fn validate_struct_bounds(&mut self, struct_name: &str, line: usize, col: usize) {
        let base_name = if let Some(bp) = struct_name.find('[') { &struct_name[..bp] } else { struct_name };
        if let Some(bounds) = self.struct_type_param_bounds.get(base_name).cloned() {
            let type_args = Self::extract_type_args_from_name(struct_name);
            for (i, (param_name, bound_traits)) in bounds.iter().enumerate() {
                if i < type_args.len() {
                    let concrete = &type_args[i];
                    let concrete_base = if let Some(bp) = concrete.find('[') { &concrete[..bp] } else { concrete.as_str() };
                    for bound in bound_traits {
                        if !self.type_implements_trait(Some(concrete_base), bound) {
                            self.errors.push(self.err(line, col,
                                format!("Type argument '{}' does not satisfy bound '{}' for parameter '{}'",
                                    concrete, bound, param_name)));
                        }
                    }
                }
            }
        }
    }

    pub(super) fn validate_enum_bounds(&mut self, enum_name: &str, line: usize, col: usize) {
        let base_name = if let Some(bp) = enum_name.find('[') { &enum_name[..bp] } else { enum_name };
        if let Some(bounds) = self.enum_type_param_bounds.get(base_name).cloned() {
            let type_args = Self::extract_type_args_from_name(enum_name);
            for (i, (param_name, bound_traits)) in bounds.iter().enumerate() {
                if i < type_args.len() {
                    let concrete = &type_args[i];
                    let concrete_base = if let Some(bp) = concrete.find('[') { &concrete[..bp] } else { concrete.as_str() };
                    for bound in bound_traits {
                        if !self.type_implements_trait(Some(concrete_base), bound) {
                            self.errors.push(self.err(line, col,
                                format!("Type argument '{}' does not satisfy bound '{}' for parameter '{}'",
                                    concrete, bound, param_name)));
                        }
                    }
                }
            }
        }
    }
}
