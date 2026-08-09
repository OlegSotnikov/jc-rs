//! Parser for `lsblk` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LsblkParser;

static INFO: ParserInfo = ParserInfo {
    name: "lsblk",
    argument: "--lsblk",
    version: "1.0.0",
    description: "Converts `lsblk` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["lsblk"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSBLK_PARSER: LsblkParser = LsblkParser;

inventory::submit! {
    ParserEntry::new(&LSBLK_PARSER)
}

impl Parser for LsblkParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_lsblk(input);
        Ok(ParseOutput::Array(rows))
    }
}

const BOOL_FIELDS: &[&str] = &["rm", "ro", "rota", "disc_zero", "rand"];
const INT_FIELDS: &[&str] = &[
    "ra",
    "alignment",
    "min_io",
    "opt_io",
    "phy_sec",
    "log_sec",
    "rq_size",
    "disc_aln",
];
/// Fields that get a corresponding `_bytes` sibling via size conversion.
const SIZE_FIELDS: &[&str] = &["size", "disc_gran", "disc_max", "wsame"];

/// Convert a human-readable size string to bytes using POSIX mode (single-letter = binary).
/// Mirrors jc's convert_size_to_int(s, posix_mode=True).
fn convert_size_to_bytes(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.is_empty() || s == "0" {
        // bare "0" with no unit → 0 bytes
        if s == "0" {
            return Some(0);
        }
        return None;
    }

    // Split into numeric part and unit part
    let split_pos = s.find(|c: char| c.is_alphabetic())?;
    let num_str = &s[..split_pos];
    let unit = s[split_pos..].trim();

    let num: f64 = num_str.trim().parse().ok()?;

    // Normalize unit: single-letter = binary (posix_mode), add 'ib'
    let unit_lower = unit.to_lowercase();
    let multiplier: f64 = if unit_lower == "b" || unit_lower.is_empty() {
        1.0
    } else if unit_lower.len() == 1 {
        // Single-letter posix: K=1024, M=1024^2, G=1024^3, T=1024^4, P=1024^5, E=1024^6
        match unit_lower.as_str() {
            "k" => 1024.0_f64.powi(1),
            "m" => 1024.0_f64.powi(2),
            "g" => 1024.0_f64.powi(3),
            "t" => 1024.0_f64.powi(4),
            "p" => 1024.0_f64.powi(5),
            "e" => 1024.0_f64.powi(6),
            _ => return None,
        }
    } else if unit_lower.ends_with("ib") {
        // KiB, MiB, GiB etc. = binary
        match unit_lower.trim_end_matches("ib") {
            "k" | "ki" => 1024.0_f64.powi(1),
            "m" | "mi" => 1024.0_f64.powi(2),
            "g" | "gi" => 1024.0_f64.powi(3),
            "t" | "ti" => 1024.0_f64.powi(4),
            "p" | "pi" => 1024.0_f64.powi(5),
            "e" | "ei" => 1024.0_f64.powi(6),
            _ => return None,
        }
    } else {
        // KB, MB, GB etc. = decimal
        match unit_lower.trim_end_matches('b') {
            "k" | "kilo" => 1000.0_f64.powi(1),
            "m" | "mega" => 1000.0_f64.powi(2),
            "g" | "giga" => 1000.0_f64.powi(3),
            "t" | "tera" => 1000.0_f64.powi(4),
            "p" | "peta" => 1000.0_f64.powi(5),
            "e" | "exa" => 1000.0_f64.powi(6),
            _ => return None,
        }
    };

    Some((num * multiplier) as i64)
}

/// Strip tree drawing characters from the name column value.
fn strip_tree_chars(s: &str) -> String {
    let s = s.trim();
    // Remove leading tree characters: ├─, └─, |-,  `- , │, |
    let mut result = s.to_string();
    loop {
        let trimmed = result.trim_start();
        if trimmed.starts_with("├─")
            || trimmed.starts_with("└─")
            || trimmed.starts_with("\u{251c}\u{2500}")
            || trimmed.starts_with("\u{2514}\u{2500}")
        {
            result = trimmed[6..].to_string(); // UTF-8: each box char is 3 bytes
        } else if trimmed.starts_with("|-") {
            result = trimmed[2..].to_string();
        } else if trimmed.starts_with("`-") {
            result = trimmed[2..].to_string();
        } else if trimmed.starts_with("│") || trimmed.starts_with("\u{2502}") {
            result = trimmed[3..].to_string(); // UTF-8: 3 bytes
        } else if trimmed.starts_with('|') {
            result = trimmed[1..].to_string();
        } else {
            break;
        }
    }
    result.trim().to_string()
}

fn normalize_header(h: &str) -> String {
    h.to_lowercase().replace([':', '-'], "_")
}

/// Adjust column end position using character indices (move left until whitespace).
fn adjust_col_end_chars(chars: &[char], end: usize) -> usize {
    let mut h_end = end;
    while h_end > 0 && h_end < chars.len() && !chars[h_end].is_whitespace() {
        h_end -= 1;
    }
    h_end
}

fn parse_lsblk(input: &str) -> Vec<Map<String, Value>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let header_line = lines[0];

    // Normalize header like jc: lowercase, replace ':' and '-' with '_'
    let normalized_header: String = header_line.to_lowercase().replace([':', '-'], "_");
    let _norm_header_chars: Vec<char> = normalized_header.chars().collect();

    // Build column names and col_ends from the normalized header.
    // col_ends[i] = position of " next_col_name " in the normalized header (= col_starts[i+1]-1).
    // This mirrors Python's sparse_table_parse: `header_text.find(' ' + header_list[i+1] + ' ')`.
    let headers: Vec<&str> = header_line.split_whitespace().collect();
    if headers.is_empty() {
        return Vec::new();
    }

    let col_names: Vec<String> = headers.iter().map(|h| normalize_header(h)).collect();
    let n = col_names.len();

    // Compute col_ends[i] for i in 0..n-1 by searching " col_names[i+1] " in normalized header.
    let norm_header_with_space = format!("{} ", normalized_header);
    let mut col_ends: Vec<usize> = Vec::new();
    for i in 0..n - 1 {
        let search = format!(" {} ", col_names[i + 1]);
        let pos = if let Some(p) = norm_header_with_space.find(&search) {
            p
        } else {
            norm_header_with_space.len()
        };
        col_ends.push(pos);
    }

    let mut results = Vec::new();

    for &line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        // Pad line to match header length for consistent column extraction
        let max_len = norm_header_with_space.len();
        let line_padded: String = if line.chars().count() < max_len {
            let pad = max_len - line.chars().count();
            format!("{}{}", line, " ".repeat(pad))
        } else {
            line.to_string()
        };
        let line_chars: Vec<char> = line_padded.chars().collect();
        let line_char_len = line_chars.len();
        let mut record = Map::new();

        let mut prev = 0usize;
        for i in 0..n {
            let col_end = if i < n - 1 {
                col_ends[i].min(line_char_len)
            } else {
                line_char_len
            };

            // Apply move-left adjustment per row (like Python's sparse_table_parse)
            let adjusted_end = if i < n - 1 {
                adjust_col_end_chars(&line_chars, col_end)
            } else {
                col_end
            };

            let val = if prev < line_char_len {
                let chunk: String = line_chars[prev..adjusted_end.min(line_char_len)]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string();
                chunk
            } else {
                String::new()
            };

            prev = adjusted_end;

            let key = &col_names[i];

            let val = if key == "name" {
                strip_tree_chars(&val)
            } else {
                val
            };

            if val.is_empty() {
                record.insert(key.clone(), Value::Null);
                continue;
            }

            if BOOL_FIELDS.contains(&key.as_str()) {
                match val.as_str() {
                    "0" => record.insert(key.clone(), Value::Bool(false)),
                    "1" => record.insert(key.clone(), Value::Bool(true)),
                    _ => record.insert(key.clone(), Value::String(val)),
                };
            } else if INT_FIELDS.contains(&key.as_str()) {
                if let Ok(n) = val.parse::<i64>() {
                    record.insert(key.clone(), Value::Number(n.into()));
                } else {
                    record.insert(key.clone(), Value::String(val));
                }
            } else if SIZE_FIELDS.contains(&key.as_str()) {
                // Insert the size field AND a _bytes sibling
                let bytes_key = format!("{}_bytes", key);
                let bytes_val = convert_size_to_bytes(&val)
                    .map(|b| Value::Number(b.into()))
                    .unwrap_or(Value::Null);
                record.insert(key.clone(), Value::String(val));
                record.insert(bytes_key, bytes_val);
            } else {
                record.insert(key.clone(), Value::String(val));
            }
        }

        results.push(record);
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsblk_basic() {
        let input = "NAME   MAJ:MIN RM  SIZE RO TYPE MOUNTPOINT\n\
                      sda      8:0    0   50G  0 disk \n\
                      ├─sda1   8:1    0    1G  0 part /boot\n\
                      └─sda2   8:2    0   49G  0 part /\n";

        let parser = LsblkParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0]["name"], Value::String("sda".into()));
            assert_eq!(arr[0]["ro"], Value::Bool(false));
            assert_eq!(arr[1]["name"], Value::String("sda1".into()));
            assert_eq!(arr[2]["name"], Value::String("sda2".into()));
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_strip_tree_chars() {
        assert_eq!(strip_tree_chars("├─sda1"), "sda1");
        assert_eq!(strip_tree_chars("└─sda2"), "sda2");
        assert_eq!(strip_tree_chars("|-sda1"), "sda1");
        assert_eq!(strip_tree_chars("`-sda2"), "sda2");
        assert_eq!(strip_tree_chars("sda"), "sda");
    }
}
