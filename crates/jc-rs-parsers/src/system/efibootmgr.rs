//! Parser for `efibootmgr` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct EfibootmgrParser;

static INFO: ParserInfo = ParserInfo {
    name: "efibootmgr",
    argument: "--efibootmgr",
    version: "1.0.0",
    description: "Converts `efibootmgr` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["efibootmgr"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static EFIBOOTMGR_PARSER: EfibootmgrParser = EfibootmgrParser;

inventory::submit! {
    ParserEntry::new(&EFIBOOTMGR_PARSER)
}

impl Parser for EfibootmgrParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_efibootmgr(input);
        Ok(ParseOutput::Object(out))
    }
}

fn parse_efibootmgr(input: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut boot_options: Vec<Value> = Vec::new();

    for line in input.lines() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }

        if line.starts_with("BootCurrent:") {
            let val = line["BootCurrent:".len()..].trim().to_string();
            out.insert("boot_current".to_string(), Value::String(val));
        } else if line.starts_with("Timeout:") {
            // "Timeout: 0 seconds"
            let val_str = line["Timeout:".len()..].trim();
            // Extract just the number
            let num_str: String = val_str.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<i64>() {
                out.insert("timeout_seconds".to_string(), Value::Number(n.into()));
            }
        } else if line.starts_with("BootOrder:") {
            let val = line["BootOrder:".len()..].trim();
            let order: Vec<Value> = val
                .split(',')
                .map(|s| Value::String(s.trim().to_string()))
                .collect();
            out.insert("boot_order".to_string(), Value::Array(order));
        } else if line.starts_with("Boot") && line.len() > 8 {
            // Boot entry lines: "Boot0000* WARNADO\tHD(...)..."
            // or "Boot0000  WARNADO\tHD(...)"
            // Format: Boot[4hex][enabled_char][space][display_name]\t[uefi_path]
            let mut boot_opt = Map::new();

            // boot_option_reference is first 8 chars: "Boot0000"
            let boot_ref = &line[..8.min(line.len())];
            boot_opt.insert(
                "boot_option_reference".to_string(),
                Value::String(boot_ref.to_string()),
            );

            if line.len() > 8 {
                let enabled_char = line.chars().nth(8).unwrap_or(' ');
                let boot_option_enabled = enabled_char == '*';
                boot_opt.insert(
                    "boot_option_enabled".to_string(),
                    Value::Bool(boot_option_enabled),
                );

                // Remainder after the enabled char
                let remainder = &line[9..].trim_start();

                // Split on tab to separate display_name and uefi_device_path
                if let Some(tab_pos) = remainder.find('\t') {
                    let display_name = remainder[..tab_pos].trim().to_string();
                    let uefi_path = remainder[tab_pos + 1..].trim().to_string();
                    boot_opt.insert("display_name".to_string(), Value::String(display_name));
                    boot_opt.insert("uefi_device_path".to_string(), Value::String(uefi_path));
                } else {
                    let display_name = remainder.trim().to_string();
                    boot_opt.insert("display_name".to_string(), Value::String(display_name));
                }
            }

            boot_options.push(Value::Object(boot_opt));
        } else if line.starts_with("MirroredPercentageAbove4G:") {
            let val_str = line["MirroredPercentageAbove4G:".len()..].trim();
            if let Ok(f) = val_str.parse::<f64>() {
                out.insert(
                    "mirrored_percentage_above_4g".to_string(),
                    serde_json::Number::from_f64(f)
                        .map(Value::Number)
                        .unwrap_or(Value::String(val_str.to_string())),
                );
            }
        } else if line.starts_with("MirrorMemoryBelow4GB:") {
            let val_str = line["MirrorMemoryBelow4GB:".len()..].trim().to_lowercase();
            let b = val_str != "false";
            out.insert("mirror_memory_below_4gb".to_string(), Value::Bool(b));
        }
    }

    out.insert("boot_options".to_string(), Value::Array(boot_options));

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efibootmgr_basic() {
        let input = "BootCurrent: 0002\n\
                     Timeout: 0 seconds\n\
                     BootOrder: 0002,0000,0001\n\
                     Boot0000* WARNADO\tHD(1,GPT,05b9...)\n\
                     Boot0001* Embedded NIC\tVenHw(3a191845)\n\
                     Boot0002* opensuse-secureboot\tHD(1,GPT,c5d4...)";

        let parser = EfibootmgrParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("boot_current"),
                Some(&Value::String("0002".to_string()))
            );
            assert_eq!(obj.get("timeout_seconds"), Some(&Value::Number(0.into())));
            if let Some(Value::Array(order)) = obj.get("boot_order") {
                assert_eq!(order.len(), 3);
                assert_eq!(order[0], Value::String("0002".to_string()));
            } else {
                panic!("Expected boot_order array");
            }
            if let Some(Value::Array(opts)) = obj.get("boot_options") {
                assert_eq!(opts.len(), 3);
                if let Some(Value::Object(opt)) = opts.get(0) {
                    assert_eq!(
                        opt.get("boot_option_reference"),
                        Some(&Value::String("Boot0000".to_string()))
                    );
                    assert_eq!(opt.get("boot_option_enabled"), Some(&Value::Bool(true)));
                }
            } else {
                panic!("Expected boot_options array");
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_efibootmgr_empty() {
        let parser = EfibootmgrParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            // boot_options should still be present as empty array
            if let Some(Value::Array(opts)) = obj.get("boot_options") {
                assert!(opts.is_empty());
            } else {
                panic!("Expected empty boot_options array");
            }
        } else {
            panic!("Expected Object");
        }
    }
}
