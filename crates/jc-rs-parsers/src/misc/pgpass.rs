use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct PgpassParser;

static INFO: ParserInfo = ParserInfo {
    name: "pgpass",
    argument: "--pgpass",
    version: "1.0.0",
    description: "PostgreSQL password file parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PGPASS_PARSER: PgpassParser = PgpassParser;

inventory::submit! {
    ParserEntry::new(&PGPASS_PARSER)
}

// Unicode invisible separator used for field splitting (same as Python jc)
const INVIS_SEP: char = '\u{2063}';

/// Parse a pgpass line into 5 fields, handling escaped colons and backslashes.
/// Python algorithm:
///   1. replace ':' with INVIS_SEP
///   2. replace '\\\\' with '\\'
///   3. replace '\\INVIS_SEP' with ':'
fn split_pgpass_line(line: &str) -> Option<[String; 5]> {
    let step1 = line.replace(':', &INVIS_SEP.to_string());
    let step2 = step1.replace("\\\\", "\\");
    let step3 = step2.replace(&format!("\\{}", INVIS_SEP), ":");
    let parts: Vec<&str> = step3.splitn(5, INVIS_SEP).collect();
    if parts.len() != 5 {
        return None;
    }
    Some([
        parts[0].to_string(),
        parts[1].to_string(),
        parts[2].to_string(),
        parts[3].to_string(),
        parts[4].to_string(),
    ])
}

impl Parser for PgpassParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some([hostname, port, database, username, password]) = split_pgpass_line(line) {
                let mut obj = Map::new();
                obj.insert("hostname".to_string(), Value::String(hostname));
                obj.insert("port".to_string(), Value::String(port));
                obj.insert("database".to_string(), Value::String(database));
                obj.insert("username".to_string(), Value::String(username));
                obj.insert("password".to_string(), Value::String(password));
                result.push(obj);
            }
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pgpass_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/pgpass.txt");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/pgpass.json"
        ))
        .unwrap();
        let parser = PgpassParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}
