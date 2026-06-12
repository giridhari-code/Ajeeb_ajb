use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct DasSection {
    pub entries: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct DasConfig {
    pub sections: HashMap<String, DasSection>,
}

impl DasConfig {
    pub fn parse(source: &str) -> Self {
        let mut sections = HashMap::new();
        let mut current_section: Option<String> = None;

        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                let name = trimmed[1..trimmed.len() - 1].trim().to_string();
                sections.entry(name.clone()).or_insert_with(|| DasSection {
                    entries: HashMap::new(),
                });
                current_section = Some(name);
                continue;
            }

            if let Some(eq_pos) = trimmed.find('=') {
                if let Some(ref section_name) = current_section {
                    let key = trimmed[..eq_pos].trim().to_string();
                    let raw = trimmed[eq_pos + 1..].trim();
                    let value = raw.trim_matches('"').to_string();
                    if let Some(section) = sections.get_mut(section_name) {
                        section.entries.insert(key, value);
                    }
                }
            }
        }

        DasConfig { sections }
    }

    pub fn get(&self, section: &str, key: &str) -> Option<&String> {
        self.sections.get(section)?.entries.get(key)
    }

    pub fn is_enabled(&self, section: &str, key: &str) -> bool {
        self.get(section, key).is_some_and(|v| v == "enabled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parse() {
        let src = r#"
            [package]
            name = "my-project"
            version = "0.1.0"

            [dependencies]

            [runtime]
            max_threads = "8"
            log_level = "info"

            [compiler]
            target = "native"
            output = "build/"
            runtime = "runtime/ajeeb_runtime.c"
        "#;
        let cfg = DasConfig::parse(src);
        assert_eq!(cfg.get("package", "name").unwrap(), "my-project");
        assert_eq!(cfg.get("compiler", "output").unwrap(), "build/");
    }
}
