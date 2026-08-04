//! Parser for `systemctl list-sockets` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SystemctlLsParser;

static INFO: ParserInfo = ParserInfo {
    name: "systemctl_ls",
    argument: "--systemctl-ls",
    version: "1.5.0",
    description: "Converts `systemctl list-sockets` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["systemctl list-sockets"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSTEMCTL_LS_PARSER: SystemctlLsParser = SystemctlLsParser;

inventory::submit! {
    ParserEntry::new(&SYSTEMCTL_LS_PARSER)
}

impl Parser for SystemctlLsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let rows = parse_systemctl_ls(input);
        Ok(ParseOutput::Array(rows))
    }
}

fn parse_systemctl_ls(input: &str) -> Vec<Map<String, Value>> {
    let all_lines: Vec<&str> = input.lines().collect();
    let mut iter = all_lines.iter();

    // Find header line (first non-empty)
    let header_line = loop {
        match iter.next() {
            None => return Vec::new(),
            Some(l) if !l.trim().is_empty() => break l,
            _ => continue,
        }
    };

    // Compute column byte positions from header (header is ASCII)
    let header_lower = header_line.to_lowercase();
    let headers: Vec<&str> = header_lower.split_whitespace().collect();
    let mut col_starts: Vec<usize> = Vec::new();
    let mut search_from = 0usize;
    for h in &headers {
        if let Some(pos) = header_lower[search_from..].find(h.as_ref() as &str) {
            col_starts.push(search_from + pos);
            search_from = search_from + pos + h.len();
        }
    }

    let n = col_starts.len();
    let mut output = Vec::new();

    for line in iter {
        if line.contains("sockets listed.") {
            break;
        }
        if line.trim().is_empty() {
            continue;
        }

        let mut record = Map::new();
        for i in 0..n {
            let start = col_starts[i];
            let end = if i + 1 < n {
                col_starts[i + 1]
            } else {
                line.len()
            };
            let val = if start < line.len() {
                let actual_end = end.min(line.len());
                line[start..actual_end].trim()
            } else {
                ""
            };
            if !val.is_empty() {
                record.insert(headers[i].to_string(), Value::String(val.to_string()));
            }
        }

        if !record.is_empty() {
            output.push(record);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemctl_ls_basic() {
        let input = "LISTEN                          UNIT                            ACTIVATES\n\
                     /dev/log                        systemd-journald.socket         systemd-journald.service\n\
                     /run/dbus/system_bus_socket     dbus.socket                     dbus.service\n\
                     \n\
                     2 sockets listed.";

        let parser = SystemctlLsParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("listen"),
                Some(&Value::String("/dev/log".to_string()))
            );
            assert_eq!(
                arr[0].get("unit"),
                Some(&Value::String("systemd-journald.socket".to_string()))
            );
            assert_eq!(
                arr[0].get("activates"),
                Some(&Value::String("systemd-journald.service".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_systemctl_ls_empty() {
        let parser = SystemctlLsParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
