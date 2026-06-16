use crate::ast::{Expr, TypeAnnot};
use std::collections::HashMap;
use super::Codegen;

impl Codegen {
    // Track variable types for method dispatch
    pub(super) fn track_var_type(&mut self, name: &str, init_expr: &Expr) {
        match init_expr {
            Expr::StructLit { struct_name, .. } => {
                // Store full generic name (e.g. "Box[string]") for field type resolution
                self.var_types.insert(name.to_string(), ("struct".into(), struct_name.clone()));
            }
            Expr::EnumCtor { enum_name, .. } | Expr::EnumRef { enum_name, .. } => {
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    enum_name[..bracket_pos].to_string()
                } else {
                    enum_name.clone()
                };
                self.var_types.insert(name.to_string(), ("enum".into(), base_name));
            }
            Expr::AssociatedFnCall { type_name, method, .. } => {
                let base_name = if let Some(bracket_pos) = type_name.find('[') {
                    type_name[..bracket_pos].to_string()
                } else {
                    type_name.clone()
                };
                // Check if the associated fn returns the same struct type (e.g. Counter::new -> Counter)
                let mangled = format!("{}_{}", base_name, method);
                if let Some(ret) = self.fn_return_types.get(&mangled).cloned() {
                    match ret {
                        TypeAnnot::Class(ref cn) => {
                            let base_cn = if let Some(p) = cn.find('[') { cn[..p].to_string() } else { cn.clone() };
                            self.var_types.insert(name.to_string(), ("struct".into(), base_cn));
                        }
                        TypeAnnot::Generic(ref gn) => {
                            let base_gn = if let Some(p) = gn.find('[') { gn[..p].to_string() } else { gn.clone() };
                            if self.struct_defs.contains_key(&base_gn) {
                                self.var_types.insert(name.to_string(), ("struct".into(), base_gn));
                            } else if self.enum_defs.contains_key(&base_gn) {
                                self.var_types.insert(name.to_string(), ("enum".into(), base_gn));
                            }
                        }
                        _ => {}
                    }
                } else if self.struct_defs.contains_key(&base_name) {
                    // Heuristic: if the type is a known struct, assume it returns itself
                    self.var_types.insert(name.to_string(), ("struct".into(), base_name));
                } else if self.enum_defs.contains_key(&base_name) {
                    self.var_types.insert(name.to_string(), ("enum".into(), base_name));
                }
            }
            Expr::MethodCall { method, .. } => {
                if let Some((kind, tn)) = self.infer_method_return_type(init_expr) {
                    if kind == "struct" || kind == "enum" {
                        self.var_types.insert(name.to_string(), (kind, tn));
                    }
                }
            }
            Expr::FnCall { name: fn_name, .. } => {
                // Track return type of user-defined functions
                if let Some(ret) = self.fn_return_types.get(fn_name.as_str()).cloned() {
                    match ret {
                        TypeAnnot::Class(ref cn) => {
                            let base_cn = if let Some(p) = cn.find('[') { cn[..p].to_string() } else { cn.clone() };
                            self.var_types.insert(name.to_string(), ("struct".into(), base_cn));
                        }
                        TypeAnnot::Generic(ref gn) => {
                            let base_gn = if let Some(p) = gn.find('[') { gn[..p].to_string() } else { gn.clone() };
                            if self.struct_defs.contains_key(&base_gn) {
                                self.var_types.insert(name.to_string(), ("struct".into(), base_gn));
                            } else if self.enum_defs.contains_key(&base_gn) {
                                self.var_types.insert(name.to_string(), ("enum".into(), base_gn));
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // Infer the return type of a method call by resolving the method and looking up its return type
    pub(super) fn infer_method_return_type(&self, expr: &Expr) -> Option<(String, String)> {
        if let Expr::MethodCall { obj, method, .. } = expr {
            let receiver_type = match obj.as_ref() {
                Expr::StructLit { struct_name, .. } => {
                    let base = if let Some(p) = struct_name.find('[') { &struct_name[..p] } else { struct_name };
                    Some(("struct".to_string(), base.to_string()))
                }
                Expr::EnumCtor { enum_name, .. } | Expr::EnumRef { enum_name, .. } => {
                    let base = if let Some(p) = enum_name.find('[') { &enum_name[..p] } else { enum_name };
                    Some(("enum".to_string(), base.to_string()))
                }
                Expr::Ident(var, ..) => self.var_types.get(var).cloned(),
                _ => None,
            };
            if let Some((kind, type_name)) = receiver_type {
                if let Some(mangled) = self.resolve_method(&type_name, method) {
                    if let Some(ret) = self.fn_return_types.get(mangled.as_str()).cloned() {
                        match ret {
                            TypeAnnot::Class(cn) => {
                                let base = if let Some(p) = cn.find('[') { cn[..p].to_string() } else { cn };
                                return Some(("struct".to_string(), base));
                            }
                            TypeAnnot::Generic(gn) => {
                                let base = if let Some(p) = gn.find('[') { gn[..p].to_string() } else { gn };
                                return Some(("struct".to_string(), base));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }

    // Check if a function/method returns a string type
    pub(super) fn returns_string(&self, expr: &Expr) -> bool {
        match expr {
            Expr::FnCall { name, .. } => {
                if let Some(ret) = self.fn_return_types.get(name.as_str()) {
                    matches!(ret, TypeAnnot::String)
                } else {
                    false
                }
            }
            Expr::MethodCall { obj, method, .. } => {
                let receiver_type = match obj.as_ref() {
                    Expr::StructLit { struct_name, .. } => {
                        let base = if let Some(p) = struct_name.find('[') { &struct_name[..p] } else { struct_name };
                        Some(base.to_string())
                    }
                    Expr::Ident(var, ..) => self.var_types.get(var).map(|(_, tn)| tn.clone()),
                    _ => None,
                };
                if let Some(type_name) = receiver_type {
                    if let Some(mangled) = self.resolve_method(&type_name, method) {
                        if let Some(ret) = self.fn_return_types.get(mangled.as_str()) {
                            return matches!(ret, TypeAnnot::String);
                        }
                    }
                }
                false
            }
            Expr::AssociatedFnCall { type_name, method, .. } => {
                let base_name = if let Some(p) = type_name.find('[') { &type_name[..p] } else { type_name };
                let mangled = format!("{}_{}", base_name, method);
                if let Some(ret) = self.fn_return_types.get(mangled.as_str()) {
                    matches!(ret, TypeAnnot::String)
                } else {
                    false
                }
            }
            Expr::GenericCall { name, .. } => {
                if let Some(ret) = self.fn_return_types.get(name.as_str()) {
                    matches!(ret, TypeAnnot::String)
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    // Determine which struct a field access expression refers to,
    // then return the positional offset for that field within that struct.
    pub(super) fn resolve_field_offset(&self, obj: &Expr, field: &str) -> Option<usize> {
        // Try to determine the struct type from the object expression
        let struct_type = match obj {
            Expr::StructLit { struct_name, .. } => {
                let base = struct_name.split('[').next().unwrap_or(struct_name);
                Some(base.to_string())
            }
            Expr::Ident(name, ..) => {
                self.var_types.get(name).and_then(|(kind, tn)| {
                    if kind == "struct" { Some(tn.clone()) } else { None }
                })
            }
            Expr::Field { obj: inner, field: inner_field, .. } => {
                // Chain: look up parent's struct type, then find field's type
                let inner_type = match inner.as_ref() {
                    Expr::Ident(v, ..) => self.var_types.get(v).map(|(_, tn)| tn.clone()),
                    _ => None,
                };
                inner_type.and_then(|tn| {
                    self.struct_defs.get(&tn)
                        .and_then(|fields| fields.iter().find(|(n, _)| n == inner_field))
                        .map(|(_, ty)| match ty {
                            TypeAnnot::Class(s) | TypeAnnot::Generic(s) => s.clone(),
                            _ => String::new(),
                        })
                })
            }
            _ => None,
        };
        // Use the specific struct's field list; fall back to searching all structs
        if let Some(st) = struct_type {
            self.struct_defs.get(&st)
                .and_then(|fields| fields.iter().position(|(n, _)| n == field))
        } else {
            self.struct_defs.iter()
                .find_map(|(_, fields)| fields.iter().position(|(n, _)| n == field))
        }
    }

    // Determine which struct a field access expression refers to,
    // then return the TypeAnnot for that field.
    pub(super) fn resolve_field_type(&self, obj: &Expr, field: &str) -> Option<TypeAnnot> {
        // Step 1: resolve the struct type name (same logic as resolve_field_offset)
        // Also capture the full name for generic resolution
        let (struct_type, full_struct_name): (Option<String>, Option<String>) = match obj {
            Expr::StructLit { struct_name, .. } => {
                let base = struct_name.split('[').next().unwrap_or(struct_name);
                (Some(base.to_string()), Some(struct_name.clone()))
            }
            Expr::Ident(name, ..) => {
                let info = self.var_types.get(name).and_then(|(kind, tn)| {
                    if kind == "struct" { Some(tn.clone()) } else { None }
                });
                // var_types may store "Box[string]" (full) or "Box" (base)
                // We need both: base for struct_defs lookup, full for generic resolution
                let base_name = info.as_ref().map(|tn| {
                    tn.split('[').next().unwrap_or(tn).to_string()
                });
                (base_name, info.clone()) // struct_type = base, full_struct_name = full
            }
            Expr::Field { obj: inner, field: inner_field, .. } => {
                // Chain: look up parent's struct type, then find field's type
                let inner_type = match inner.as_ref() {
                    Expr::Ident(v, ..) => self.var_types.get(v).map(|(_, tn)| tn.clone()),
                    _ => None,
                };
                let resolved = inner_type.and_then(|tn| {
                    self.struct_defs.get(&tn)
                        .and_then(|fields| fields.iter().find(|(n, _)| n == inner_field))
                        .map(|(_, ty)| match ty {
                            TypeAnnot::Class(s) | TypeAnnot::Generic(s) => s.clone(),
                            _ => String::new(),
                        })
                });
                (resolved, None)
            }
            _ => (None, None),
        };
        // Step 2: look up the field's TypeAnnot in that struct's definition
        // If the result is Generic and we have a full generic name, try resolving
        let direct_result = struct_type.as_ref().and_then(|st| {
            self.struct_defs.get(st)
                .and_then(|fields| fields.iter().find(|(n, _)| n == field))
                .map(|(_, ty)| ty.clone())
        });
        // If we got a concrete type (not Generic), return it directly
        if let Some(ref dt) = direct_result {
            if !matches!(dt, TypeAnnot::Generic(_)) {
                return direct_result;
            }
        }
        // If direct_result is Some(Generic), fall through to generic resolution below
        // Step 3: try to resolve generic fields
        // e.g. for Box[string], struct_type = "Box" but field type is Generic("T")
        // We need to find the generic struct def "Box[T]" and substitute T=string
        if let (Some(ref base), Some(ref full)) = (&struct_type, &full_struct_name) {
            if let Some(def_params) = self.struct_type_params.get(base.as_str()) {
                let full_args: Option<Vec<String>> = full.split('[').nth(1)
                    .map(|s| {
                        let cleaned = s.trim_end_matches(']');
                        cleaned.split(',').map(|s| s.trim().to_string()).collect()
                    });
                if let (Some(args), Some(fields)) = (full_args, self.struct_defs.get(base.as_str())) {
                    let mapping: HashMap<String, String> = def_params.iter()
                        .cloned()
                        .zip(args.into_iter())
                        .collect();
                    if let Some(resolved) = fields.iter().find(|(n, _)| n == field).and_then(|(_, ty)| {
                        match ty {
                            TypeAnnot::Generic(t) => {
                                mapping.get(t).map(|resolved| {
                                    if self.struct_defs.contains_key(resolved) {
                                        TypeAnnot::Class(resolved.clone())
                                    } else if resolved.eq_ignore_ascii_case("string") {
                                        TypeAnnot::String
                                    } else if resolved.eq_ignore_ascii_case("int") {
                                        TypeAnnot::Int
                                    } else if resolved.eq_ignore_ascii_case("bool") {
                                        TypeAnnot::Bool
                                    } else {
                                        TypeAnnot::Generic(resolved.clone())
                                    }
                                })
                            }
                            TypeAnnot::Class(cn) => {
                                if let Some(resolved) = mapping.get(cn) {
                                    Some(TypeAnnot::Class(resolved.clone()))
                                } else {
                                    Some(ty.clone())
                                }
                            }
                            _ => Some(ty.clone()),
                        }
                    }) {
                        return Some(resolved);
                    }
                }
            }
        }
        None
    }
}
