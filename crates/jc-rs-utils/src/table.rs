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
    let mut output = Vec::new();

    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts = split_whitespace_n(line, ncols);

        let mut record = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            let value = parts.get(i).copied().unwrap_or("").trim().to_string();
            record.insert(header.clone(), Value::String(value));
        }
        output.push(record);
    }

    output
}

/// Split on whitespace up to `n` parts (last part captures the remainder including spaces).
fn split_whitespace_n(s: &str, n: usize) -> Vec<&str> {
    if n == 0 {
        return Vec::new();
    }
    let mut parts = Vec::new();
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

    parts
}

/// Parse a sparse table where column positions are determined by header positions.
///
/// Columns may be empty (represented as `Value::Null`). Each value spans from
/// its column's start position to the next column's start position.
///
/// Mirrors jc's `sparse_table_parse`.
pub fn sparse_table_parse(data: &str) -> Vec<HashMap<String, Value>> {
    let raw_lines: Vec<&str> = data.lines().collect();
    if raw_lines.is_empty() {
        return Vec::new();
    }

    // Pad all lines to the length of the longest line
    let max_len = raw_lines.iter().map(|l| l.len()).max().unwrap_or(0);
    let padded: Vec<String> = raw_lines
        .iter()
        .map(|l| format!("{:<width$}", l, width = max_len))
        .collect();

    let header_text = format!("{} ", &padded[0]);
    let header_list: Vec<&str> = header_text.split_whitespace().collect();

    if header_list.is_empty() {
        return Vec::new();
    }

    // Build header specs: name → end position (where next column starts)
    let mut header_spec_list: Vec<(&str, usize)> = Vec::new();

    for i in 0..header_list.len().saturating_sub(1) {
        let next_header = header_list[i + 1];
        // Find position of " <next_header> " in header_text
        let search = format!(" {} ", next_header);
        let end_pos = header_text.find(&search).unwrap_or(header_text.len());
        header_spec_list.push((header_list[i], end_pos));
    }

    let data_lines = &padded[1..];
    let mut output = Vec::new();

    // Use the invisible separator technique (U+2063)
    const DELIM: char = '\u{2063}';

    for line in data_lines {
        // Convert to a char vec once per row, do all delimiter insertions in-place,
        // then convert back. This avoids O(n²) re-scanning from chars().nth().
        let mut chars: Vec<char> = line.chars().collect();
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

        let entry: String = chars.into_iter().collect();
        let parts: Vec<&str> = entry.splitn(header_list.len(), DELIM).collect();

        let mut record = HashMap::new();
        for (i, header) in header_list.iter().enumerate() {
            let val = parts.get(i).copied().unwrap_or("").trim();
            if val.is_empty() {
                record.insert(header.to_string(), Value::Null);
            } else {
                record.insert(header.to_string(), Value::String(val.to_string()));
            }
        }
        output.push(record);
    }

    output
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
