use super::Codegen;

impl Codegen {
    /// Look up the mangled function name for a method call.
    /// Checks inherent methods first (exact match), then trait methods (method@trait key).
    pub(super) fn resolve_method(&self, type_name: &str, method: &str) -> Option<String> {
        self.method_map.get(&(type_name.to_string(), method.to_string())).cloned()
            .or_else(|| {
                self.method_map.iter()
                    .find(|(k, _)| k.0 == type_name && k.1.starts_with(&format!("{}@", method)))
                    .map(|(_, v)| v.clone())
            })
    }
}
