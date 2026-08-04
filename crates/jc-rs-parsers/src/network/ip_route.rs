//! Parser for `ip route` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct IpRouteParser;

static INFO: ParserInfo = ParserInfo {
    name: "ip_route",
    argument: "--ip-route",
    version: "1.0.0",
    description: "Converts `ip route` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &["ip route"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IP_ROUTE_PARSER: IpRouteParser = IpRouteParser;

inventory::submit! { ParserEntry::new(&IP_ROUTE_PARSER) }

impl Parser for IpRouteParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut items = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let mut structure: Map<String, Value> = Map::new();
            let temp: Vec<&str> = line.split_whitespace().collect();
            let mut i = 0;

            while i < temp.len() {
                match temp[i] {
                    "via" => {
                        if i + 1 < temp.len() {
                            structure
                                .insert("via".to_string(), Value::String(temp[i + 1].to_string()));
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "dev" => {
                        if i + 1 < temp.len() {
                            structure
                                .insert("dev".to_string(), Value::String(temp[i + 1].to_string()));
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "metric" => {
                        if i + 1 < temp.len() {
                            let metric = temp[i + 1];
                            if let Ok(n) = metric.parse::<i64>() {
                                structure.insert("metric".to_string(), Value::Number(n.into()));
                            } else {
                                structure.insert(
                                    "metric".to_string(),
                                    Value::String(metric.to_string()),
                                );
                            }
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "proto" => {
                        if i + 1 < temp.len() {
                            structure.insert(
                                "proto".to_string(),
                                Value::String(temp[i + 1].to_string()),
                            );
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "scope" => {
                        if i + 1 < temp.len() {
                            structure.insert(
                                "scope".to_string(),
                                Value::String(temp[i + 1].to_string()),
                            );
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "src" => {
                        if i + 1 < temp.len() {
                            structure
                                .insert("src".to_string(), Value::String(temp[i + 1].to_string()));
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "status" => {
                        if i + 1 < temp.len() {
                            structure.insert(
                                "status".to_string(),
                                Value::String(temp[i + 1].to_string()),
                            );
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "default" => {
                        structure.insert("ip".to_string(), Value::String("default".to_string()));
                        i += 1;
                    }
                    "linkdown" => {
                        structure
                            .insert("status".to_string(), Value::String("linkdown".to_string()));
                        i += 1;
                    }
                    word => {
                        // First token that doesn't match a keyword is the IP/route
                        if !structure.contains_key("ip") {
                            structure.insert("ip".to_string(), Value::String(word.to_string()));
                        }
                        i += 1;
                    }
                }
            }

            if structure.contains_key("ip") || !structure.is_empty() {
                items.push(structure);
            }
        }

        Ok(ParseOutput::Array(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_ip_route_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/ip_route.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/ip_route.json"
        ))
        .unwrap();
        let result = IpRouteParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ip_route_empty() {
        let result = IpRouteParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_ip_route_registered() {
        assert!(jc_rs_core::registry::find_parser("ip_route").is_some());
    }
}
