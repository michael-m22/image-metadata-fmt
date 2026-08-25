use crate::error::{ParseError, Position};

/// One `key: value` (or `key = value`) line, before any normalization.
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub key: String,
    pub value: String,
    pub line: usize,
}

/// Parses an .imeta sidecar file into raw key/value entries.
///
/// The format is deliberately loose on input (that's the whole point of the
/// formatter) but a line still has to resolve to a key and a value, and a
/// quoted value still has to be closed. Anything that fails those checks
/// gets reported with the exact line and column of the problem.
pub fn parse(input: &str) -> Result<Vec<RawEntry>, ParseError> {
    let mut entries = Vec::new();

    for (line_idx, raw_line) in input.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let sep_byte = match find_separator(raw_line) {
            Some(idx) => idx,
            None => {
                return Err(ParseError::new(
                    Position {
                        line: line_no,
                        col: raw_line.chars().count() + 1,
                    },
                    "expected ':' or '=' to separate key and value",
                    raw_line.to_string(),
                ));
            }
        };

        let key = raw_line[..sep_byte].trim();
        if key.is_empty() {
            return Err(ParseError::new(
                Position { line: line_no, col: 1 },
                "empty key before separator",
                raw_line.to_string(),
            ));
        }

        let rest = &raw_line[sep_byte + 1..];
        let leading_ws = rest.len() - rest.trim_start().len();
        let value_start_byte = sep_byte + 1 + leading_ws;
        let value = rest.trim();

        if value.starts_with('"') && !is_closed_string(value) {
            return Err(ParseError::new(
                Position {
                    line: line_no,
                    col: char_col(raw_line, value_start_byte),
                },
                "unterminated string literal",
                raw_line.to_string(),
            ));
        }

        entries.push(RawEntry {
            key: key.to_string(),
            value: value.to_string(),
            line: line_no,
        });
    }

    Ok(entries)
}

fn find_separator(line: &str) -> Option<usize> {
    line.char_indices()
        .find(|&(_, c)| c == ':' || c == '=')
        .map(|(i, _)| i)
}

fn is_closed_string(value: &str) -> bool {
    let chars: Vec<char> = value.chars().collect();
    chars.len() >= 2 && chars[0] == '"' && *chars.last().unwrap() == '"'
}

/// Converts a byte offset into `line` to a 1-based character column.
fn char_col(line: &str, byte_idx: usize) -> usize {
    line[..byte_idx].chars().count() + 1
}
