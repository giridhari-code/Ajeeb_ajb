use crate::ast::{Expr, TypeAnnot};
use super::Codegen;

impl Codegen {
    // Track variable types for method dispatch
    pub(super) fn track_var_type(&mut self, name: &str, init_expr: &Expr) {
        match init_expr {
            Expr::StructLit { struct_name, .. } => {
                // Strip generic type args: "Box[Int]" -> "Box"
                let base_name = if let Some(bracket_pos) = struct_name.find('[') {
                    struct_name[..bracket_pos].to_string()
                } else {
                    struct_name.clone()
                };
                self.var_types.insert(name.to_string(), ("struct".into(), base_name));
            }
            Expr::EnumCtor { enum_name, .. } | Expr::EnumRef { enum_name, .. } => {
                // Strip generic type args: "Option[Int]" -> "Option"
                let base_name = if let Some(bracket_pos) = enum_name.find('[') {
                    enum_name[..bracket_pos].to_string()
                } else {
                    enum_name.clone()
                };
                self.var_types.insert(name.to_string(), ("enum".into(), base_name));
            }
            _ => {}
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
        let struct_type: Option<String> = match obj {
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
        // Step 2: look up the field's TypeAnnot in that struct's definition
        struct_type.and_then(|st| {
            self.struct_defs.get(&st)
                .and_then(|fields| fields.iter().find(|(n, _)| n == field))
                .map(|(_, ty)| ty.clone())
        })
    }
}
