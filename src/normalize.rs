use crate::parser::RawEntry;

/// Turns raw entries into the canonical text form: lower_snake_case keys,
/// quoted scalars, bracketed lists, and zero-padded dates.
pub fn normalize(entries: &[RawEntry]) -> String {
    let mut out = String::new();

    for entry in entries {
        let key = normalize_key(&entry.key);
        let line = match key.as_str() {
            "tags" | "keywords" => format!("tags = {}", normalize_list(&entry.value)),
            "date_taken" | "date" => format!("date_taken = {}", normalize_date(&entry.value)),
            _ => format!("{} = {}", key, normalize_scalar(&entry.value)),
        };
        out.push_str(&line);
        out.push('\n');
    }

    out
}

pub(crate) fn normalize_key(raw: &str) -> String {
    raw.trim().to_lowercase().replace([' ', '-'], "_")
}

fn normalize_scalar(raw: &str) -> String {
    format!("\"{}\"", escape_newlines(strip_quotes(raw)))
}

pub(crate) fn strip_quotes(value: &str) -> &str {
    let trimmed = value.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

// Values that spanned multiple lines in the source carry real `\n`
// characters; the canonical form stays one line per entry, so those get
// written back out as the two-character escape instead.
fn escape_newlines(value: &str) -> String {
    value.replace('\n', "\\n")
}

fn normalize_list(raw: &str) -> String {
    let items: Vec<String> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| format!("\"{}\"", escape_newlines(strip_quotes(s))))
        .collect();
    format!("[{}]", items.join(", "))
}

/// Accepts `YYYY-M-D`, `YYYY/M/D`, or `YYYY.M.D` and zero-pads month/day.
/// Anything that doesn't parse as three numeric parts is left as a quoted
/// string rather than silently guessed at. In the normal CLI flow, schema
/// validation rejects unparseable dates before this ever runs, but this
/// stays defensive for direct callers.
fn normalize_date(raw: &str) -> String {
    let value = strip_quotes(raw.trim());
    match parse_date_parts(value) {
        Some((year, month, day)) => format!("{}-{:02}-{:02}", year, month, day),
        None => format!("\"{}\"", escape_newlines(value)),
    }
}

/// Splits a date value into (year, month, day) if it's three numeric parts
/// separated by `-`, `/`, or `.`, with month and day in range. Shared with
/// schema validation so "what counts as a valid date" is defined in one
/// place.
pub(crate) fn parse_date_parts(value: &str) -> Option<(&str, u32, u32)> {
    let parts: Vec<&str> = value.split(['-', '/', '.']).collect();
    if parts.len() != 3 {
        return None;
    }

    let year = parts[0];
    if year.is_empty() || !year.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let month: u32 = parts[1].parse().ok()?;
    let day: u32 = parts[2].parse().ok()?;
    if (1..=12).contains(&month) && (1..=31).contains(&day) {
        Some((year, month, day))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::RawEntry;

    fn entry(key: &str, value: &str) -> RawEntry {
        RawEntry {
            key: key.to_string(),
            value: value.to_string(),
            line: 1,
            value_col: 1,
        }
    }

    #[test]
    fn normalize_key_lowercases_and_joins_separators() {
        assert_eq!(normalize_key("Date Taken"), "date_taken");
        assert_eq!(normalize_key("date-taken"), "date_taken");
        assert_eq!(normalize_key("DATE_TAKEN"), "date_taken");
    }

    #[test]
    fn strip_quotes_removes_matching_quotes_only() {
        assert_eq!(strip_quotes("\"mike\""), "mike");
        assert_eq!(strip_quotes("mike"), "mike");
        assert_eq!(strip_quotes("\"unbalanced"), "\"unbalanced");
    }

    #[test]
    fn parse_date_parts_accepts_any_of_the_three_separators() {
        assert_eq!(parse_date_parts("2023-3-4"), Some(("2023", 3, 4)));
        assert_eq!(parse_date_parts("2023/3/4"), Some(("2023", 3, 4)));
        assert_eq!(parse_date_parts("2023.3.4"), Some(("2023", 3, 4)));
    }

    #[test]
    fn parse_date_parts_rejects_out_of_range_month_or_day() {
        assert_eq!(parse_date_parts("2023-13-4"), None);
        assert_eq!(parse_date_parts("2023-3-32"), None);
    }

    #[test]
    fn parse_date_parts_rejects_non_numeric_year() {
        assert_eq!(parse_date_parts("unknown-3-4"), None);
    }

    #[test]
    fn normalize_scalar_quotes_and_escapes_embedded_newline() {
        let out = normalize(&[entry("artist", "mike")]);
        assert_eq!(out, "artist = \"mike\"\n");

        let out = normalize(&[entry("notes", "\"line one\nline two\"")]);
        assert_eq!(out, "notes = \"line one\\nline two\"\n");
    }

    #[test]
    fn normalize_list_trims_items_and_drops_trailing_comma() {
        let out = normalize(&[entry("tags", "beach, sunset,  golden hour,")]);
        assert_eq!(out, "tags = [\"beach\", \"sunset\", \"golden hour\"]\n");
    }

    #[test]
    fn keywords_alias_normalizes_to_tags_key() {
        let out = normalize(&[entry("keywords", "beach")]);
        assert_eq!(out, "tags = [\"beach\"]\n");
    }

    #[test]
    fn normalize_date_zero_pads_month_and_day() {
        let out = normalize(&[entry("date_taken", "2023-3-4")]);
        assert_eq!(out, "date_taken = 2023-03-04\n");
    }

    #[test]
    fn date_alias_normalizes_to_date_taken_key() {
        let out = normalize(&[entry("date", "2023/3/4")]);
        assert_eq!(out, "date_taken = 2023-03-04\n");
    }

    #[test]
    fn unparseable_date_falls_back_to_quoted_string() {
        let out = normalize(&[entry("date_taken", "next tuesday")]);
        assert_eq!(out, "date_taken = \"next tuesday\"\n");
    }

    #[test]
    fn multiple_entries_produce_one_line_each_in_order() {
        let out = normalize(&[entry("title", "sunset"), entry("artist", "mike")]);
        assert_eq!(out, "title = \"sunset\"\nartist = \"mike\"\n");
    }
}
