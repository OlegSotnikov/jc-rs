//! Parser for `/proc/crypto`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct ProcCryptoParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_crypto",
    argument: "--proc-crypto",
    version: "1.0.0",
    description: "Converts `/proc/crypto` file to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/crypto"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static PROC_CRYPTO_PARSER: ProcCryptoParser = ProcCryptoParser;

inventory::submit! { ParserEntry::new(&PROC_CRYPTO_PARSER) }

impl Parser for ProcCryptoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut entries = Vec::new();
        let mut current = Map::new();

        for line in input.lines() {
            if line.trim().is_empty() {
                if !current.is_empty() {
                    entries.push(current);
                    current = Map::new();
                }
                continue;
            }

            // Format: "key         : value"
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_string();
                let val = line[colon_pos + 1..].trim().to_string();

                // Try to parse as integer
                if let Ok(n) = val.parse::<i64>() {
                    current.insert(key, Value::Number(n.into()));
                } else {
                    current.insert(key, Value::String(val));
                }
            }
        }

        if !current.is_empty() {
            entries.push(current);
        }

        Ok(ParseOutput::Array(entries))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_crypto() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/crypto");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/crypto.json"
        ))
        .unwrap();
        let parser = ProcCryptoParser;
        let result = parser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
