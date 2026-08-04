use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub struct SshdConfParser;

static INFO: ParserInfo = ParserInfo {
    name: "sshd_conf",
    argument: "--sshd-conf",
    version: "1.1.0",
    description: "`sshd` config file and `sshd -T` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::File],
    magic_commands: &["sshd -T"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SSHD_CONF_PARSER: SshdConfParser = SshdConfParser;

inventory::submit! {
    ParserEntry::new(&SSHD_CONF_PARSER)
}

// Fields that can appear multiple times
fn multi_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &["acceptenv", "hostkey", "include", "listenaddress", "port"] {
        s.insert(*f);
    }
    s
}

// Fields split on whitespace in _process
fn split_space_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "authorizedkeysfile",
        "include",
        "ipqos",
        "permitlisten",
        "permitopen",
    ] {
        s.insert(*f);
    }
    s
}

// Fields split on comma in _process
fn split_comma_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "casignaturealgorithms",
        "ciphers",
        "gssapikexalgorithms",
        "hostbasedacceptedalgorithms",
        "hostbasedacceptedkeytypes",
        "hostkeyalgorithms",
        "kexalgorithms",
        "macs",
        "pubkeyacceptedalgorithms",
        "pubkeyacceptedkeytypes",
    ] {
        s.insert(*f);
    }
    s
}

// Fields converted to int in _process
fn int_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "clientalivecountmax",
        "clientaliveinterval",
        "logingracetime",
        "maxauthtries",
        "maxsessions",
        "maxstartups",
        "maxstartups_rate",
        "maxstartups_full",
        "rekeylimit",
        "rekeylimit_time",
        "x11displayoffset",
        "x11maxdisplays",
    ] {
        s.insert(*f);
    }
    s
}

// Fields with modifier prefixes (+, -, ^)
fn modified_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "casignaturealgorithms",
        "ciphers",
        "hostbasedacceptedalgorithms",
        "kexalgorithms",
        "macs",
        "pubkeyacceptedalgorithms",
    ] {
        s.insert(*f);
    }
    s
}

fn process_output(raw: Map<String, Value>) -> Map<String, Value> {
    let split_space = split_space_fields();
    let split_comma = split_comma_fields();
    let int_flds = int_fields();
    let mut result = Map::new();

    for (key, val) in &raw {
        // acceptenv: flatten list of strings split on whitespace
        if key == "acceptenv"
            && let Value::Array(items) = val
        {
            let mut flat: Vec<Value> = Vec::new();
            for item in items {
                if let Value::String(s) = item {
                    for part in s.split_whitespace() {
                        flat.push(Value::String(part.to_string()));
                    }
                }
            }
            result.insert(key.clone(), Value::Array(flat));
            continue;
        }

        // include: flatten list
        if key == "include"
            && let Value::Array(items) = val
        {
            let mut flat: Vec<Value> = Vec::new();
            for item in items {
                if let Value::String(s) = item {
                    for part in s.split_whitespace() {
                        flat.push(Value::String(part.to_string()));
                    }
                }
            }
            result.insert(key.clone(), Value::Array(flat));
            continue;
        }

        // port: convert list of strings to list of ints
        if key == "port"
            && let Value::Array(items) = val
        {
            let port_list: Vec<Value> = items
                .iter()
                .filter_map(|v| {
                    if let Value::String(s) = v {
                        s.parse::<i64>().ok().map(|n| Value::Number(n.into()))
                    } else {
                        None
                    }
                })
                .collect();
            result.insert(key.clone(), Value::Array(port_list));
            continue;
        }

        // Other array fields (hostkey, listenaddress) stay as-is
        if let Value::Array(_) = val {
            result.insert(key.clone(), val.clone());
            continue;
        }

        if let Value::String(s) = val {
            // maxstartups: split on ':'
            if key == "maxstartups" {
                let parts: Vec<&str> = s.splitn(3, ':').collect();
                let base = parts[0];
                result.insert(
                    key.clone(),
                    match base.parse::<i64>() {
                        Ok(n) => Value::Number(n.into()),
                        Err(_) => Value::String(base.to_string()),
                    },
                );
                if parts.len() > 1 {
                    let rate = parts[1];
                    result.insert(
                        "maxstartups_rate".to_string(),
                        match rate.parse::<i64>() {
                            Ok(n) => Value::Number(n.into()),
                            Err(_) => Value::String(rate.to_string()),
                        },
                    );
                }
                if parts.len() > 2 {
                    let full = parts[2];
                    result.insert(
                        "maxstartups_full".to_string(),
                        match full.parse::<i64>() {
                            Ok(n) => Value::Number(n.into()),
                            Err(_) => Value::String(full.to_string()),
                        },
                    );
                }
                continue;
            }

            // rekeylimit: split on space, then convert to int
            if key == "rekeylimit" {
                let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
                let rekey_val = parts[0];
                result.insert(
                    key.clone(),
                    match rekey_val.parse::<i64>() {
                        Ok(n) => Value::Number(n.into()),
                        Err(_) => Value::String(rekey_val.to_string()),
                    },
                );
                if parts.len() > 1 {
                    let time_val = parts[1].trim();
                    result.insert(
                        "rekeylimit_time".to_string(),
                        match time_val.parse::<i64>() {
                            Ok(n) => Value::Number(n.into()),
                            Err(_) => Value::String(time_val.to_string()),
                        },
                    );
                }
                continue;
            }

            // subsystem: split on space
            if key == "subsystem" {
                let parts: Vec<&str> = s.splitn(2, char::is_whitespace).collect();
                result.insert(key.clone(), Value::String(parts[0].to_string()));
                if parts.len() > 1 {
                    result.insert(
                        "subsystem_command".to_string(),
                        Value::String(parts[1].trim().to_string()),
                    );
                }
                continue;
            }

            if split_space.contains(key.as_str()) {
                let parts: Vec<Value> = s
                    .split_whitespace()
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                result.insert(key.clone(), Value::Array(parts));
                continue;
            }

            if split_comma.contains(key.as_str()) {
                let parts: Vec<Value> =
                    s.split(',').map(|p| Value::String(p.to_string())).collect();
                result.insert(key.clone(), Value::Array(parts));
                continue;
            }

            if int_flds.contains(key.as_str()) {
                match convert_to_int(s) {
                    Some(n) => result.insert(key.clone(), Value::Number(n.into())),
                    None => result.insert(key.clone(), Value::Null),
                };
                continue;
            }

            result.insert(key.clone(), Value::String(s.clone()));
        } else {
            result.insert(key.clone(), val.clone());
        }
    }

    result
}

impl Parser for SshdConfParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let multi = multi_fields();
        let modified = modified_fields();
        let modifiers: HashSet<char> = ['+', '-', '^'].iter().cloned().collect();

        let mut raw_output: Map<String, Value> = Map::new();
        let mut match_block_found = false;

        for line in input.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line_trimmed.starts_with('#') {
                continue;
            }

            if line_trimmed.starts_with("Match all") {
                match_block_found = false;
                continue;
            }
            if line_trimmed.starts_with("Match") {
                match_block_found = true;
                continue;
            }
            if match_block_found {
                continue;
            }

            let parts: Vec<&str> = line_trimmed.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                continue;
            }
            let key = parts[0].to_lowercase();
            let val = parts[1].trim();

            if multi.contains(key.as_str()) {
                let entry = raw_output
                    .entry(key.clone())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = entry {
                    arr.push(Value::String(val.to_string()));
                }
                continue;
            }

            if modified.contains(key.as_str())
                && let Some(first_char) = val.chars().next()
                && modifiers.contains(&first_char)
            {
                raw_output.insert(key.clone(), Value::String(val[1..].to_string()));
                raw_output.insert(
                    format!("{}_strategy", key),
                    Value::String(first_char.to_string()),
                );
                continue;
            }

            raw_output.insert(key, Value::String(val.to_string()));
        }

        let processed = process_output(raw_output);
        Ok(ParseOutput::Object(processed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sshd_conf_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/sshd_config");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/sshd_config.json"
        ))
        .unwrap();
        let parser = SshdConfParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }

    #[test]
    fn test_sshd_t_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/sshd-T.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/sshd-T.json"
        ))
        .unwrap();
        let parser = SshdConfParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }

    #[test]
    fn test_sshd_t_2_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/sshd-T-2.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/sshd-T-2.json"
        ))
        .unwrap();
        let parser = SshdConfParser;
        let result = parser.parse(input, false).unwrap();
        let result_val = serde_json::to_value(result).unwrap();
        assert_eq!(result_val, expected);
    }
}
