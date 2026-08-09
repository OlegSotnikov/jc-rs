//! Parser for `env` / `printenv` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

fn var_def_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*=").unwrap())
}

pub struct EnvParser;

static INFO: ParserInfo = ParserInfo {
    name: "env",
    argument: "--env",
    version: "1.4.0",
    description: "Converts `env` and `printenv` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["env", "printenv"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static ENV_PARSER: EnvParser = EnvParser;

inventory::submit! {
    ParserEntry::new(&ENV_PARSER)
}

impl Parser for EnvParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_env(input);
        Ok(ParseOutput::Array(rows))
    }

    /// jc's raw form is the environment as a single object; `_process` turns it
    /// into one `{name, value}` record per variable.
    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut obj = Map::new();
        for row in parse_env(input) {
            let (Some(Value::String(name)), Some(value)) = (row.get("name"), row.get("value"))
            else {
                continue;
            };
            obj.insert(name.clone(), value.clone());
        }
        Ok(ParseOutput::Object(obj))
    }
}

/// Parse env output.
///
/// A new env variable starts when the line matches ^[a-zA-Z_][a-zA-Z0-9_]*=
/// (a valid variable name followed by =). Any other line is a continuation
/// of the previous value.
fn parse_env(input: &str) -> Vec<Map<String, Value>> {
    let re = var_def_re();
    let mut output: Vec<Map<String, Value>> = Vec::new();

    for line in input.lines() {
        if re.is_match(line) {
            // New variable
            let eq_pos = line.find('=').unwrap();
            let name = line[..eq_pos].to_string();
            let value = line[eq_pos + 1..].to_string();
            let mut record = Map::new();
            record.insert("name".to_string(), Value::String(name));
            record.insert("value".to_string(), Value::String(value));
            output.push(record);
        } else {
            // Continuation line: append to last value
            if let Some(last) = output.last_mut()
                && let Some(Value::String(v)) = last.get_mut("value")
            {
                v.push('\n');
                v.push_str(line);
            }
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/env.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/env.json"
        ))
        .unwrap();

        let parser = EnvParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_env_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/env.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/env.json"
        ))
        .unwrap();

        let parser = EnvParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_env_multiline() {
        let input = include_str!("../../../../tests/fixtures/generic/env-multiline.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/env-multiline.json"
        ))
        .unwrap();

        let parser = EnvParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
