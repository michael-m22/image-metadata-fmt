use crate::error::{ParseError, Position};
use crate::normalize::{normalize_key, parse_date_parts, strip_quotes};
use crate::parser::RawEntry;

/// The shape a known field's value is expected to have. Fields not listed
/// in `field_type` are freeform strings, so nothing about their value can
/// fail validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    String,
    List,
    Date,
}

/// Maps a canonical (already-normalized) key to its expected type.
pub fn field_type(canonical_key: &str) -> FieldType {
    match canonical_key {
        "tags" | "keywords" => FieldType::List,
        "date_taken" | "date" => FieldType::Date,
        _ => FieldType::String,
    }
}

/// Checks every entry's value against the type implied by its key. Lists
/// and freeform strings accept anything, so `date_taken`/`date` is
/// currently the only field narrow enough to reject a value. `input` is
/// used to recover the original source line for the error display.
pub fn validate(entries: &[RawEntry], input: &str) -> Result<(), ParseError> {
    let lines: Vec<&str> = input.lines().collect();

    for entry in entries {
        let canonical_key = normalize_key(&entry.key);
        if field_type(&canonical_key) != FieldType::Date {
            continue;
        }

        let value = strip_quotes(entry.value.trim());
        if parse_date_parts(value).is_some() {
            continue;
        }

        let source_line = lines.get(entry.line - 1).copied().unwrap_or("").to_string();
        return Err(ParseError::new(
            Position { line: entry.line, col: entry.value_col },
            format!("'{}' is not a valid date, expected Y-M-D", value),
            source_line,
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn validate_str(input: &str) -> Result<(), ParseError> {
        let entries = parse(input).unwrap();
        validate(&entries, input)
    }

    #[test]
    fn accepts_loosely_formatted_valid_date() {
        assert!(validate_str("date_taken: 2023-3-4\n").is_ok());
    }

    #[test]
    fn accepts_date_key_alias() {
        assert!(validate_str("Date Taken: 2023/03/04\n").is_ok());
    }

    #[test]
    fn rejects_month_out_of_range() {
        let err = validate_str("date_taken: 2023-13-4\n").unwrap_err();
        assert_eq!(err.pos.line, 1);
    }

    #[test]
    fn rejects_non_numeric_year() {
        assert!(validate_str("date: unknown-3-4\n").is_err());
    }

    #[test]
    fn rejects_non_numeric_date() {
        assert!(validate_str("date: next tuesday\n").is_err());
    }

    #[test]
    fn error_points_at_the_value_not_the_key() {
        let err = validate_str("date_taken: 2023-13-4\n").unwrap_err();
        assert_eq!(err.pos.col, "date_taken: ".chars().count() + 1);
    }

    #[test]
    fn unknown_fields_are_never_rejected() {
        assert!(validate_str("artist: definitely not a date\n").is_ok());
    }

    #[test]
    fn tag_lists_are_never_rejected() {
        assert!(validate_str("tags: not, a, date\n").is_ok());
    }
}
