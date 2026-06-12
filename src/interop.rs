use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LanguageBridge {
    external_registry: HashMap<String, String>,
}

impl LanguageBridge {
    pub fn new() -> Self {
        LanguageBridge {
            external_registry: HashMap::new(),
        }
    }

    pub fn load_compatibility_block(&mut self, language: &str, module_name: &str) {
        println!(
            "🔄 [Ajeeb Bridge] Loading {} compatibility for module: '{}'...",
            language, module_name
        );
        self.external_registry
            .insert(module_name.to_string(), language.to_string());
    }

    pub fn resolve(&self, module_name: &str) -> Option<&String> {
        self.external_registry.get(module_name)
    }

    pub fn summary(&self) {
        if self.external_registry.is_empty() {
            println!("  ╰ No external bridges active");
            return;
        }
        for (module, lang) in &self.external_registry {
            println!("  ├── {}  →  via [{}] bridge", module, lang);
        }
        println!(
            "  ╰ Total: {} external module(s)",
            self.external_registry.len()
        );
    }
}
