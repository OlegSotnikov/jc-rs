//! Parser for `lspci -mmv` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct LspciParser;

static INFO: ParserInfo = ParserInfo {
    name: "lspci",
    argument: "--lspci",
    version: "1.1.0",
    description: "Converts `lspci -mmv` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["lspci"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static LSPCI_PARSER: LspciParser = LspciParser;

inventory::submit! {
    ParserEntry::new(&LSPCI_PARSER)
}

/// Fields that get _int variants (parsed as hex)
const INT_FIELDS: &[&str] = &[
    "domain",
    "bus",
    "dev",
    "function",
    "class_id",
    "vendor_id",
    "device_id",
    "svendor_id",
    "sdevice_id",
    "progif",
];

impl Parser for LspciParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        // Regex-like patterns for bracket IDs
        // item_id_p: exactly 4 hex digits
        // item_id_bracket_p: string [xxxx] at end
        let mut raw_output: Vec<Map<String, Value>> = Vec::new();
        let mut device_output: Map<String, Value> = Map::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if line.starts_with("Slot:") {
                if !device_output.is_empty() {
                    let processed = process_device(device_output);
                    raw_output.push(processed);
                    device_output = Map::new();
                }

                let slot_info = line["Slot:".len()..].trim();
                device_output.insert("slot".to_string(), Value::String(slot_info.to_string()));

                // Parse domain/bus/dev/function from slot
                let (domain, bus, dev, fun) = parse_slot(slot_info);
                device_output.insert("domain".to_string(), Value::String(domain));
                device_output.insert("bus".to_string(), Value::String(bus));
                device_output.insert("dev".to_string(), Value::String(dev));
                device_output.insert("function".to_string(), Value::String(fun));
                continue;
            }

            // Split key: value
            if let Some(colon_pos) = line.find(':') {
                let key = line[..colon_pos].trim().to_lowercase();
                let val = line[colon_pos + 1..].trim();

                // Check for numeric-only (-nmmv): exactly 4 hex digits
                if is_pure_hex_id(val) {
                    device_output.insert(key + "_id", Value::String(val.to_string()));
                    continue;
                }

                // Check for string [xxxx] format (-nnmmv)
                if let Some(bracket_id) = extract_bracket_id(val) {
                    let string_part = val[..val.rfind('[').unwrap()].trim();
                    device_output.insert(key.clone(), Value::String(string_part.to_string()));
                    device_output.insert(key + "_id", Value::String(bracket_id));
                    continue;
                }

                // Plain string value (-mmv)
                device_output.insert(key, Value::String(val.to_string()));
            }
        }

        if !device_output.is_empty() {
            let processed = process_device(device_output);
            raw_output.push(processed);
        }

        Ok(ParseOutput::Array(raw_output))
    }
}

fn parse_slot(slot: &str) -> (String, String, String, String) {
    // Slot format: [domain:]bus:dev.function
    let parts: Vec<&str> = slot.split(':').collect();
    let (domain, bus, dev_fun) = match parts.len() {
        3 => (parts[0], parts[1], parts[2]),
        2 => ("00", parts[0], parts[1]),
        _ => ("00", "00", "0.0"),
    };

    let (dev, fun) = if let Some(dot_pos) = dev_fun.find('.') {
        (&dev_fun[..dot_pos], &dev_fun[dot_pos + 1..])
    } else {
        (dev_fun, "0")
    };

    (
        domain.to_string(),
        bus.to_string(),
        dev.to_string(),
        fun.to_string(),
    )
}

fn is_pure_hex_id(s: &str) -> bool {
    s.len() == 4 && s.chars().all(|c| c.is_ascii_hexdigit())
}

fn extract_bracket_id(s: &str) -> Option<String> {
    // Look for " [xxxx]" at the end
    if let Some(open) = s.rfind('[')
        && let Some(close) = s.rfind(']')
        && close == s.len() - 1
        && close > open
    {
        let id = &s[open + 1..close];
        if id.len() == 4 && id.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(id.to_string());
        }
    }
    None
}

fn process_device(mut device: Map<String, Value>) -> Map<String, Value> {
    let mut additions: Vec<(String, Value)> = Vec::new();

    for field in INT_FIELDS {
        if let Some(Value::String(s)) = device.get(*field)
            && let Ok(n) = i64::from_str_radix(s, 16)
        {
            additions.push((format!("{}_int", field), Value::Number(n.into())));
        }
    }

    // Also handle "physlot" if present - keep as string
    // Insert _int fields after their corresponding string fields
    for (k, v) in additions {
        // Find position of the base key and insert after it
        device.insert(k, v);
    }

    device
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lspci_mmv_basic() {
        let input = "Slot:\t00:00.0\nClass:\tHost bridge\nVendor:\tIntel Corporation\nDevice:\t440BX/ZX/DX - 82443BX/ZX/DX Host bridge\nRev:\t01\n\nSlot:\t00:01.0\nClass:\tPCI bridge\nVendor:\tIntel Corporation\nDevice:\t440BX/ZX/DX - 82443BX/ZX/DX AGP bridge\nRev:\t01\n";
        let parser = LspciParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(
                arr[0].get("slot"),
                Some(&Value::String("00:00.0".to_string()))
            );
            assert_eq!(arr[0].get("domain"), Some(&Value::String("00".to_string())));
            assert_eq!(arr[0].get("bus"), Some(&Value::String("00".to_string())));
            assert_eq!(
                arr[0].get("class"),
                Some(&Value::String("Host bridge".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_lspci_empty() {
        let parser = LspciParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
