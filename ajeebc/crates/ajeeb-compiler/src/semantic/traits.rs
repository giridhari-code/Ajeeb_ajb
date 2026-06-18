use super::SemanticAnalyzer;

impl SemanticAnalyzer {
    pub(super) fn type_implements_trait(&self, type_name: Option<&str>, trait_name: &str) -> bool {
        if let Some(tn) = type_name {
            if let Some(impls) = self.impls.get(tn) {
                for (impl_trait, _, _) in impls {
                    if impl_trait == trait_name {
                        return true;
                    }
                }
            }
            if let Some(impls) = self.impls.get(tn) {
                for (impl_trait, _, _) in impls {
                    if impl_trait == trait_name {
                        return true;
                    }
                }
            }
        }
        false
    }
}
