//! Windows `dir` command parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct DirParser;

static INFO: ParserInfo = ParserInfo {
    name: "dir",
    argument: "--dir",
    version: "1.5.0",
    description: "Windows `dir` command parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["dir"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

// "03/24/2021  03:15 PM    <DIR>          filename"
// "12/07/2019  02:09 AM            54,784 filename"
static DIR_ENTRY_RE: OnceLock<Regex> = OnceLock::new();
// " Directory of C:\path\to\dir"
static DIR_OF_RE: OnceLock<Regex> = OnceLock::new();

fn get_entry_re() -> &'static Regex {
    DIR_ENTRY_RE.get_or_init(|| {
        Regex::new(
            r"^(?P<date>\d{2}/\d{2}/\d{4})\s+(?P<time>\d{2}:\d{2} (?:AM|PM))\s+(?P<size_or_dir><DIR>|[\d,]+)\s+(?P<filename>.+)$"
        ).expect("dir entry regex")
    })
}

fn get_dir_of_re() -> &'static Regex {
    DIR_OF_RE.get_or_init(|| Regex::new(r"^\s+Directory of (.+)$").expect("dir of regex"))
}

/// Strip commas from a size string and parse as i64.
fn parse_size(s: &str) -> Option<i64> {
    let clean: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    clean.parse::<i64>().ok()
}

pub fn parse_dir(input: &str) -> Vec<Map<String, Value>> {
    let mut records = Vec::new();
    let mut current_parent = String::new();

    for line in input.lines() {
        // "Directory of ..."
        if let Some(caps) = get_dir_of_re().captures(line) {
            current_parent = caps
                .get(1)
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            continue;
        }

        // File/dir entry
        let entry_re = get_entry_re();
        if let Some(caps) = entry_re.captures(line) {
            let date = caps.name("date").map(|m| m.as_str()).unwrap_or("");
            let time = caps.name("time").map(|m| m.as_str()).unwrap_or("");
            let size_or_dir = caps.name("size_or_dir").map(|m| m.as_str()).unwrap_or("");
            let filename = caps
                .name("filename")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();

            let is_dir = size_or_dir == "<DIR>";
            let size: Value = if is_dir {
                Value::Null
            } else {
                parse_size(size_or_dir)
                    .map(|n| Value::Number(n.into()))
                    .unwrap_or(Value::Null)
            };

            // Parse epoch
            let ts_str = format!("{date} {time}");
            let parsed = parse_timestamp(&ts_str, &[formats::F1600]);
            let epoch_val = match parsed.naive_epoch {
                Some(e) => Value::Number(e.into()),
                None => Value::Null,
            };

            let mut record = Map::new();
            record.insert("date".to_string(), Value::String(date.to_string()));
            record.insert("time".to_string(), Value::String(time.to_string()));
            record.insert("dir".to_string(), Value::Bool(is_dir));
            record.insert("size".to_string(), size);
            record.insert("filename".to_string(), Value::String(filename.to_string()));
            record.insert("parent".to_string(), Value::String(current_parent.clone()));
            record.insert("epoch".to_string(), epoch_val);
            records.push(record);
        }
    }

    records
}

impl Parser for DirParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Array(parse_dir(input)))
    }
}

static INSTANCE: DirParser = DirParser;

inventory::submit! {
    ParserEntry::new(&INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_dir_registered() {
        assert!(find_parser("dir").is_some());
    }

    #[test]
    fn test_dir_basic() {
        let input = " Directory of C:\\Program Files\\test\n\n03/24/2021  03:15 PM    <DIR>          .\n12/07/2019  02:09 AM            54,784 foo.exe\n";
        let records = parse_dir(input);
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["dir"], serde_json::json!(true));
        assert_eq!(records[0]["filename"], serde_json::json!("."));
        assert_eq!(
            records[0]["parent"],
            serde_json::json!("C:\\Program Files\\test")
        );
        assert!(records[0]["size"].is_null());
        assert_eq!(records[1]["dir"], serde_json::json!(false));
        assert_eq!(records[1]["size"], serde_json::json!(54784));
    }

    #[test]
    fn test_dir_fixture() {
        let input = get_fixture("windows-10/dir.out");
        let parser = find_parser("dir").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let arr = match result {
            ParseOutput::Array(v) => v,
            _ => panic!("expected array"),
        };
        assert!(!arr.is_empty());
        // First entry should be a dir
        assert_eq!(arr[0]["dir"], serde_json::json!(true));
        assert_eq!(arr[0]["filename"], serde_json::json!("."));
        assert_eq!(
            arr[0]["parent"],
            serde_json::json!("C:\\Program Files\\Internet Explorer")
        );
        // A file entry should have non-null size
        let file_entry = arr
            .iter()
            .find(|r| r["dir"] == serde_json::json!(false))
            .unwrap();
        assert!(!file_entry["size"].is_null());
        // epoch should be present
        assert!(!arr[0]["epoch"].is_null());
    }

    #[test]
    fn test_dir_multiple_dirs() {
        let input = get_fixture("windows-10/dir-dirs.out");
        let records = parse_dir(&input);
        // Should have entries from both directories
        let parents: std::collections::HashSet<String> = records
            .iter()
            .map(|r| r["parent"].as_str().unwrap_or("").to_string())
            .collect();
        assert_eq!(parents.len(), 2);
    }
}
