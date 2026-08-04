//! Parser for `ufw status` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct UfwParser;

static INFO: ParserInfo = ParserInfo {
    name: "ufw",
    argument: "--ufw",
    version: "1.2.0",
    description: "Converts `ufw status` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["ufw status"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static UFW_PARSER: UfwParser = UfwParser;

inventory::submit! {
    ParserEntry::new(&UFW_PARSER)
}

/// Parse a "to" or "from" part of a ufw rule line.
/// direction: "to" or "from"
/// Returns a map with appropriate fields.
fn parse_to_from(linedata: &str, direction: &str, rule_obj: &mut Map<String, Value>) {
    let mut linedata = linedata.to_string();

    // Extract rule index for "to" direction: "[ 1] ..."
    if direction == "to" {
        let re_line_num = Regex::new(r"\[[ 0-9]+\]\s").unwrap();
        if let Some(m) = re_line_num.find(&linedata) {
            let num_str = &linedata[m.start()..m.end()];
            let num = num_str
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .to_string();
            rule_obj.insert("index".to_string(), Value::String(num));
            linedata = re_line_num.replace(&linedata, "").to_string();
        } else {
            rule_obj.insert("index".to_string(), Value::Null);
        }
    }

    // Extract comment for "from" direction
    if direction == "from" {
        let re_comment = Regex::new(r"#.+$").unwrap();
        if let Some(m) = re_comment.find(&linedata) {
            let comment = linedata[m.start()..]
                .trim_start_matches('#')
                .trim()
                .to_string();
            rule_obj.insert("comment".to_string(), Value::String(comment));
            linedata = re_comment.replace(&linedata, "").to_string();
        } else {
            rule_obj.insert("comment".to_string(), Value::Null);
        }
    }

    // Pull `(log)`. It can appear on either side of the rule and has to come
    // out of the line before the address is read, or it ends up parsed as the
    // service name.
    let re_log = Regex::new(r"\(log\)").unwrap();
    if re_log.is_match(&linedata) {
        rule_obj.insert("log".to_string(), Value::Bool(true));
        linedata = re_log.replace_all(&linedata, "").to_string();
    } else if !rule_obj.contains_key("log") {
        rule_obj.insert("log".to_string(), Value::Bool(false));
    }

    // Detect IPv6
    let re_v6 = Regex::new(r"\(v6\)").unwrap();
    if re_v6.is_match(&linedata) {
        rule_obj.insert(
            "network_protocol".to_string(),
            Value::String("ipv6".to_string()),
        );
        linedata = re_v6.replace_all(&linedata, "").to_string();
    } else if rule_obj.get("network_protocol").is_none() {
        rule_obj.insert(
            "network_protocol".to_string(),
            Value::String("ipv4".to_string()),
        );
    }

    let is_ipv6 = rule_obj
        .get("network_protocol")
        .and_then(|v| v.as_str())
        .map(|s| s == "ipv6")
        .unwrap_or(false);

    // Handle "Anywhere"
    if linedata.contains("Anywhere") {
        if is_ipv6 {
            rule_obj.insert(format!("{}_ip", direction), Value::String("::".to_string()));
            rule_obj.insert(
                format!("{}_ip_prefix", direction),
                Value::String("0".to_string()),
            );
        } else {
            rule_obj.insert(
                format!("{}_ip", direction),
                Value::String("0.0.0.0".to_string()),
            );
            rule_obj.insert(
                format!("{}_ip_prefix", direction),
                Value::String("0".to_string()),
            );
        }
        linedata = linedata.replace("Anywhere", "");
    }

    // Extract interface: " on ifname"
    let parts: Vec<&str> = linedata.splitn(2, " on ").collect();
    if parts.len() > 1 {
        rule_obj.insert(
            format!("{}_interface", direction),
            Value::String(parts[1].trim().to_string()),
        );
        linedata = parts[0].to_string();
    } else {
        rule_obj.insert(
            format!("{}_interface", direction),
            Value::String("any".to_string()),
        );
    }

    // Extract transport: split on '/' at end
    let transport_re = Regex::new(r"/(tcp|udp|ah|esp|gre|ipv6|igmp)\s*$").unwrap();
    let transport = if let Some(caps) = transport_re.captures(&linedata) {
        let t = caps[1].to_string();
        linedata = transport_re.replace(&linedata, "").to_string();
        Some(t)
    } else {
        None
    };

    // Extract IP addresses from remaining linedata
    let words: Vec<&str> = linedata.split_whitespace().collect();
    let mut remaining_words: Vec<String> = Vec::new();
    let mut found_ip = false;

    for word in &words {
        // Try to parse as IPv4Interface or IPv6Interface
        let (ip, prefix) = if let Some(slash) = word.find('/') {
            let ip_part = &word[..slash];
            let prefix_part = &word[slash + 1..];
            (ip_part, prefix_part)
        } else {
            (*word, "")
        };

        let is_ipv4 = ip.chars().all(|c| c.is_ascii_digit() || c == '.')
            && ip.contains('.')
            && ip.split('.').count() == 4;
        // IPv6 requires >= 2 colons (e.g. ::1, fe80::1, 2001:db8::1)
        // single-colon strings like "8080:8081" are port ranges, not IPv6
        // comma-containing strings like "100:200,300:400" are port range lists, not IPv6
        let is_ipv6 = ip.contains(':') && ip.matches(':').count() >= 2 && !ip.contains(',');

        if is_ipv4 || is_ipv6 {
            let ip_prefix = if prefix.is_empty() {
                if is_ipv4 { "32" } else { "128" }
            } else {
                prefix
            };
            rule_obj.insert(format!("{}_ip", direction), Value::String(ip.to_string()));
            rule_obj.insert(
                format!("{}_ip_prefix", direction),
                Value::String(ip_prefix.to_string()),
            );
            found_ip = true;
        } else {
            remaining_words.push(word.to_string());
        }
    }
    let _ = found_ip;

    linedata = remaining_words.join(" ");

    // Find numeric ports and port ranges
    let items: Vec<&str> = linedata.split(',').collect();
    let mut port_list: Vec<String> = Vec::new();
    let mut port_ranges: Vec<(String, String)> = Vec::new();

    for item in &items {
        let item = item.trim();
        if item.chars().all(|c| c.is_ascii_digit()) && !item.is_empty() {
            port_list.push(item.to_string());
        } else if item.contains(':') {
            let p: Vec<&str> = item.splitn(2, ':').collect();
            if p.len() == 2 {
                port_ranges.push((p[0].trim().to_string(), p[1].trim().to_string()));
            }
        }
    }

    if !port_list.is_empty() || !port_ranges.is_empty() {
        rule_obj.insert(format!("{}_service", direction), Value::Null);
        if !port_list.is_empty() {
            rule_obj.insert(
                format!("{}_ports", direction),
                Value::Array(port_list.iter().map(|s| Value::String(s.clone())).collect()),
            );
        }
        if !port_ranges.is_empty() {
            rule_obj.insert(
                format!("{}_port_ranges", direction),
                Value::Array(
                    port_ranges
                        .iter()
                        .map(|(s, e)| {
                            let mut r = Map::new();
                            r.insert("start".to_string(), Value::String(s.clone()));
                            r.insert("end".to_string(), Value::String(e.clone()));
                            Value::Object(r)
                        })
                        .collect(),
                ),
            );
        }
        linedata = String::new();
    }

    // Set transport
    if let Some(t) = &transport {
        rule_obj.insert(format!("{}_transport", direction), Value::String(t.clone()));
    } else {
        // Check linedata for service name
        let service = linedata.trim();
        if !service.is_empty() {
            rule_obj.insert(
                format!("{}_service", direction),
                Value::String(service.to_string()),
            );
            rule_obj.insert(format!("{}_transport", direction), Value::Null);
        } else {
            rule_obj.insert(
                format!("{}_transport", direction),
                Value::String("any".to_string()),
            );
        }
    }

    // Set default IPs if not found
    if !rule_obj.contains_key(&format!("{}_ip", direction)) {
        if is_ipv6 {
            rule_obj.insert(format!("{}_ip", direction), Value::String("::".to_string()));
            rule_obj.insert(
                format!("{}_ip_prefix", direction),
                Value::String("0".to_string()),
            );
        } else {
            rule_obj.insert(
                format!("{}_ip", direction),
                Value::String("0.0.0.0".to_string()),
            );
            rule_obj.insert(
                format!("{}_ip_prefix", direction),
                Value::String("0".to_string()),
            );
        }
    }

    // Set default port ranges if transport is tcp/udp/any but no ports specified
    let has_ports = rule_obj.contains_key(&format!("{}_ports", direction));
    let has_ranges = rule_obj.contains_key(&format!("{}_port_ranges", direction));

    if !has_ports
        && !has_ranges
        && let Some(Value::String(t)) = rule_obj.get(&format!("{}_transport", direction))
        && (t == "tcp" || t == "udp" || t == "any")
    {
        rule_obj.insert(
            format!("{}_port_ranges", direction),
            Value::Array(vec![{
                let mut r = Map::new();
                r.insert("start".to_string(), Value::String("0".to_string()));
                r.insert("end".to_string(), Value::String("65535".to_string()));
                Value::Object(r)
            }]),
        );
        rule_obj.insert(format!("{}_service", direction), Value::Null);
    }
}

fn process_rule(rule: &mut Map<String, Value>) {
    let int_fields = ["index", "to_ip_prefix", "from_ip_prefix"];
    for field in &int_fields {
        if let Some(Value::String(s)) = rule.get(*field).cloned()
            && let Ok(n) = s.trim().parse::<i64>()
        {
            rule.insert(field.to_string(), Value::Number(n.into()));
        }
    }

    for direction in &["to", "from"] {
        let ports_key = format!("{}_ports", direction);
        if let Some(Value::Array(ports)) = rule.get(&ports_key).cloned() {
            let converted: Vec<Value> = ports
                .into_iter()
                .map(|v| {
                    if let Value::String(s) = &v
                        && let Ok(n) = s.parse::<i64>()
                    {
                        return Value::Number(n.into());
                    }
                    v
                })
                .collect();
            rule.insert(ports_key, Value::Array(converted));
        }

        let ranges_key = format!("{}_port_ranges", direction);
        if let Some(Value::Array(ranges)) = rule.get(&ranges_key).cloned() {
            let converted: Vec<Value> = ranges
                .into_iter()
                .map(|r| {
                    if let Value::Object(mut range_obj) = r {
                        if let Some(Value::String(s)) = range_obj.get("start").cloned()
                            && let Ok(n) = s.parse::<i64>()
                        {
                            range_obj.insert("start".to_string(), Value::Number(n.into()));
                        }
                        if let Some(Value::String(s)) = range_obj.get("end").cloned()
                            && let Ok(n) = s.parse::<i64>()
                        {
                            range_obj.insert("end".to_string(), Value::Number(n.into()));
                        }
                        Value::Object(range_obj)
                    } else {
                        r
                    }
                })
                .collect();
            rule.insert(ranges_key, Value::Array(converted));
        }
    }
}

impl Parser for UfwParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut raw_output: Map<String, Value> = Map::new();
        let mut rules_list: Vec<Value> = Vec::new();
        let mut rule_lines = false;

        // Regex for splitting on action keywords
        let action_re = Regex::new(
            r"(ALLOW IN|ALLOW OUT|ALLOW FWD|DENY IN|DENY OUT|DENY FWD|LIMIT IN|LIMIT OUT|LIMIT FWD|REJECT IN|REJECT OUT|REJECT FWD|ALLOW|DENY|LIMIT|REJECT)",
        ).unwrap();

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if line.starts_with("Status: ") {
                raw_output.insert(
                    "status".to_string(),
                    Value::String(line[8..].trim().to_string()),
                );
                continue;
            }

            if line.starts_with("Logging: ") {
                let log_line = line[9..].trim();
                let parts: Vec<&str> = log_line.split_whitespace().collect();
                if !parts.is_empty() {
                    raw_output.insert("logging".to_string(), Value::String(parts[0].to_string()));
                    if parts.len() >= 2 {
                        let level = parts[1]
                            .trim_start_matches('(')
                            .trim_end_matches(')')
                            .to_string();
                        raw_output.insert("logging_level".to_string(), Value::String(level));
                    }
                }
                continue;
            }

            if line.starts_with("Default: ") {
                raw_output.insert(
                    "default".to_string(),
                    Value::String(line[9..].trim().to_string()),
                );
                continue;
            }

            if line.starts_with("New profiles: ") {
                raw_output.insert(
                    "new_profiles".to_string(),
                    Value::String(line[14..].trim().to_string()),
                );
                continue;
            }

            if line.contains("To") && line.contains("Action") && line.contains("From") {
                rule_lines = true;
                continue;
            }

            if rule_lines {
                if line.contains("------") {
                    continue;
                }

                // Split on action keyword
                let action_match: Vec<regex::Match> = action_re.find_iter(line).collect();

                if action_match.is_empty() {
                    continue;
                }

                let to_line = &line[..action_match[0].start()];
                let action_str = action_match[0].as_str();
                let from_line = &line[action_match[0].end()..];

                let action_parts: Vec<&str> = action_str.split_whitespace().collect();
                let action = action_parts[0].to_string();
                let action_direction = if action_parts.len() > 1 {
                    Value::String(action_parts[1].to_string())
                } else {
                    Value::Null
                };

                let mut rule: Map<String, Value> = Map::new();
                rule.insert("action".to_string(), Value::String(action));
                rule.insert("action_direction".to_string(), action_direction);

                parse_to_from(to_line, "to", &mut rule);
                parse_to_from(from_line, "from", &mut rule);

                process_rule(&mut rule);
                rules_list.push(Value::Object(rule));
            }
        }

        raw_output.insert("rules".to_string(), Value::Array(rules_list));

        Ok(ParseOutput::Object(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ufw_inactive() {
        let input = "Status: inactive\n";
        let parser = UfwParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("status"),
                Some(&Value::String("inactive".to_string()))
            );
            if let Some(Value::Array(rules)) = obj.get("rules") {
                assert_eq!(rules.len(), 0);
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_ufw_basic() {
        let input = "Status: active\nLogging: on (low)\nDefault: deny (incoming), allow (outgoing)\nNew profiles: skip\n \nTo                         Action      From\n--                         ------      ----\n22/tcp                     ALLOW IN    Anywhere\n";
        let parser = UfwParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("status"),
                Some(&Value::String("active".to_string()))
            );
            assert_eq!(obj.get("logging"), Some(&Value::String("on".to_string())));
            assert_eq!(
                obj.get("logging_level"),
                Some(&Value::String("low".to_string()))
            );
            if let Some(Value::Array(rules)) = obj.get("rules") {
                assert_eq!(rules.len(), 1);
                if let Value::Object(rule) = &rules[0] {
                    assert_eq!(
                        rule.get("action"),
                        Some(&Value::String("ALLOW".to_string()))
                    );
                    assert_eq!(
                        rule.get("action_direction"),
                        Some(&Value::String("IN".to_string()))
                    );
                }
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_ufw_empty() {
        let parser = UfwParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
