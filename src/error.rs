use std::fmt;

/// A 1-based position in the source file.
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub pos: Position,
    pub message: String,
    pub source_line: String,
}

impl ParseError {
    pub fn new(pos: Position, message: impl Into<String>, source_line: impl Into<String>) -> Self {
        ParseError {
            pos,
            message: message.into(),
            source_line: source_line.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let line_label = self.pos.line.to_string();
        let gutter = " ".repeat(line_label.len());

        // Rustc-style layout: a blank gutter line, the source line, and a
        // caret line under it. The caret has to be computed in characters,
        // not bytes, or it drifts on any line with multi-byte UTF-8.
        writeln!(f, "error: {}", self.message)?;
        writeln!(f, "{} |", gutter)?;
        writeln!(f, "{} | {}", line_label, self.source_line)?;
        let caret = " ".repeat(self.pos.col.saturating_sub(1));
        writeln!(f, "{} | {}^", gutter, caret)?;
        write!(f, "  at line {}, column {}", self.pos.line, self.pos.col)
    }
}

impl std::error::Error for ParseError {}
