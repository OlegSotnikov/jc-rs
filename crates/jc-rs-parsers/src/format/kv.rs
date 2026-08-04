//! Key/Value file parser.
//!
//! Parses files with simple key=value or key: value pairs.
//! No sections. Comments (#, ;) are ignored. Duplicate keys: last value wins.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

pub struct KvParser;

static KV_INFO: ParserInfo = ParserInfo {
    name: "kv",
    argument: "--kv",
    version: "1.0.0",
    description: "Key/Value file and string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
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

pub fn parse_kv_input(
    input: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, ParseError> {
    let mut map = serde_json::Map::new();

    for line in input.lines() {
        let trimmed = line.trim();

        // Skip blank lines, comments, and section headers
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        // Skip section headers (kv parser doesn't use sections)
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            continue;
        }

        // Parse key=value or key: value
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
                map.insert(key, serde_json::Value::String(val));
            }
        } else {
            // Key with no value
            let key = trimmed.to_string();
            if !key.is_empty() {
                map.insert(key, serde_json::Value::String(String::new()));
            }
        }
    }

    Ok(map)
}

impl Parser for KvParser {
    fn info(&self) -> &'static ParserInfo {
        &KV_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }
        let map = parse_kv_input(input)?;
        Ok(ParseOutput::Object(map))
    }
}

static KV_PARSER_INSTANCE: KvParser = KvParser;

inventory::submit! {
    ParserEntry::new(&KV_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_basic() {
        let input = "name = John Doe\naddress=555 California Drive\nage: 34\n; comment\noccupation:\"Engineer\"";
        let parser = KvParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(map) = result {
            assert_eq!(
                map["name"],
                serde_json::Value::String("John Doe".to_string())
            );
            assert_eq!(
                map["address"],
                serde_json::Value::String("555 California Drive".to_string())
            );
            assert_eq!(map["age"], serde_json::Value::String("34".to_string()));
            assert_eq!(
                map["occupation"],
                serde_json::Value::String("Engineer".to_string())
            );
        } else {
            panic!("expected Object output");
        }
    }

    #[test]
    fn test_kv_empty() {
        let parser = KvParser;
        assert!(parser.parse("", false).is_err());
    }
}
