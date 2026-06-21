use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub fn set_module_prefix(&mut self, prefix: &str) {
        self.module_prefix = prefix.to_string();
    }

    pub(super) fn qualify(&self, name: &str) -> String {
        if self.module_prefix.is_empty() {
            name.to_string()
        } else {
            format!("{}::{}", self.module_prefix, name)
        }
    }
}
