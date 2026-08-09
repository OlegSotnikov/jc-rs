//! Parser for `nsd-control` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct NsdControlParser;

static INFO: ParserInfo = ParserInfo {
    name: "nsd_control",
    argument: "--nsd-control",
    version: "1.2.0",
    description: "Converts `nsd-control` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::Command],
    magic_commands: &["nsd-control"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static NSD_CONTROL_PARSER: NsdControlParser = NsdControlParser;

inventory::submit! {
    ParserEntry::new(&NSD_CONTROL_PARSER)
}

fn parse_int(s: &str) -> Value {
    if let Ok(n) = s.trim().parse::<i64>() {
        Value::Number(n.into())
    } else {
        Value::String(s.trim().to_string())
    }
}

fn parse_float(s: &str) -> Value {
    if let Ok(f) = s.trim().parse::<f64>()
        && let Some(n) = serde_json::Number::from_f64(f)
    {
        return Value::Number(n);
    }
    Value::String(s.trim().to_string())
}

impl Parser for NsdControlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut result: Vec<Map<String, Value>> = Vec::new();
        let mut itr: Map<String, Value> = Map::new();
        let mut itr_parse = false;

        // Zone state tracking
        let mut zone_name: Option<String> = None;
        let mut zone_status: Map<String, Value> = Map::new();

        // Status block tracking
        let mut status_obj: Option<Map<String, Value>> = None;

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line == "ok" {
                let mut obj = Map::new();
                obj.insert("command".to_string(), Value::String("ok".to_string()));
                result.push(obj);
                continue;
            }

            if line.starts_with("version:") {
                let val = line["version:".len()..].trim().to_string();
                let mut obj = Map::new();
                obj.insert("version".to_string(), Value::String(val));
                status_obj = Some(obj);
                continue;
            }

            if line.starts_with("verbosity:") {
                let val = line["verbosity:".len()..].trim().to_string();
                if let Some(ref mut obj) = status_obj {
                    obj.insert("verbosity".to_string(), parse_int(&val));
                }
                continue;
            }

            if line.starts_with("ratelimit:") {
                let val = line["ratelimit:".len()..].trim().to_string();
                if let Some(mut obj) = status_obj.take() {
                    obj.insert("ratelimit".to_string(), parse_int(&val));
                    result.push(obj);
                }
                continue;
            }

            if line.starts_with("active") {
                itr_parse = true;
                itr = Map::new();
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    itr.insert(
                        "active".to_string(),
                        Value::String(parts[1].trim().to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("staging") {
                let parts: Vec<&str> = line.splitn(2, ':').collect();
                if parts.len() == 2 {
                    itr.insert(
                        "staging".to_string(),
                        Value::String(parts[1].trim().to_string()),
                    );
                }
                continue;
            }

            if line.starts_with("key:") {
                // Format: key: name: "X" secret: "Y" algorithm: Z
                let tokens: Vec<&str> = line.splitn(7, ' ').collect();
                // tokens: ["key:", "name:", "\"X\"", "secret:", "\"Y\"", "algorithm:", "Z"]
                if tokens.len() >= 7 {
                    let name = tokens[2].trim_matches('"').to_string();
                    let secret = tokens[4].trim_matches('"').to_string();
                    let algorithm = tokens[6].trim_matches('"').to_string();
                    let mut tsig_data = Map::new();
                    tsig_data.insert("name".to_string(), Value::String(name));
                    tsig_data.insert("secret".to_string(), Value::String(secret));
                    tsig_data.insert("algorithm".to_string(), Value::String(algorithm));
                    let mut tsig = Map::new();
                    tsig.insert("key".to_string(), Value::Object(tsig_data));
                    result.push(tsig);
                }
                continue;
            }

            if line.starts_with("zone:") {
                // Flush previous zone if any
                if let Some(zn) = zone_name.take() {
                    // Zone not yet appended (shouldn't happen if wait/transfer came)
                    // but just in case, append with what we have
                    let mut zone_obj = Map::new();
                    zone_obj.insert("zone".to_string(), Value::String(zn));
                    if !zone_status.is_empty() {
                        zone_obj.insert("status".to_string(), Value::Object(zone_status.clone()));
                        zone_status.clear();
                    }
                    result.push(zone_obj);
                }
                // Parse: zone:\tname
                let parts: Vec<&str> = line.splitn(2, ":\t").collect();
                if parts.len() == 2 {
                    zone_name = Some(parts[1].trim().to_string());
                } else {
                    // fallback: split on ': '
                    let parts2: Vec<&str> = line.splitn(2, ": ").collect();
                    if parts2.len() == 2 {
                        zone_name = Some(parts2[1].trim().to_string());
                    }
                }
                zone_status = Map::new();
                continue;
            }

            if line.starts_with("pattern:") {
                let val = line["pattern:".len()..].trim_start_matches(' ').to_string();
                zone_status.insert("pattern".to_string(), Value::String(val.trim().to_string()));
                continue;
            }

            if line.starts_with("catalog-member-id:") {
                let val = line["catalog-member-id:".len()..].trim().to_string();
                zone_status.insert("catalog-member-id".to_string(), Value::String(val));
                continue;
            }

            if line.starts_with("state:") {
                let val = line["state:".len()..].trim().to_string();
                zone_status.insert("state".to_string(), Value::String(val));
                continue;
            }

            if line.starts_with("served-serial:") {
                let val = line["served-serial:".len()..]
                    .trim()
                    .trim_matches('"')
                    .to_string();
                zone_status.insert("served-serial".to_string(), Value::String(val));
                continue;
            }

            if line.starts_with("commit-serial:") {
                let val = line["commit-serial:".len()..]
                    .trim()
                    .trim_matches('"')
                    .to_string();
                zone_status.insert("commit-serial".to_string(), Value::String(val));
                continue;
            }

            if line.starts_with("notified-serial:") {
                let val = line["notified-serial:".len()..]
                    .trim()
                    .trim_matches('"')
                    .to_string();
                zone_status.insert("notified-serial".to_string(), Value::String(val));
                continue;
            }

            if line.starts_with("wait:") {
                let val = line["wait:".len()..].trim().trim_matches('"').to_string();
                zone_status.insert("wait".to_string(), Value::String(val));
                // Append zone
                if let Some(zn) = zone_name.take() {
                    let mut zone_obj = Map::new();
                    zone_obj.insert("zone".to_string(), Value::String(zn));
                    zone_obj.insert("status".to_string(), Value::Object(zone_status.clone()));
                    zone_status.clear();
                    result.push(zone_obj);
                }
                continue;
            }

            if line.starts_with("transfer:") {
                let val = line["transfer:".len()..]
                    .trim()
                    .trim_matches('"')
                    .to_string();
                zone_status.insert("transfer".to_string(), Value::String(val));
                // Append zone
                if let Some(zn) = zone_name.take() {
                    let mut zone_obj = Map::new();
                    zone_obj.insert("zone".to_string(), Value::String(zn));
                    zone_obj.insert("status".to_string(), Value::Object(zone_status.clone()));
                    zone_status.clear();
                    result.push(zone_obj);
                }
                continue;
            }

            // Stats: key=value pairs (server*, num.*, size.*, time.*, zone.*)
            if line.starts_with("server")
                || line.starts_with("num.")
                || line.starts_with("size.")
                || line.starts_with("time.")
                || line.starts_with("zone.")
            {
                itr_parse = true;
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].to_string();
                    let val_str = parts[1];
                    let val = if key.starts_with("time.") {
                        parse_float(val_str)
                    } else {
                        parse_int(val_str)
                    };
                    itr.insert(key, val);
                }
                continue;
            }
        }

        // Flush accumulated stats/cookie
        if itr_parse && !itr.is_empty() {
            result.push(itr);
        }

        // Flush any pending zone without wait/transfer
        if let Some(zn) = zone_name {
            let mut zone_obj = Map::new();
            zone_obj.insert("zone".to_string(), Value::String(zn));
            if !zone_status.is_empty() {
                zone_obj.insert("status".to_string(), Value::Object(zone_status));
            }
            result.push(zone_obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_json_array(input: &str, expected: &str) {
        let parser = NsdControlParser;
        let result = parser.parse(input, false).unwrap();
        let expected_val: Vec<serde_json::Value> = serde_json::from_str(expected).unwrap();
        match result {
            ParseOutput::Array(arr) => {
                assert_eq!(
                    serde_json::Value::Array(arr.into_iter().map(Value::Object).collect()),
                    serde_json::Value::Array(expected_val)
                );
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn test_nsd_control_ok() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control.json"),
        );
    }

    #[test]
    fn test_nsd_control_status() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control-status.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control-status.json"),
        );
    }

    #[test]
    fn test_nsd_control_zonestatus() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control-zonestatus.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control-zonestatus.json"),
        );
    }

    #[test]
    fn test_nsd_control_tsig() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control-tsig.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control-tsig.json"),
        );
    }

    #[test]
    fn test_nsd_control_cookie() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control-cookie_secrets.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control-cookie_secrets.json"),
        );
    }

    #[test]
    fn test_nsd_control_stats() {
        parse_json_array(
            include_str!("../../../../tests/fixtures/generic/nsd_control-stats.out"),
            include_str!("../../../../tests/fixtures/generic/nsd_control-stats.json"),
        );
    }

    #[test]
    fn test_nsd_control_empty() {
        let parser = NsdControlParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("expected Array");
        }
    }
}
