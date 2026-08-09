//! Parser for `df` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_size_to_int;
use serde_json::{Map, Value};

pub struct DfParser;

static INFO: ParserInfo = ParserInfo {
    name: "df",
    argument: "--df",
    version: "1.1.1",
    description: "Converts `df` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["df"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static DF_PARSER: DfParser = DfParser;

inventory::submit! {
    ParserEntry::new(&DF_PARSER)
}

impl Parser for DfParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_df(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Normalize a df key after parsing.
fn normalize_df_key(k: &str) -> String {
    match k {
        "use%" => "use_percent".to_string(),
        "%iused" => "iused_percent".to_string(),
        "capacity" => "capacity_percent".to_string(),
        "avail" => "available".to_string(),
        "1k-blocks" | "1k_blocks" => "1k_blocks".to_string(),
        "1024-blocks" | "1024_blocks" => "1024_blocks".to_string(),
        "512-blocks" | "512_blocks" => "512_blocks".to_string(),
        other => other.replace('-', "_"),
    }
}

/// Sparse table parse: a direct Rust port of jc's Python sparse_table_parse.
/// Takes a list of lines (first is header, rest are data).
/// Returns rows as Vec<(column_name, value)>.
fn df_sparse_parse(lines: &[String]) -> Vec<Vec<(String, Option<String>)>> {
    if lines.is_empty() {
        return Vec::new();
    }

    // Find max line length. Only the header still gets a materialised padded
    // copy, because the column offsets are read out of it; the data rows are
    // padded in a buffer that is reused for the whole table.
    let max_len = lines.iter().map(|l| l.len()).max().unwrap_or(0);

    let header_text = format!("{:<width$} ", lines[0], width = max_len);
    let header_list: Vec<&str> = header_text.split_whitespace().collect();
    let n = header_list.len();

    if n == 0 {
        return Vec::new();
    }

    // Find end position for each column (all except the last)
    let mut col_ends: Vec<usize> = Vec::with_capacity(n.saturating_sub(1));
    let mut search = String::new();
    for h in &header_list[1..] {
        search.clear();
        search.push(' ');
        search.push_str(h);
        search.push(' ');
        col_ends.push(header_text.find(&search).unwrap_or(header_text.len()));
    }

    const DELIM: char = '\u{2063}';
    let mut output = Vec::with_capacity(lines.len().saturating_sub(1));

    let mut entry: Vec<char> = Vec::with_capacity(max_len);
    let mut field = String::with_capacity(max_len);

    for line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        entry.clear();
        entry.extend(line.chars());
        if entry.len() < max_len {
            entry.resize(max_len, ' ');
        }
        let entry_len = entry.len();

        // Insert delimiters at column boundaries (process in reverse)
        for &h_end in col_ends.iter().rev() {
            let mut pos = h_end;
            // Walk left until whitespace found
            while pos > 0 && pos < entry_len && !entry[pos].is_whitespace() {
                pos -= 1;
            }
            if pos < entry_len {
                entry[pos] = DELIM;
            }
        }

        // Walk the delimited row in place rather than rebuilding it as a
        // `String` only to split that back into slices. `splitn` semantics: the
        // last column keeps any further delimiters.
        let mut row: Vec<(String, Option<String>)> = Vec::with_capacity(n);
        let mut start = 0;
        for pos in 0..=entry_len {
            let last_col = row.len() + 1 == n;
            let at_end = pos == entry_len;
            if !at_end && (last_col || entry[pos] != DELIM) {
                continue;
            }
            field.clear();
            field.extend(&entry[start..pos]);
            let val = field.trim();
            let cell = if val.is_empty() {
                None
            } else {
                Some(val.to_string())
            };
            row.push((header_list[row.len()].to_string(), cell));
            start = pos + 1;
            if at_end {
                break;
            }
        }
        // Columns the row ran out of characters for.
        while row.len() < n {
            row.push((header_list[row.len()].to_string(), None));
        }

        output.push(row);
    }

    output
}

fn parse_df(input: &str) -> Vec<Map<String, Value>> {
    if input.trim().is_empty() {
        return Vec::new();
    }

    let raw_lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

    if raw_lines.is_empty() {
        return Vec::new();
    }

    // Process header: lowercase, replace dashes, fix "mounted on"
    // IMPORTANT: Do NOT change column widths here (don't rename use% → use_percent yet)
    let header = raw_lines[0]
        .to_lowercase()
        .replace("mounted on", "mounted_on")
        .replace('-', "_");

    // Detect POSIX mode: when "use%" is in the header, single-letter units
    // (K, M, G) in size fields should be treated as binary (1024-based).
    let posix_mode = header.contains("use%");

    // Check for long filesystem lines (filesystem name alone on a line)
    let mut lines: Vec<String> = Vec::new();
    lines.push(header.clone());

    let mut i = 1;
    while i < raw_lines.len() {
        let line = raw_lines[i];
        // A lone filesystem line has exactly 1 token (a path)
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() == 1 && i + 1 < raw_lines.len() {
            // Merge with next line
            let merged = format!("{} {}", line.trim(), raw_lines[i + 1].trim());
            lines.push(merged);
            i += 2;
        } else {
            lines.push(line.to_string());
            i += 1;
        }
    }

    // Handle filesystem names that are longer than the filesystem column
    // (jc's _long_filesystem_hash logic)
    // Find the width of the filesystem column from the header
    let fs_col_end = {
        let header_words: Vec<&str> = header.split_whitespace().collect();
        if header_words.len() >= 2 {
            // Find the position of the second column header
            let second_col = header_words[1];
            let search = format!(" {} ", second_col);
            header.find(&search).unwrap_or(header.len())
        } else {
            header.len()
        }
    };

    let mut filesystem_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let lines: Vec<String> = lines
        .into_iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 {
                return line; // skip header
            }
            let fs_field = line.split_whitespace().next().unwrap_or("").to_string();
            if fs_field.len() > fs_col_end {
                // Use truncation as a placeholder to make filesystem fit in its column
                let ph: String = fs_field.chars().take(fs_col_end).collect();
                filesystem_map.insert(ph.clone(), fs_field.clone());
                return line.replacen(&fs_field, &ph, 1);
            }
            line
        })
        .collect();

    // Now do sparse parse
    let raw_rows = df_sparse_parse(&lines);

    raw_rows
        .into_iter()
        .map(|row| {
            let mut record = Map::new();
            for (raw_key, val_opt) in row {
                let key = normalize_df_key(&raw_key);
                let converted = match val_opt {
                    None => Value::Null,
                    Some(v) => convert_df_value(&key, &v, posix_mode),
                };
                record.insert(key, converted);
            }

            // Restore original filesystem name if it was truncated
            if let Some(Value::String(fs)) = record.get("filesystem") {
                let fs = fs.clone();
                if let Some(real) = filesystem_map.get(&fs) {
                    record.insert("filesystem".to_string(), Value::String(real.clone()));
                }
            }

            record
        })
        .collect()
}

fn convert_df_value(key: &str, val: &str, posix_mode: bool) -> Value {
    let val = val.trim();
    if val.is_empty() {
        return Value::Null;
    }

    if key.ends_with("_percent") {
        let stripped = val.trim_end_matches('%').trim();
        if stripped == "-" || stripped.is_empty() {
            return Value::Null;
        }
        return stripped
            .parse::<i64>()
            .map(|n| Value::Number(n.into()))
            .unwrap_or(Value::Null);
    }

    // Block count fields: always plain integers
    let block_fields = ["1k_blocks", "1024_blocks", "512_blocks", "iused", "ifree"];
    if block_fields.contains(&key)
        && let Ok(n) = val.parse::<i64>()
    {
        return Value::Number(n.into());
    }

    // Size fields: parse as integer first, then try human-readable conversion
    let size_fields = ["size", "used", "available"];
    if size_fields.contains(&key) {
        if let Ok(n) = val.parse::<i64>() {
            return Value::Number(n.into());
        }
        // Human-readable sizes (e.g. "1.9G", "500M"): convert to bytes.
        // In posix_mode, single-letter units are binary (1024-based).
        if let Some(n) = convert_size_to_int(val, posix_mode) {
            return Value::Number(n.into());
        }
    }

    Value::String(val.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_df_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/df.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/df.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/df.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/df.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/df.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/df.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_h_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/df-h.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/df-h.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_h_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/df-h.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/df-h.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_h_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/df-h.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/df-h.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_df_long_filesystem() {
        let input = include_str!("../../../../tests/fixtures/generic/df-long-filesystem.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/df-long-filesystem.json"
        ))
        .unwrap();

        let parser = DfParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
