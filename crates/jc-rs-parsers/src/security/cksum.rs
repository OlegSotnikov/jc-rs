//! Parser for `cksum` and `sum` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct CksumParser;

static INFO: ParserInfo = ParserInfo {
    name: "cksum",
    argument: "--cksum",
    version: "1.4.0",
    description: "Converts `cksum` and `sum` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::Aix,
        Platform::FreeBSD,
    ],
    tags: &[Tag::Command],
    magic_commands: &["cksum", "sum"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CKSUM_PARSER: CksumParser = CksumParser;

inventory::submit! {
    ParserEntry::new(&CKSUM_PARSER)
}

impl Parser for CksumParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.splitn(3, char::is_whitespace).collect();
            if parts.len() < 3 {
                continue;
            }
            let checksum_str = parts[0];
            let blocks_str = parts[1];
            // filename is the rest after splitting on whitespace twice
            let rest = line
                .trim_start()
                .split_once(char::is_whitespace)
                .map(|x| x.1)
                .unwrap_or("")
                .trim_start()
                .split_once(char::is_whitespace)
                .map(|x| x.1)
                .unwrap_or("");

            let mut obj = Map::new();
            obj.insert("filename".to_string(), Value::String(rest.to_string()));

            if let Ok(n) = checksum_str.parse::<i64>() {
                obj.insert("checksum".to_string(), Value::Number(n.into()));
            } else {
                obj.insert(
                    "checksum".to_string(),
                    Value::String(checksum_str.to_string()),
                );
            }

            if let Ok(n) = blocks_str.parse::<i64>() {
                obj.insert("blocks".to_string(), Value::Number(n.into()));
            } else {
                obj.insert("blocks".to_string(), Value::String(blocks_str.to_string()));
            }

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cksum_basic() {
        let input = "4294967295 0 __init__.py\n2208551092 3745 airport.py";
        let parser = CksumParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("filename"),
                Some(&Value::String("__init__.py".to_string()))
            );
            assert_eq!(
                arr[0].get("checksum"),
                Some(&Value::Number(4294967295i64.into()))
            );
            assert_eq!(arr[0].get("blocks"), Some(&Value::Number(0i64.into())));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_cksum_empty() {
        let parser = CksumParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}
