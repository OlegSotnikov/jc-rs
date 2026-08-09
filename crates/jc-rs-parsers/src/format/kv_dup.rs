//! Key/Value file parser with duplicate key support.
//!
//! Like kv but duplicate keys become arrays of values.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct KvDupParser;

static KV_DUP_INFO: ParserInfo = ParserInfo {
    name: "kv_dup",
    argument: "--kv-dup",
    version: "1.0.0",
    description: "Key/Value file and string parser with duplicate key support",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Remove outer quotes (single or double) from a value string.
fn remove_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

pub fn parse_kv_dup_input(
    input: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, ParseError> {
    let mut acc: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();

    for line in input.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }

        let eq_pos = trimmed.find('=');
        let colon_pos = trimmed.find(':');

        let sep_pos = match (eq_pos, colon_pos) {
            (Some(e), Some(c)) => Some(e.min(c)),
            (Some(e), None) => Some(e),
            (None, Some(c)) => Some(c),
            (None, None) => None,
        };

        if let Some(pos) = sep_pos {
            let key = trimmed[..pos].trim().to_string();
            let val = remove_quotes(trimmed[pos + 1..].trim());
            if !key.is_empty() {
                acc.entry(key).or_default().push(val);
            }
        } else {
            let key = trimmed.to_string();
            if !key.is_empty() {
                acc.entry(key).or_default().push(String::new());
            }
        }
    }

    let mut map = serde_json::Map::new();
    for (k, values) in acc {
        let arr: Vec<serde_json::Value> =
            values.into_iter().map(serde_json::Value::String).collect();
        map.insert(k, serde_json::Value::Array(arr));
    }

    Ok(map)
}

impl Parser for KvDupParser {
    fn info(&self) -> &'static ParserInfo {
        &KV_DUP_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let map = parse_kv_dup_input(input)?;
        Ok(ParseOutput::Object(map))
    }
}

static KV_DUP_PARSER_INSTANCE: KvDupParser = KvDupParser;

inventory::submit! {
    ParserEntry::new(&KV_DUP_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_dup_basic() {
        let input = "fruit=apple\nfruit=orange\ncolor=blue";
        let parser = KvDupParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(map["fruit"], serde_json::json!(["apple", "orange"]));
            assert_eq!(map["color"], serde_json::json!(["blue"]));
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_kv_dup_empty() {
        let parser = KvDupParser;
        assert!(parser.parse("", false).is_err());
    }
}
