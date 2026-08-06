//! Table parsing utilities, ported from jc's parsers/universal.py.

use serde_json::Value;
use std::collections::HashMap;

/// Parse a simple whitespace-delimited table.
///
/// The first line is the header row. Remaining lines are data rows.
/// The last column captures any remaining text (including spaces).
/// Missing values (shorter rows) are represented as empty strings.
///
/// Mirrors jc's `simple_table_parse`.
pub fn simple_table_parse(data: &str) -> Vec<HashMap<String, Value>> {
    let mut lines = data.lines();

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    // Parse headers: normalize whitespace
    let headers: Vec<String> = header_line
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    if headers.is_empty() {
        return Vec::new();
    }

    let ncols = headers.len();
    let mut output = Vec::with_capacity(count_lines(data).saturating_sub(1));
    // One splitting buffer for the whole table rather than one per row.
    let mut parts: Vec<&str> = Vec::with_capacity(ncols);

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        split_whitespace_n_into(line, ncols, &mut parts);

        let mut record = HashMap::with_capacity(ncols);
        for (i, header) in headers.iter().enumerate() {
            let value = parts.get(i).copied().unwrap_or("").trim().to_string();
            record.insert(header.clone(), Value::String(value));
        }
        output.push(record);
    }

    output
}

/// Number of lines `str::lines` will yield, without materialising any of them.
fn count_lines(data: &str) -> usize {
    if data.is_empty() {
        return 0;
    }
    let newlines = data.as_bytes().iter().filter(|&&b| b == b'\n').count();
    if data.ends_with('\n') {
        newlines
    } else {
        newlines + 1
    }
}

/// Split on whitespace up to `n` parts (last part captures the remainder including spaces),
/// reusing `parts` as scratch so a table costs one allocation rather than one per row.
fn split_whitespace_n_into<'a>(s: &'a str, n: usize, parts: &mut Vec<&'a str>) {
    parts.clear();
    if n == 0 {
        return;
    }
    let mut remaining = s.trim_start();

    for i in 0..n {
        if remaining.is_empty() {
            break;
        }
        if i == n - 1 {
            // Last column: take the rest
            parts.push(remaining);
            break;
        }
        // Find end of current token
        let token_end = remaining
            .find(char::is_whitespace)
            .unwrap_or(remaining.len());
        parts.push(&remaining[..token_end]);
        remaining = remaining[token_end..].trim_start();
    }
}

/// Parse a sparse table where column positions are determined by header positions.
///
/// Columns may be empty (represented as `Value::Null`). Each value spans from
/// its column's start position to the next column's start position.
///
/// Mirrors jc's `sparse_table_parse`.
pub fn sparse_table_parse(data: &str) -> Vec<HashMap<String, Value>> {
    let mut raw_lines = data.lines();
    let Some(header_line) = raw_lines.next() else {
        return Vec::new();
    };

    // Every line is padded to the longest one. Padding the *header* cannot move
    // any column boundary — the positions searched for all lie inside the real
    // text — so only the data rows need it, and only conceptually: a position
    // past the end of a short line is a space, which the scan below knows.
    let max_len = data.lines().map(|l| l.len()).max().unwrap_or(0);

    // The one line that still gets a materialised padded copy, so the column
    // offsets below are byte-for-byte what the padded form produced.
    let header_text = format!("{:<width$} ", header_line, width = max_len);
    let header_list: Vec<&str> = header_text.split_whitespace().collect();

    if header_list.is_empty() {
        return Vec::new();
    }

    // Build header specs: name → end position (where next column starts)
    let mut header_spec_list: Vec<(&str, usize)> = Vec::with_capacity(header_list.len());

    let mut search = String::new();
    for i in 0..header_list.len().saturating_sub(1) {
        let next_header = header_list[i + 1];
        // Find position of " <next_header> " in header_text
        search.clear();
        search.push(' ');
        search.push_str(next_header);
        search.push(' ');
        let end_pos = header_text.find(&search).unwrap_or(header_text.len());
        header_spec_list.push((header_list[i], end_pos));
    }

    let ncols = header_list.len();
    let mut output = Vec::with_capacity(count_lines(data).saturating_sub(1));

    // Use the invisible separator technique (U+2063)
    const DELIM: char = '\u{2063}';

    // Scratch reused for every row: the padded row, and the field being trimmed.
    let mut chars: Vec<char> = Vec::with_capacity(max_len);
    let mut field = String::with_capacity(max_len);

    for line in raw_lines {
        chars.clear();
        chars.extend(line.chars());
        // The padding the old code produced with `format!` — as spaces in the
        // buffer we already own. Every line has the same length in chars
        // afterwards, because `max_len` counts bytes and so bounds every line's
        // character count.
        if chars.len() < max_len {
            chars.resize(max_len, ' ');
        }
        let char_len = chars.len();

        // Process columns in reverse, insert delimiter at column boundaries
        for &(_col_name, h_end) in header_spec_list.iter().rev() {
            let mut pos = h_end.min(char_len.saturating_sub(1));
            // Walk left until we find whitespace
            while pos > 0 && !chars[pos].is_whitespace() {
                pos -= 1;
            }
            if pos < char_len {
                chars[pos] = DELIM;
            }
        }

        // Walk the delimited row directly instead of rebuilding it as a String
        // and re-splitting that. `splitn` semantics: the last field keeps any
        // further delimiters.
        let mut record = HashMap::with_capacity(ncols);
        let mut start = 0;
        let mut col = 0;
        for pos in 0..=char_len {
            let last_field = col + 1 == ncols;
            let at_end = pos == char_len;
            if !at_end && (last_field || chars[pos] != DELIM) {
                continue;
            }
            field.clear();
            field.extend(&chars[start..pos]);
            insert_sparse_cell(&mut record, header_list[col], field.trim());
            start = pos + 1;
            col += 1;
            if at_end {
                break;
            }
        }
        // Columns the row ran out of characters for.
        for header in &header_list[col.min(ncols)..] {
            record.insert((*header).to_string(), Value::Null);
        }
        output.push(record);
    }

    output
}

fn insert_sparse_cell(record: &mut HashMap<String, Value>, header: &str, val: &str) {
    let cell = if val.is_empty() {
        Value::Null
    } else {
        Value::String(val.to_string())
    };
    record.insert(header.to_string(), cell);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_table_parse_basic() {
        let data = "name       pid   cpu\nfoo        123   0.5\nbar        456   1.0";
        let result = simple_table_parse(data);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].get("name"),
            Some(&Value::String("foo".to_string()))
        );
        assert_eq!(
            result[0].get("pid"),
            Some(&Value::String("123".to_string()))
        );
        assert_eq!(
            result[0].get("cpu"),
            Some(&Value::String("0.5".to_string()))
        );
    }

    #[test]
    fn test_simple_table_last_column_with_spaces() {
        let data = "col_1     col_2     col_5\napple     orange    my favorite fruits\ncarrot    squash    my favorite veggies";
        let result = simple_table_parse(data);
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0].get("col_5"),
            Some(&Value::String("my favorite fruits".to_string()))
        );
        assert_eq!(
            result[1].get("col_5"),
            Some(&Value::String("my favorite veggies".to_string()))
        );
    }

    #[test]
    fn test_simple_table_empty() {
        assert_eq!(simple_table_parse(""), Vec::new());
    }

    #[test]
    fn test_simple_table_short_rows() {
        let data = "a  b  c\n1  2";
        let result = simple_table_parse(data);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].get("a"), Some(&Value::String("1".to_string())));
        assert_eq!(result[0].get("b"), Some(&Value::String("2".to_string())));
        assert_eq!(result[0].get("c"), Some(&Value::String("".to_string())));
    }
}
