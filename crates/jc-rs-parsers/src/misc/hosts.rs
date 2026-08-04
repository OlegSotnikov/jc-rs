use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct HostsParser;

static INFO: ParserInfo = ParserInfo {
    name: "hosts",
    argument: "--hosts",
    version: "1.4.0",
    description: "Converts `/etc/hosts` file content to JSON",
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

static HOSTS_PARSER: HostsParser = HostsParser;

inventory::submit! {
    ParserEntry::new(&HOSTS_PARSER)
}

impl Parser for HostsParser {
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

            let parts: Vec<&str> = line.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                continue;
            }

            let ip = parts[0].trim();
            let rest = parts[1].trim();

            // Split hostnames and remove inline comments
            let mut hosts_list: Vec<&str> = rest.split_whitespace().collect();
            if let Some(comment_pos) = hosts_list.iter().position(|h| h.contains('#')) {
                hosts_list.truncate(comment_pos);
            }

            if hosts_list.is_empty() {
                continue;
            }

            let mut obj = Map::new();
            obj.insert("ip".to_string(), Value::String(ip.to_string()));
            obj.insert(
                "hostname".to_string(),
                Value::Array(
                    hosts_list
                        .iter()
                        .map(|h| Value::String(h.to_string()))
                        .collect(),
                ),
            );
            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hosts_basic() {
        let input = "127.0.0.1 localhost\n::1 ip6-localhost ip6-loopback\n";
        let parser = HostsParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert_eq!(arr[0]["ip"], Value::String("127.0.0.1".to_string()));
                let hostnames = arr[0]["hostname"].as_array().unwrap();
                assert_eq!(hostnames[0], Value::String("localhost".to_string()));
            }
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_hosts_skip_comments() {
        let input = "# comment\n127.0.0.1 localhost # inline comment\n";
        let parser = HostsParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(arr.len(), 1);
            }
            _ => panic!("expected array"),
        }
    }
}
