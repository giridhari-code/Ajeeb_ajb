#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
    pub line: usize,
    pub col: usize,
    pub is_block: bool,
    pub end_line: usize,
}

impl Comment {
    pub fn is_line_before(&self, node_line: usize, node_col: usize) -> bool {
        self.line < node_line || (self.line == node_line && self.col < node_col)
    }
}

pub fn extract_comments(source: &str) -> Vec<Comment> {
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut comments = Vec::new();
    let mut line = 1;
    let mut col = 1;
    let mut i = 0;

    while i < len {
        if i + 1 < len {
            if chars[i] == '/' && chars[i + 1] == '/' {
                let start_line = line;
                let start_col = col;
                let mut text = String::new();
                i += 2;
                col += 2;
                // Skip all leading slashes (for /// doc comments)
                while i < len && chars[i] == '/' {
                    i += 1;
                    col += 1;
                }
                // Skip one space if present after slashes
                if i < len && chars[i] == ' ' {
                    i += 1;
                    col += 1;
                }
                while i < len && chars[i] != '\n' {
                    text.push(chars[i]);
                    i += 1;
                    col += 1;
                }
                comments.push(Comment {
                    text: text.trim().to_string(),
                    line: start_line,
                    col: start_col,
                    is_block: false,
                    end_line: line,
                });
                continue;
            }
            if chars[i] == '/' && chars[i + 1] == '*' {
                let start_line = line;
                let start_col = col;
                let mut text = String::new();
                i += 2;
                col += 2;
                let mut end_line = line;
                while i + 1 < len && !(chars[i] == '*' && chars[i + 1] == '/') {
                    if chars[i] == '\n' {
                        text.push('\n');
                        line += 1;
                        col = 1;
                    } else {
                        text.push(chars[i]);
                        col += 1;
                    }
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                    col += 2;
                    end_line = line;
                }
                let trimmed = text
                    .lines()
                    .map(|l| l.trim())
                    .collect::<Vec<_>>()
                    .join("\n");
                comments.push(Comment {
                    text: trimmed,
                    line: start_line,
                    col: start_col,
                    is_block: true,
                    end_line,
                });
                continue;
            }
        }
        if chars[i] == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
        i += 1;
    }

    comments.sort_by(|a, b| {
        if a.line != b.line {
            a.line.cmp(&b.line)
        } else {
            a.col.cmp(&b.col)
        }
    });
    comments
}
