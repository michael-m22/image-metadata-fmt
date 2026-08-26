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

fn normalize_key(raw: &str) -> String {
    raw.trim().to_lowercase().replace([' ', '-'], "_")
}

fn normalize_scalar(raw: &str) -> String {
    format!("\"{}\"", escape_newlines(strip_quotes(raw)))
}

fn strip_quotes(value: &str) -> &str {
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
/// string rather than silently guessed at.
fn normalize_date(raw: &str) -> String {
    let value = strip_quotes(raw.trim());
    let parts: Vec<&str> = value.split(['-', '/', '.']).collect();
    if parts.len() != 3 {
        return format!("\"{}\"", escape_newlines(value));
    }

    let year = parts[0];
    let month: u32 = match parts[1].parse() {
        Ok(m) if (1..=12).contains(&m) => m,
        _ => return format!("\"{}\"", escape_newlines(value)),
    };
    let day: u32 = match parts[2].parse() {
        Ok(d) if (1..=31).contains(&d) => d,
        _ => return format!("\"{}\"", escape_newlines(value)),
    };

    format!("{}-{:02}-{:02}", year, month, day)
}
