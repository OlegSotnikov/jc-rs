//! Parser for `host` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct HostParser;

static INFO: ParserInfo = ParserInfo {
    name: "host",
    argument: "--host",
    version: "1.0.0",
    description: "Converts `host` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["host"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static HOST_PARSER: HostParser = HostParser;

inventory::submit! { ParserEntry::new(&HOST_PARSER) }

impl Parser for HostParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut addresses: Vec<Value> = Vec::new();
        let mut v6addresses: Vec<Value> = Vec::new();
        let mut mail: Vec<Value> = Vec::new();
        let mut text: Vec<Value> = Vec::new();
        let mut rrdata: Map<String, Value> = Map::new();
        let mut soa_parse = false;

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Standard A record
            if line.contains(" has address ") {
                let parts: Vec<&str> = line.splitn(4, ' ').collect();
                let hostname = parts[0].to_string();
                let address = parts.get(3).copied().unwrap_or("").to_string();
                addresses.push(Value::String(address));
                rrdata.insert("hostname".to_string(), Value::String(hostname));
                rrdata.insert("address".to_string(), Value::Array(addresses.clone()));
                continue;
            }

            // AAAA record
            if line.contains(" has IPv6 address ") {
                let parts: Vec<&str> = line.splitn(5, ' ').collect();
                let hostname = parts[0].to_string();
                let v6address = parts.get(4).copied().unwrap_or("").to_string();
                v6addresses.push(Value::String(v6address));
                rrdata.insert("hostname".to_string(), Value::String(hostname));
                rrdata.insert("v6-address".to_string(), Value::Array(v6addresses.clone()));
                continue;
            }

            // MX record
            if line.contains(" mail is handled by ") {
                let parts: Vec<&str> = line.splitn(7, ' ').collect();
                let hostname = parts[0].to_string();
                let mx = parts.get(6).copied().unwrap_or("").to_string();
                mail.push(Value::String(mx));
                rrdata.insert("hostname".to_string(), Value::String(hostname));
                rrdata.insert("mail".to_string(), Value::Array(mail.clone()));
                continue;
            }

            // TXT record
            if line.contains(" descriptive text ") {
                if let Some(text_start) = line.find("descriptive text \"") {
                    let txt = line[text_start + 18..].trim_end_matches('"').to_string();
                    let hostname = line[..line.find(" descriptive").unwrap_or(0)].to_string();
                    text.push(Value::String(txt));
                    rrdata.insert("hostname".to_string(), Value::String(hostname));
                    rrdata.insert("text".to_string(), Value::Array(text.clone()));
                }
                continue;
            }

            // SOA parsing (-C flag)
            if line.starts_with("Nameserver ") {
                soa_parse = true;
                rrdata = Map::new();
                addresses = Vec::new();
                v6addresses = Vec::new();
                mail = Vec::new();
                text = Vec::new();
                let nameserver_ip = line["Nameserver ".len()..]
                    .trim_end_matches(':')
                    .to_string();
                rrdata.insert("nameserver".to_string(), Value::String(nameserver_ip));
                continue;
            }

            if line.contains(" has SOA record ") {
                let parts: Vec<&str> = line.splitn(11, ' ').collect();
                if parts.len() >= 11 {
                    let zone = parts[0].to_string();
                    let mname = parts[4].to_string();
                    let rname = parts[5].to_string();
                    let serial = parts[6].to_string();
                    let refresh = parts[7].to_string();
                    let retry = parts[8].to_string();
                    let expire = parts[9].to_string();
                    let minimum = parts[10].to_string();

                    rrdata.insert("zone".to_string(), Value::String(zone));
                    rrdata.insert("mname".to_string(), Value::String(mname));
                    rrdata.insert("rname".to_string(), Value::String(rname));
                    rrdata.insert("serial".to_string(), parse_int_or_str(&serial));
                    rrdata.insert("refresh".to_string(), parse_int_or_str(&refresh));
                    rrdata.insert("retry".to_string(), parse_int_or_str(&retry));
                    rrdata.insert("expire".to_string(), parse_int_or_str(&expire));
                    rrdata.insert("minimum".to_string(), parse_int_or_str(&minimum));

                    raw_output.push(rrdata.clone());
                    rrdata = Map::new();
                }
                continue;
            }
        }

        if !soa_parse && !rrdata.is_empty() {
            raw_output.push(rrdata);
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

fn parse_int_or_str(s: &str) -> Value {
    s.parse::<i64>()
        .map(|n| Value::Number(n.into()))
        .unwrap_or_else(|_| Value::String(s.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_host_google_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/host-google.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/host-google.json"
        ))
        .unwrap();
        let result = HostParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_host_sunet_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/host-sunet.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/host-sunet.json"
        ))
        .unwrap();
        let result = HostParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_host_empty() {
        let result = HostParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_host_registered() {
        assert!(jc_rs_core::registry::find_parser("host").is_some());
    }
}
