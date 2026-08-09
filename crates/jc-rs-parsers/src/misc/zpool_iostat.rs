//! Parser for `zpool iostat` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{convert_to_float, convert_to_int};
use serde_json::{Map, Value};

pub struct ZpoolIostatParser;

static INFO: ParserInfo = ParserInfo {
    name: "zpool_iostat",
    argument: "--zpool-iostat",
    version: "1.0.0",
    description: "Converts `zpool iostat` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["zpool iostat"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ZPOOL_IOSTAT_PARSER: ZpoolIostatParser = ZpoolIostatParser;

inventory::submit! {
    ParserEntry::new(&ZPOOL_IOSTAT_PARSER)
}

/// Split a value string like "2.89T" into (2.89, "T")
fn split_value_unit(s: &str) -> (f64, String) {
    if s.is_empty() {
        return (0.0, String::new());
    }
    let last_char = s.chars().last().unwrap_or('0');
    if last_char.is_ascii_alphabetic() {
        let num_str = &s[..s.len() - last_char.len_utf8()];
        let num = convert_to_float(num_str).unwrap_or(0.0);
        (num, last_char.to_string())
    } else {
        let num = convert_to_float(s).unwrap_or(0.0);
        (num, String::new())
    }
}

impl Parser for ZpoolIostatParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();
        let mut current_pool: Option<String> = None;

        for line in input.lines() {
            // Skip separator lines and header lines
            if line.contains("---")
                || line.trim().ends_with("bandwidth")
                || line.trim().ends_with("write")
            {
                continue;
            }

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() < 7 {
                continue;
            }

            let mut obj = Map::new();

            // If line starts with whitespace, it's a child (has parent)
            if line.starts_with(' ') || line.starts_with('\t') {
                obj.insert("pool".to_string(), Value::String(tokens[0].to_string()));
                if let Some(ref parent) = current_pool {
                    obj.insert("parent".to_string(), Value::String(parent.clone()));
                }
            } else {
                current_pool = Some(tokens[0].to_string());
                obj.insert("pool".to_string(), Value::String(tokens[0].to_string()));
            }

            // cap_alloc (tokens[1])
            let (cap_alloc_val, cap_alloc_unit) = split_value_unit(tokens[1]);
            obj.insert(
                "cap_alloc".to_string(),
                serde_json::Number::from_f64(cap_alloc_val)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );
            obj.insert(
                "cap_free".to_string(),
                serde_json::Number::from_f64({
                    let (v, _) = split_value_unit(tokens[2]);
                    v
                })
                .map(Value::Number)
                .unwrap_or(Value::Null),
            );

            let cap_free_unit = {
                let (_, u) = split_value_unit(tokens[2]);
                u
            };

            // ops_read (tokens[3]) -> integer
            let ops_read = convert_to_int(tokens[3])
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("ops_read".to_string(), ops_read);

            // ops_write (tokens[4]) -> integer
            let ops_write = convert_to_int(tokens[4])
                .map(Value::from)
                .unwrap_or(Value::Null);
            obj.insert("ops_write".to_string(), ops_write);

            // bw_read (tokens[5])
            let (bw_read_val, bw_read_unit) = split_value_unit(tokens[5]);
            obj.insert(
                "bw_read".to_string(),
                serde_json::Number::from_f64(bw_read_val)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );

            // bw_write (tokens[6])
            let (bw_write_val, bw_write_unit) = split_value_unit(tokens[6]);
            obj.insert(
                "bw_write".to_string(),
                serde_json::Number::from_f64(bw_write_val)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
            );

            // Unit fields
            obj.insert("cap_alloc_unit".to_string(), Value::String(cap_alloc_unit));
            obj.insert("cap_free_unit".to_string(), Value::String(cap_free_unit));
            obj.insert("bw_read_unit".to_string(), Value::String(bw_read_unit));
            obj.insert("bw_write_unit".to_string(), Value::String(bw_write_unit));

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_fixture(input: &str, expected_json: &str) {
        let parser = ZpoolIostatParser;
        let result = parser.parse(input, false).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), expected.len(), "row count mismatch");
                for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                    for field in &[
                        "pool",
                        "cap_alloc",
                        "cap_free",
                        "ops_read",
                        "ops_write",
                        "bw_read",
                        "bw_write",
                        "cap_alloc_unit",
                        "cap_free_unit",
                        "bw_read_unit",
                        "bw_write_unit",
                    ] {
                        assert_eq!(
                            got.get(*field).unwrap_or(&Value::Null),
                            exp.get(*field).unwrap_or(&Value::Null),
                            "row {} field '{}' mismatch",
                            i,
                            field
                        );
                    }
                    // parent may or may not be present
                    if exp.get("parent").is_some() {
                        assert_eq!(
                            got.get("parent").unwrap_or(&Value::Null),
                            exp.get("parent").unwrap_or(&Value::Null),
                            "row {} field 'parent' mismatch",
                            i
                        );
                    }
                }
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_zpool_iostat() {
        check_fixture(
            include_str!("../../../../tests/fixtures/generic/zpool-iostat.out"),
            include_str!("../../../../tests/fixtures/generic/zpool-iostat.json"),
        );
    }

    #[test]
    fn test_zpool_iostat_v() {
        check_fixture(
            include_str!("../../../../tests/fixtures/generic/zpool-iostat-v.out"),
            include_str!("../../../../tests/fixtures/generic/zpool-iostat-v.json"),
        );
    }

    #[test]
    fn test_zpool_iostat_empty() {
        let parser = ZpoolIostatParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}
