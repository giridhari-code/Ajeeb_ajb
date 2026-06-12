use std::fmt;

#[derive(Debug, Clone)]
pub struct CompileError {
    pub line: usize,
    pub col: usize,
    pub message: String,
}

impl CompileError {
    pub fn new(line: usize, col: usize, message: String) -> Self {
        CompileError { line, col, message }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "❌ Line {}, Col {}: {}",
            self.line, self.col, self.message
        )
    }
}
