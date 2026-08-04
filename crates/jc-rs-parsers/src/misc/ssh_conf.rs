use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};
use std::collections::HashSet;

pub struct SshConfParser;

static INFO: ParserInfo = ParserInfo {
    name: "ssh_conf",
    argument: "--ssh-conf",
    version: "1.0.0",
    description: "`ssh` config file and `ssh -G` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::File],
    magic_commands: &["ssh -G"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SSH_CONF_PARSER: SshConfParser = SshConfParser;

inventory::submit! {
    ParserEntry::new(&SSH_CONF_PARSER)
}

// Fields that can appear multiple times (accumulated as a list of raw strings)
fn multi_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "certificatefile",
        "identityfile",
        "include",
        "localforward",
        "sendenv",
        "setenv",
    ] {
        s.insert(*f);
    }
    s
}

// Fields split on whitespace in _process
fn split_space_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "canonicaldomains",
        "globalknownhostsfile",
        "include",
        "ipqos",
        "permitremoteopen",
        "sendenv",
        "setenv",
        "userknownhostsfile",
    ] {
        s.insert(*f);
    }
    s
}

// Fields split on comma in _process
fn split_comma_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "canonicalizepermittedcnames",
        "casignaturealgorithms",
        "ciphers",
        "hostbasedacceptedalgorithms",
        "hostkeyalgorithms",
        "kbdinteractivedevices",
        "kexalgorithms",
        "logverbose",
        "macs",
        "preferredauthentications",
        "proxyjump",
        "pubkeyacceptedalgorithms",
    ] {
        s.insert(*f);
    }
    s
}

// Fields converted to int in _process
fn int_fields() -> HashSet<&'static str> {
    let mut s = HashSet::new();
    for f in &[
        "canonicalizemaxdots",
        "connectionattempts",
        "connecttimeout",
        "forwardx11timeout",
        "numberofpasswordprompts",
        "port",
        "protocol",
        "requiredrsasize",
        "serveralivecountmax",
        "serveraliveinterval",
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
        "hostkeyalgorithms",
        "kexalgorithms",
        "macs",
        "pubkeyacceptedalgorithms",
    ] {
        s.insert(*f);
    }
    s
}

fn process_host(host: Map<String, Value>) -> Map<String, Value> {
    let split_space = split_space_fields();
    let split_comma = split_comma_fields();
    let int_flds = int_fields();
    let mut result = Map::new();

    for (key, val) in &host {
        if key == "host" || key == "host_list" {
            result.insert(key.clone(), val.clone());
            continue;
        }

        // Fields that are arrays of raw strings (from multi_fields)
        if let Value::Array(items) = val {
            if key == "sendenv" || key == "setenv" || key == "include" {
                // Flatten: split each item on whitespace
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
            // Other multi-fields stay as arrays
            result.insert(key.clone(), val.clone());
            continue;
        }

        if let Value::String(s) = val {
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

impl Parser for SshConfParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let multi = multi_fields();
        let modified = modified_fields();
        let modifiers: HashSet<char> = ['+', '-', '^'].iter().cloned().collect();

        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut host: Map<String, Value> = Map::new();
        let mut match_block_found = false;

        for line in input.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line_trimmed.starts_with('#') {
                continue;
            }

            // Handle Match blocks
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

            // Host block
            if line_trimmed.starts_with("Host ") {
                if !host.is_empty() {
                    raw_output.push(host.clone());
                }
                let hostnames = line_trimmed["Host ".len()..].trim();
                host = Map::new();
                host.insert("host".to_string(), Value::String(hostnames.to_string()));
                let host_list: Vec<Value> = hostnames
                    .split_whitespace()
                    .map(|h| Value::String(h.to_string()))
                    .collect();
                host.insert("host_list".to_string(), Value::Array(host_list));
                continue;
            }

            // Parse key-value
            let parts: Vec<&str> = line_trimmed.splitn(2, char::is_whitespace).collect();
            if parts.len() < 2 {
                continue;
            }
            let key = parts[0].to_lowercase();
            let val = parts[1].trim();

            if multi.contains(key.as_str()) {
                let entry = host
                    .entry(key.clone())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = entry {
                    arr.push(Value::String(val.to_string()));
                }
                continue;
            }

            // Handle modified fields (+/- prefix)
            if modified.contains(key.as_str())
                && let Some(first_char) = val.chars().next()
                && modifiers.contains(&first_char)
            {
                host.insert(key.clone(), Value::String(val[1..].to_string()));
                host.insert(
                    format!("{}_strategy", key),
                    Value::String(first_char.to_string()),
                );
                continue;
            }

            host.insert(key, Value::String(val.to_string()));
        }

        if !host.is_empty() {
            raw_output.push(host);
        }

        // Process each host
        let processed: Vec<Map<String, Value>> = raw_output.into_iter().map(process_host).collect();

        Ok(ParseOutput::Array(processed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! ssh_conf_test {
        ($name:ident, $input:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let input = include_str!($input);
                let expected: serde_json::Value =
                    serde_json::from_str(include_str!($expected)).unwrap();
                let parser = SshConfParser;
                let result = parser.parse(input, false).unwrap();
                let result_val = serde_json::to_value(result).unwrap();
                assert_eq!(result_val, expected);
            }
        };
    }

    ssh_conf_test!(
        test_ssh_conf_1,
        "../../../../tests/fixtures/generic/ssh_config1",
        "../../../../tests/fixtures/generic/ssh_config1.json"
    );
    ssh_conf_test!(
        test_ssh_conf_2,
        "../../../../tests/fixtures/generic/ssh_config2",
        "../../../../tests/fixtures/generic/ssh_config2.json"
    );
    ssh_conf_test!(
        test_ssh_conf_3,
        "../../../../tests/fixtures/generic/ssh_config3",
        "../../../../tests/fixtures/generic/ssh_config3.json"
    );
    ssh_conf_test!(
        test_ssh_conf_4,
        "../../../../tests/fixtures/generic/ssh_config4",
        "../../../../tests/fixtures/generic/ssh_config4.json"
    );
    ssh_conf_test!(
        test_ssh_conf_5,
        "../../../../tests/fixtures/generic/ssh_config5",
        "../../../../tests/fixtures/generic/ssh_config5.json"
    );
}
