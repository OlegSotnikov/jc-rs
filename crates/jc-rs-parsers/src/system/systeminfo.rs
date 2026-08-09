//! Parser for `systeminfo` command output (Windows).

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct SysteminfoParser;

static INFO: ParserInfo = ParserInfo {
    name: "systeminfo",
    argument: "--systeminfo",
    version: "1.3.0",
    description: "Converts `systeminfo` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &["systeminfo"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSTEMINFO_PARSER: SysteminfoParser = SysteminfoParser;

inventory::submit! {
    ParserEntry::new(&SYSTEMINFO_PARSER)
}

impl Parser for SysteminfoParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_systeminfo(input)?;
        Ok(ParseOutput::Object(out))
    }
}

fn transform_key(key: &str) -> String {
    let k = key.trim().to_lowercase().replace(' ', "_");
    // Remove invalid key characters
    k.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn get_value_pos(line: &str, delim: char) -> Option<usize> {
    if let Some(pos) = line.find(delim) {
        let after = &line[pos + 1..];
        let trimmed = after.trim_start();
        Some(line.len() - trimmed.len())
    } else {
        None
    }
}

fn parse_hotfixes_or_processors(data: &str) -> Vec<Value> {
    let mut result = Vec::new();
    for (i, line) in data.lines().enumerate() {
        if i == 0 {
            continue; // skip count line
        }
        if line.contains(':') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                result.push(Value::String(parts[1].trim().to_string()));
            }
        }
    }
    result
}

fn parse_hyperv_requirements(data: &str) -> Map<String, Value> {
    let mut result = Map::new();
    for line in data.lines() {
        if line.contains(':') {
            let parts: Vec<&str> = line.splitn(2, ':').collect();
            if parts.len() == 2 {
                let k = transform_key(parts[0]);
                result.insert(k, Value::String(parts[1].trim().to_string()));
            }
        }
    }
    result
}

fn default_nic() -> Map<String, Value> {
    let mut nic = Map::new();
    nic.insert("name".to_string(), Value::String(String::new()));
    nic.insert("connection_name".to_string(), Value::String(String::new()));
    nic.insert("status".to_string(), Value::String(String::new()));
    nic.insert("dhcp_enabled".to_string(), Value::String("No".to_string()));
    nic.insert("dhcp_server".to_string(), Value::String(String::new()));
    nic.insert("ip_addresses".to_string(), Value::Array(Vec::new()));
    nic
}

fn parse_network_cards(data: &str) -> Vec<Value> {
    let mut result: Vec<Value> = Vec::new();
    let mut cur_nic: Option<Map<String, Value>> = None;
    let mut is_ip = false;
    let mut nic_value_pos: usize = 0;

    for (i, line) in data.lines().enumerate() {
        if i == 0 {
            continue; // skip count line
        }

        if line.contains("IP address(es)") {
            is_ip = true;
            continue;
        }

        let cur_value_pos = line.len() - line.trim_start().len();
        let line_trimmed = line.trim();

        // Check for [N]: pattern
        if let Some(bracket_end) = line_trimmed.find("]:")
            && line_trimmed.starts_with('[')
        {
            let _num_str = &line_trimmed[1..bracket_end];
            let val = line_trimmed[bracket_end + 2..].trim();

            if is_ip && cur_value_pos > nic_value_pos {
                // IP address entry
                if let Some(ref mut nic) = cur_nic
                    && let Some(Value::Array(ips)) = nic.get_mut("ip_addresses")
                {
                    ips.push(Value::String(val.to_string()));
                }
            } else {
                // New NIC
                if let Some(nic) = cur_nic.take() {
                    result.push(Value::Object(nic));
                }
                let mut new_nic = default_nic();
                new_nic.insert("name".to_string(), Value::String(val.to_string()));
                nic_value_pos = cur_value_pos;
                cur_nic = Some(new_nic);
                is_ip = false;
            }
            continue;
        }

        // Key: Value lines
        if line_trimmed.contains(':') {
            let parts: Vec<&str> = line_trimmed.splitn(2, ':').collect();
            if parts.len() == 2 {
                let k = transform_key(parts[0]);
                let v = parts[1].trim().to_string();
                if let Some(ref mut nic) = cur_nic {
                    nic.insert(k, Value::String(v));
                }
            }
        }
    }

    if let Some(nic) = cur_nic {
        result.push(Value::Object(nic));
    }

    result
}

fn parse_systeminfo(input: &str) -> Result<Map<String, Value>, ParseError> {
    if input.trim().is_empty() {
        return Ok(Map::new());
    }

    let lines: Vec<&str> = input.lines().filter(|l| !l.trim().is_empty()).collect();

    if lines.is_empty() {
        return Ok(Map::new());
    }

    // Find value position from first line
    let start_value_pos = get_value_pos(lines[0], ':').ok_or_else(|| {
        ParseError::InvalidInput("Cannot find delimiter in first line".to_string())
    })?;

    // Collect raw key/value pairs (handling multiline values)
    let mut raw_data: Vec<(String, String)> = Vec::new();
    let mut last_key_idx: Option<usize> = None;

    for line in &lines {
        if line.len() < start_value_pos {
            // Possibly continuation
            if let Some(idx) = last_key_idx {
                raw_data[idx].1.push('\n');
                raw_data[idx].1.push_str(line);
            }
            continue;
        }

        let key_part = &line[..start_value_pos];
        let val_part = &line[start_value_pos..];

        if !key_part.contains(':') {
            // Continuation line
            if let Some(idx) = last_key_idx {
                raw_data[idx].1.push('\n');
                raw_data[idx].1.push_str(line);
            }
        } else {
            raw_data.push((key_part.to_string(), val_part.to_string()));
            last_key_idx = Some(raw_data.len() - 1);
        }
    }

    let mut out = Map::new();

    for (raw_key, raw_val) in raw_data {
        let key = transform_key(raw_key.trim_end_matches(':').trim());
        let val = raw_val.trim().to_string();

        match key.as_str() {
            "hotfixs" | "processors" => {
                out.insert(key, Value::Array(parse_hotfixes_or_processors(&val)));
            }
            "network_cards" => {
                out.insert(key, Value::Array(parse_network_cards(&val)));
            }
            "hyperv_requirements" => {
                out.insert(key, Value::Object(parse_hyperv_requirements(&val)));
            }
            "total_physical_memory"
            | "available_physical_memory"
            | "virtual_memory_max_size"
            | "virtual_memory_available"
            | "virtual_memory_in_use" => {
                out.insert(key + "_mb", Value::String(val));
            }
            _ => {
                out.insert(key, Value::String(val));
            }
        }
    }

    // Post-process: convert types
    post_process(&mut out);

    Ok(out)
}

fn convert_to_int_mb(s: &str) -> Option<i64> {
    // "32,503 MB" -> 32503
    let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn post_process(out: &mut Map<String, Value>) {
    let int_mb_keys = [
        "total_physical_memory_mb",
        "available_physical_memory_mb",
        "virtual_memory_max_size_mb",
        "virtual_memory_available_mb",
        "virtual_memory_in_use_mb",
    ];

    for key in &int_mb_keys {
        if let Some(Value::String(s)) = out.get(*key).cloned()
            && let Some(n) = convert_to_int_mb(&s)
        {
            out.insert(key.to_string(), Value::Number(n.into()));
        }
    }

    // Convert empty strings to null
    for val in out.values_mut() {
        if let Value::String(s) = val
            && s.is_empty()
        {
            *val = Value::Null;
        }
    }

    // Convert network card dhcp_enabled bool and empty strings
    if let Some(Value::Array(nics)) = out.get_mut("network_cards") {
        for nic_val in nics.iter_mut() {
            if let Value::Object(nic) = nic_val {
                if let Some(Value::String(s)) = nic.get("dhcp_enabled").cloned() {
                    let b = s.to_lowercase() == "yes";
                    nic.insert("dhcp_enabled".to_string(), Value::Bool(b));
                }
                // Convert empty strings to null
                let keys: Vec<String> = nic.keys().cloned().collect();
                for k in keys {
                    if let Some(Value::String(s)) = nic.get(&k)
                        && s.is_empty()
                    {
                        nic.insert(k, Value::Null);
                    }
                }
            }
        }
    }

    // Convert hyperv requirement bool fields
    let hyperv_bool_keys = [
        "vm_monitor_mode_extensions",
        "virtualization_enabled_in_firmware",
        "second_level_address_translation",
        "data_execution_prevention_available",
    ];

    if let Some(Value::Object(hyperv)) = out.get_mut("hyperv_requirements") {
        for key in &hyperv_bool_keys {
            if let Some(Value::String(s)) = hyperv.get(*key).cloned() {
                let b = s.to_lowercase() == "yes";
                hyperv.insert(key.to_string(), Value::Bool(b));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systeminfo_basic() {
        let input = "Host Name:                 TESTLAPTOP\n\
                     OS Name:                   Microsoft Windows 10 Enterprise\n\
                     Total Physical Memory:     32,503 MB\n\
                     Available Physical Memory: 19,743 MB\n\
                     Virtual Memory: Max Size:  37,367 MB\n\
                     Virtual Memory: Available: 22,266 MB\n\
                     Virtual Memory: In Use:    15,101 MB\n\
                     Hotfix(s):                 2 Hotfix(s) Installed.\n\
                                                [01]: KB2693643\n\
                                                [02]: KB4601054\n\
                     Network Card(s):           1 NIC(s) Installed.\n\
                                                [01]: Intel(R) Wireless-AC 9260\n\
                                                      Connection Name: Wi-Fi\n\
                                                      DHCP Enabled:    Yes\n\
                                                      DHCP Server:     192.168.2.1\n\
                                                      IP address(es)\n\
                                                      [01]: 192.168.2.219\n\
                     Hyper-V Requirements:      VM Monitor Mode Extensions: Yes\n\
                                                Virtualization Enabled In Firmware: Yes\n\
                                                Second Level Address Translation: No\n\
                                                Data Execution Prevention Available: Yes";

        let parser = SysteminfoParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("host_name"),
                Some(&Value::String("TESTLAPTOP".to_string()))
            );
            assert_eq!(
                obj.get("os_name"),
                Some(&Value::String(
                    "Microsoft Windows 10 Enterprise".to_string()
                ))
            );
            // MB fields converted to int
            assert_eq!(
                obj.get("total_physical_memory_mb"),
                Some(&Value::Number(32503_i64.into()))
            );
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_systeminfo_empty() {
        let parser = SysteminfoParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
