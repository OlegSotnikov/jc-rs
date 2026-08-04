//! Parser for `sysctl` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SysctlParser;

static INFO: ParserInfo = ParserInfo {
    name: "sysctl",
    argument: "--sysctl",
    version: "1.2.0",
    description: "Converts `sysctl` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["sysctl"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSCTL_PARSER: SysctlParser = SysctlParser;

inventory::submit! {
    ParserEntry::new(&SYSCTL_PARSER)
}

impl Parser for SysctlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let record = parse_sysctl(input);
        Ok(ParseOutput::Object(record))
    }
}

pub fn parse_sysctl(input: &str) -> Map<String, Value> {
    let mut raw: Map<String, Value> = Map::new();

    if input.trim().is_empty() {
        return raw;
    }

    let lines: Vec<&str> = input.lines().collect();

    // Detect delimiter: Linux uses ' = ', BSD uses ': '
    // Check if any line has a key-like prefix (no spaces, has a dot) followed by ' = '
    let delim = if lines.iter().any(|l| {
        if let Some(idx) = l.find(" = ") {
            let key = &l[..idx];
            !key.contains(' ') && key.contains('.')
        } else {
            false
        }
    }) {
        " = "
    } else {
        ": "
    };

    let mut last_key: Option<String> = None;

    for line in &lines {
        match line.split_once(delim) {
            Some((key, value)) => {
                let key = key.to_string();
                let value = value.to_string();

                // Check for key sanity: no spaces and has at least one dot (or is known key format)
                if key.contains(' ') || !key.contains('.') {
                    // This is a continuation value on a new line
                    if let Some(ref lk) = last_key
                        && let Some(Value::String(existing)) = raw.get_mut(lk)
                    {
                        existing.push('\n');
                        existing.push_str(line);
                    }
                    continue;
                }

                // Duplicate key: append
                if raw.contains_key(&key)
                    && let Some(Value::String(existing)) = raw.get_mut(&key)
                {
                    existing.push('\n');
                    existing.push_str(&value);
                    last_key = Some(key);
                    continue;
                }

                last_key = Some(key.clone());
                raw.insert(key, Value::String(value));
            }
            None => {
                // No delimiter: continuation of previous value
                if let Some(ref lk) = last_key
                    && let Some(Value::String(existing)) = raw.get_mut(lk)
                {
                    existing.push('\n');
                    existing.push_str(line);
                }
            }
        }
    }

    // Convert int/float values where possible
    let mut processed: Map<String, Value> = Map::new();
    for (key, val) in raw {
        let new_val = match &val {
            Value::String(s) => {
                if let Ok(i) = s.parse::<i64>() {
                    Value::Number(i.into())
                } else if let Ok(u) = s.parse::<u64>() {
                    Value::Number(u.into())
                } else if let Ok(f) = s.parse::<f64>() {
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(val)
                } else {
                    val
                }
            }
            _ => val,
        };
        processed.insert(key, new_val);
    }

    processed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sysctl_centos() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/sysctl-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/sysctl-a.json"
        ))
        .unwrap();
        let parser = SysctlParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_sysctl_ubuntu18() {
        let input = include_str!("../../../../tests/fixtures/ubuntu-18.04/sysctl-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/ubuntu-18.04/sysctl-a.json"
        ))
        .unwrap();
        let parser = SysctlParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_sysctl_osx() {
        let input = include_str!("../../../../tests/fixtures/osx-10.14.6/sysctl-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/osx-10.14.6/sysctl-a.json"
        ))
        .unwrap();
        let parser = SysctlParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }

    #[test]
    fn test_sysctl_freebsd() {
        let input = include_str!("../../../../tests/fixtures/freebsd12/sysctl-a.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/freebsd12/sysctl-a.json"
        ))
        .unwrap();
        let parser = SysctlParser;
        let result = parser.parse(input, false).unwrap();
        let result_value: serde_json::Value = serde_json::to_value(result).unwrap();
        assert_eq!(result_value, expected);
    }
}
