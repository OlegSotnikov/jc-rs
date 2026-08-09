//! Parser for `findmnt` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct FindmntParser;

static INFO: ParserInfo = ParserInfo {
    name: "findmnt",
    argument: "--findmnt",
    version: "1.0.0",
    description: "Converts `findmnt` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["findmnt"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static FINDMNT_PARSER: FindmntParser = FindmntParser;

inventory::submit! {
    ParserEntry::new(&FINDMNT_PARSER)
}

impl Parser for FindmntParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_findmnt(input);
        Ok(ParseOutput::Array(rows))
    }
}

/// Remove tree drawing characters from a string (for TARGET column).
fn strip_tree_drawing(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;

    // Skip leading tree chars and whitespace until we hit a '/' or alphanumeric
    while i < len {
        let ch = chars[i];
        match ch {
            '├' | '└' | '│' | '─' | '|' | '`' | '-' | ' ' => {
                i += 1;
            }
            _ => break,
        }
    }

    while i < len {
        result.push(chars[i]);
        i += 1;
    }

    result.trim().to_string()
}

fn parse_findmnt(input: &str) -> Vec<Map<String, Value>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.is_empty() {
        return Vec::new();
    }

    let header_line = lines[0];
    let headers: Vec<&str> = header_line.split_whitespace().collect();
    if headers.is_empty() {
        return Vec::new();
    }

    // Find column positions from header (header is ASCII so byte == char position)
    let mut col_starts: Vec<usize> = Vec::new(); // char positions
    let mut col_names: Vec<String> = Vec::new();

    let mut search_from = 0;
    for h in &headers {
        if let Some(pos) = header_line[search_from..].find(h) {
            // convert byte offset in header to char index (header is ASCII so equal)
            let byte_pos = search_from + pos;
            col_starts.push(byte_pos); // byte_pos == char_idx for ASCII header
            col_names.push(h.to_lowercase());
            search_from = byte_pos + h.len();
        }
    }

    let n = col_starts.len();
    let mut results = Vec::new();

    for &line in &lines[1..] {
        if line.trim().is_empty() {
            continue;
        }

        // Build a char-index → byte-offset map for this line (handles multi-byte Unicode)
        let char_to_byte: Vec<usize> = line
            .char_indices()
            .map(|(byte_offset, _)| byte_offset)
            .collect();
        let total_chars = char_to_byte.len();

        let mut record = Map::new();

        for i in 0..n {
            let char_start = col_starts[i];
            let char_end = if i + 1 < n {
                col_starts[i + 1]
            } else {
                total_chars + 1
            };

            // Convert char positions to byte positions for this line
            let byte_start = if char_start < total_chars {
                char_to_byte[char_start]
            } else {
                line.len()
            };
            let byte_end = if char_end < total_chars {
                char_to_byte[char_end]
            } else {
                line.len()
            };

            let val = if byte_start < line.len() {
                line[byte_start..byte_end].trim()
            } else {
                ""
            };

            let key = &col_names[i];

            let val = if key == "target" {
                strip_tree_drawing(val)
            } else {
                val.to_string()
            };

            if val.is_empty() {
                record.insert(key.clone(), Value::Null);
                continue;
            }

            // Process OPTIONS column
            if key == "options" {
                let (opts, kv_opts) = parse_mount_options(&val);
                record.insert(
                    "options".to_string(),
                    Value::Array(opts.into_iter().map(Value::String).collect()),
                );
                if !kv_opts.is_empty() {
                    let mut m = Map::new();
                    for (k, v) in kv_opts {
                        m.insert(k, Value::String(v));
                    }
                    record.insert("kv_options".to_string(), Value::Object(m));
                }
            } else {
                record.insert(key.clone(), Value::String(val));
            }
        }

        results.push(record);
    }

    results
}

/// Split mount options string into regular options and key=value pairs.
fn parse_mount_options(opts_str: &str) -> (Vec<String>, Vec<(String, String)>) {
    let mut options = Vec::new();
    let mut kv_options = Vec::new();

    for opt in opts_str.split(',') {
        let opt = opt.trim();
        if opt.is_empty() {
            continue;
        }
        if let Some(eq_pos) = opt.find('=') {
            let key = opt[..eq_pos].to_string();
            let val = opt[eq_pos + 1..].to_string();
            kv_options.push((key, val));
        } else {
            options.push(opt.to_string());
        }
    }

    (options, kv_options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_findmnt_basic() {
        let input = "TARGET        SOURCE    FSTYPE OPTIONS\n\
                      /             /dev/sda1 ext4   rw,relatime,data=ordered\n\
                      └─/boot       /dev/sda2 ext2   rw,nosuid\n";

        let parser = FindmntParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0]["target"], Value::String("/".into()));
            assert_eq!(arr[1]["target"], Value::String("/boot".into()));
            // Check options parsing
            assert!(arr[0]["options"].is_array());
            assert!(arr[0]["kv_options"].is_object());
        } else {
            panic!("expected array");
        }
    }

    #[test]
    fn test_parse_mount_options() {
        let (opts, kv) = parse_mount_options("rw,nosuid,relatime,data=ordered,seclabel");
        assert_eq!(opts, vec!["rw", "nosuid", "relatime", "seclabel"]);
        assert_eq!(kv, vec![("data".to_string(), "ordered".to_string())]);
    }

    #[test]
    fn test_strip_tree_drawing() {
        assert_eq!(strip_tree_drawing("├─/boot"), "/boot");
        assert_eq!(strip_tree_drawing("└─/home"), "/home");
        assert_eq!(strip_tree_drawing("│ └─/var"), "/var");
        assert_eq!(strip_tree_drawing("/"), "/");
    }
}
