//! Parser for `systemctl list-jobs` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SystemctlLjParser;

static INFO: ParserInfo = ParserInfo {
    name: "systemctl_lj",
    argument: "--systemctl-lj",
    version: "1.7.0",
    description: "Converts `systemctl list-jobs` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["systemctl list-jobs"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSTEMCTL_LJ_PARSER: SystemctlLjParser = SystemctlLjParser;

inventory::submit! {
    ParserEntry::new(&SYSTEMCTL_LJ_PARSER)
}

impl Parser for SystemctlLjParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_systemctl_lj(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_systemctl_lj(input: &str) -> Vec<Map<String, Value>> {
    let mut lines = input.lines().filter(|l| !l.trim().is_empty());

    let header_line = match lines.next() {
        Some(l) => l,
        None => return Vec::new(),
    };

    let header_list: Vec<String> = header_line
        .to_lowercase()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    let mut output = Vec::new();

    for line in lines {
        if line.contains("No jobs running.") || line.contains("jobs listed.") {
            break;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        let mut record = Map::new();

        for (i, header) in header_list.iter().enumerate() {
            if i < header_list.len() - 1 {
                if let Some(&val) = tokens.get(i) {
                    // job field is integer
                    if header == "job" {
                        if let Ok(n) = val.parse::<i64>() {
                            record.insert(header.clone(), Value::Number(n.into()));
                        } else {
                            record.insert(header.clone(), Value::String(val.to_string()));
                        }
                    } else {
                        record.insert(header.clone(), Value::String(val.to_string()));
                    }
                }
            } else {
                if tokens.len() > i {
                    let val = tokens[i..].join(" ");
                    record.insert(header.clone(), Value::String(val));
                }
            }
        }

        output.push(record);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemctl_lj_basic() {
        let input = "JOB UNIT                                TYPE  STATE\n\
                     3543 nginxAfterGlusterfs.service         start waiting\n\
                     3545 glusterReadyForLocalhostMount.service start running\n\
                     \n\
                     2 jobs listed.";

        let parser = SystemctlLjParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].get("job"), Some(&Value::Number(3543.into())));
            assert_eq!(
                arr[0].get("unit"),
                Some(&Value::String("nginxAfterGlusterfs.service".to_string()))
            );
            assert_eq!(
                arr[0].get("type"),
                Some(&Value::String("start".to_string()))
            );
            assert_eq!(
                arr[0].get("state"),
                Some(&Value::String("waiting".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_systemctl_lj_empty() {
        let parser = SystemctlLjParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
