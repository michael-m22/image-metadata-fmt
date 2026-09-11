use crate::error::{ParseError, Position};

/// One `key: value` (or `key = value`) line, before any normalization.
#[derive(Debug, Clone)]
pub struct RawEntry {
    pub key: String,
    pub value: String,
    pub line: usize,
    /// 1-based character column where the value starts on `line`. Kept so
    /// schema validation can point at the value itself rather than the
    /// start of the line.
    pub value_col: usize,
}

/// Parses an .imeta sidecar file into raw key/value entries.
///
/// The format is deliberately loose on input (that's the whole point of the
/// formatter) but a line still has to resolve to a key and a value, and a
/// quoted value still has to be closed. Anything that fails those checks
/// gets reported with the exact line and column of the problem.
///
/// A quoted value that isn't closed on its opening line is treated as
/// spanning subsequent lines: parsing keeps consuming raw lines, trimmed,
/// until one ends in an unescaped `"`. The line breaks are preserved in the
/// value as `\n` so the normalized form still prints as one line per entry.
pub fn parse(input: &str) -> Result<Vec<RawEntry>, ParseError> {
    let lines: Vec<&str> = input.lines().collect();
    let mut entries = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let raw_line = lines[i];
        let line_no = i + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
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
        let first_part = rest.trim();
        let value_col = char_col(raw_line, value_start_byte);

        let (value, next_i) = if first_part.starts_with('"') && !is_closed_string(first_part) {
            match read_multiline_string(&lines, i, first_part) {
                Some((joined, end_idx)) => (format!("\"{}\"", joined), end_idx + 1),
                None => {
                    return Err(ParseError::new(
                        Position { line: line_no, col: value_col },
                        "unterminated string literal",
                        raw_line.to_string(),
                    ));
                }
            }
        } else {
            (first_part.to_string(), i + 1)
        };

        entries.push(RawEntry {
            key: key.to_string(),
            value,
            line: line_no,
            value_col,
        });
        i = next_i;
    }

    Ok(entries)
}

/// Given the opening line of an unclosed quoted value, scans forward through
/// `lines` for a line whose trimmed content ends in an unescaped `"`. Returns
/// the string's contents (opening and closing quotes stripped, lines joined
/// by `\n`) and the index of the closing line, or `None` if EOF is reached
/// first.
fn read_multiline_string(lines: &[&str], start_idx: usize, first_part: &str) -> Option<(String, usize)> {
    let mut joined = first_part[1..].to_string();
    let mut i = start_idx;

    loop {
        i += 1;
        if i >= lines.len() {
            return None;
        }
        let trimmed = lines[i].trim();
        joined.push('\n');
        match trimmed.strip_suffix('"') {
            Some(rest) => {
                joined.push_str(rest);
                return Some((joined, i));
            }
            None => joined.push_str(trimmed),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiline_string_joins_with_escaped_newline() {
        let input = "title: \"Sunset over\nthe bay\"\n";
        let entries = parse(input).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].value, "\"Sunset over\nthe bay\"");
        assert_eq!(entries[0].line, 1);
    }

    #[test]
    fn multiline_string_can_span_more_than_two_lines() {
        let input = "notes: \"one\ntwo\nthree\"\n";
        let entries = parse(input).unwrap();
        assert_eq!(entries[0].value, "\"one\ntwo\nthree\"");
    }

    #[test]
    fn entry_after_multiline_string_parses_from_the_right_line() {
        let input = "title: \"Sunset over\nthe bay\"\nartist: mike\n";
        let entries = parse(input).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[1].key, "artist");
        assert_eq!(entries[1].line, 3);
    }

    #[test]
    fn unterminated_multiline_string_reports_opening_position() {
        let input = "title: \"Sunset over\nthe bay\n";
        let err = parse(input).unwrap_err();
        assert_eq!(err.pos.line, 1);
        assert_eq!(err.pos.col, 8);
    }

    #[test]
    fn single_line_string_still_closes_normally() {
        let input = "title: \"Sunset over the bay\"\n";
        let entries = parse(input).unwrap();
        assert_eq!(entries[0].value, "\"Sunset over the bay\"");
    }

    #[test]
    fn missing_separator_reports_column_past_the_last_character() {
        let input = "no separator here\n";
        let err = parse(input).unwrap_err();
        assert_eq!(err.pos.line, 1);
        assert_eq!(err.pos.col, "no separator here".chars().count() + 1);
        assert!(err.message.contains("':' or '='"));
    }

    #[test]
    fn missing_separator_on_second_line_reports_that_line_number() {
        let input = "title: ok\nno separator here\n";
        let err = parse(input).unwrap_err();
        assert_eq!(err.pos.line, 2);
    }

    #[test]
    fn empty_key_before_separator_reports_column_one() {
        let input = "  : value\n";
        let err = parse(input).unwrap_err();
        assert_eq!(err.pos.line, 1);
        assert_eq!(err.pos.col, 1);
        assert!(err.message.contains("empty key"));
    }

    #[test]
    fn unterminated_string_column_is_character_based_not_byte_based() {
        // 'é' is two bytes in UTF-8; if the column were computed from byte
        // offsets instead of char offsets, this would point one column too
        // far to the right.
        let input = "café: \"unterminated\n";
        let err = parse(input).unwrap_err();
        assert_eq!(err.pos.line, 1);
        assert_eq!(err.pos.col, 7);
    }
}
