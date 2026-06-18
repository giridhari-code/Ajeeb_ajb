#[derive(Clone)]
pub struct FormatConfig {
    pub indent_size: usize,
    pub max_line_width: usize,
    pub use_tabs: bool,
    pub check_mode: bool,
    pub write_mode: bool,
    pub stdout_mode: bool,
    pub files: Vec<String>,
}

impl Default for FormatConfig {
    fn default() -> Self {
        FormatConfig {
            indent_size: 4,
            max_line_width: 100,
            use_tabs: false,
            check_mode: false,
            write_mode: true,
            stdout_mode: false,
            files: Vec::new(),
        }
    }
}

impl FormatConfig {
    pub fn indent(&self) -> String {
        if self.use_tabs {
            "\t".to_string()
        } else {
            " ".repeat(self.indent_size)
        }
    }
}
