use std::fmt::Write;
use super::Codegen;

impl Codegen {
    pub(super) fn global_str(&mut self, s: &str) -> String {
        let idx = self.str_count;
        self.str_count += 1;
        let name = format!(".str.{}", idx);
        // LLVM IR string constants use \XX (two hex digits, NO 'x' prefix) for escapes.
        // We emit printable chars as-is; everything else as \XX.
        let mut escaped = String::new();
        for b in s.bytes() {
            match b {
                b'\\' => escaped.push_str("\\\\"),
                b'"'  => escaped.push_str("\\22"),
                0x20..=0x7e => escaped.push(b as char),
                _ => write!(escaped, "\\{:02x}", b).unwrap(),
            }
        }
        writeln!(self.globals, "@{} = private unnamed_addr constant [{} x i8] c\"{}\\00\"",
            name, s.len() + 1, escaped).unwrap();
        name
    }
}
