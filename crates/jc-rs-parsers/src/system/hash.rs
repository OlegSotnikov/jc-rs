//! Parser for `hash` shell builtin output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::simple_table_parse;
use serde_json::{Map, Value};

pub struct HashParser;

static INFO: ParserInfo = ParserInfo {
    name: "hash",
    argument: "--hash",
    version: "1.4.0",
    description: "Converts `hash` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HASH_PARSER: HashParser = HashParser;

inventory::submit! {
    ParserEntry::new(&HASH_PARSER)
}

impl Parser for HashParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Normalize header to lowercase
        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let header = lines[0].to_lowercase();
        let rest = lines[1..].join("\n");
        let normalized = format!("{}\n{}", header, rest);

        let rows = simple_table_parse(&normalized);

        let result = rows
            .into_iter()
            .map(|row| {
                let mut out = Map::new();

                if let Some(Value::String(s)) = row.get("hits") {
                    let trimmed = s.trim();
                    if let Ok(n) = trimmed.parse::<i64>() {
                        out.insert("hits".to_string(), Value::Number(n.into()));
                    } else {
                        out.insert("hits".to_string(), Value::String(trimmed.to_string()));
                    }
                }

                if let Some(Value::String(s)) = row.get("command") {
                    out.insert("command".to_string(), Value::String(s.trim().to_string()));
                }

                out
            })
            .collect();

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_basic() {
        let input = "hits\tcommand\n   3\t/usr/bin/sum\n   2\t/usr/bin/cat\n   2\t/usr/bin/cksum\n";
        let parser = HashParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0].get("hits"), Some(&Value::Number(3.into())));
            assert_eq!(
                arr[0].get("command"),
                Some(&Value::String("/usr/bin/sum".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_hash_empty() {
        let parser = HashParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
