//! Parser for `pci.ids` database file.
//!
//! Parses the PCI ID database at https://raw.githubusercontent.com/pciutils/pciids/master/pci.ids
//!
//! Output schema:
//! ```json
//! {
//!   "vendors": {
//!     "_<vendor_id>": {
//!       "vendor_name": "...",
//!       "_<device_id>": {
//!         "device_name": "...",
//!         "_<subvendor>": {
//!           "_<subdevice>": { "subsystem_name": "..." }
//!         }
//!       }
//!     }
//!   },
//!   "classes": {
//!     "_<class_id>": {
//!       "class_name": "...",
//!       "_<subclass_id>": {
//!         "subclass_name": "...",
//!         "_<prog_if>": "..."
//!       }
//!     }
//!   }
//! }
//! ```

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

pub struct PciIdsParser;

static INFO: ParserInfo = ParserInfo {
    name: "pci_ids",
    argument: "--pci-ids",
    version: "1.1.0",
    description: "Converts `pci.ids` file content to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Windows,
    ],
    tags: &[Tag::File],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PCI_IDS_PARSER: PciIdsParser = PciIdsParser;

inventory::submit! {
    ParserEntry::new(&PCI_IDS_PARSER)
}

static VDC_HEADER_RE: OnceLock<Regex> = OnceLock::new();
static VDC_DEVICE_RE: OnceLock<Regex> = OnceLock::new();
static VDC_SUBVENDOR_RE: OnceLock<Regex> = OnceLock::new();
static CLASS_HEADER_RE: OnceLock<Regex> = OnceLock::new();
static CLASS_SUB_RE: OnceLock<Regex> = OnceLock::new();
static CLASS_PROGIF_RE: OnceLock<Regex> = OnceLock::new();

fn get_vdc_header_re() -> &'static Regex {
    VDC_HEADER_RE.get_or_init(|| Regex::new(r"^([0-9a-f]{4})\s+(.+)").unwrap())
}

fn get_vdc_device_re() -> &'static Regex {
    VDC_DEVICE_RE.get_or_init(|| Regex::new(r"^\t([0-9a-f]{4})\s+(.+)").unwrap())
}

fn get_vdc_subvendor_re() -> &'static Regex {
    VDC_SUBVENDOR_RE
        .get_or_init(|| Regex::new(r"^\t\t([0-9a-f]{4})\s+([0-9a-f]{4})\s+(.+)").unwrap())
}

fn get_class_header_re() -> &'static Regex {
    CLASS_HEADER_RE.get_or_init(|| Regex::new(r"^C\s+([0-9a-f]{2})\s+(.+)").unwrap())
}

fn get_class_sub_re() -> &'static Regex {
    CLASS_SUB_RE.get_or_init(|| Regex::new(r"^\t([0-9a-f]{2})\s+(.+)").unwrap())
}

fn get_class_progif_re() -> &'static Regex {
    CLASS_PROGIF_RE.get_or_init(|| Regex::new(r"^\t\t([0-9a-f]{2})\s+(.+)").unwrap())
}

impl Parser for PciIdsParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut raw_output: Map<String, Value> = Map::new();
        let mut vendors: Map<String, Value> = Map::new();
        let mut classes: Map<String, Value> = Map::new();

        // Current vendor context
        let mut vendor_id = String::new();
        let mut vendor_obj: Map<String, Value> = Map::new();

        // Current device context
        let mut device_id = String::new();

        // Current class context
        let mut class_id = String::new();
        let mut class_obj: Map<String, Value> = Map::new();

        // Current subclass context
        let mut subclass_id = String::new();

        // Track whether we're in vendor or class section
        let mut in_classes = false;

        for line in input.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Try class header first (starts with "C ")
            if let Some(caps) = get_class_header_re().captures(line) {
                // Flush previous class
                if !class_id.is_empty() {
                    classes.insert(class_id.clone(), Value::Object(class_obj.clone()));
                    class_obj = Map::new();
                }
                // Flush current vendor if any
                if !vendor_id.is_empty() && !in_classes {
                    vendors.insert(vendor_id.clone(), Value::Object(vendor_obj.clone()));
                    vendor_obj = Map::new();
                    vendor_id.clear();
                }
                in_classes = true;
                class_id = format!("_{}", &caps[1]);
                class_obj.insert("class_name".to_string(), Value::String(caps[2].to_string()));
                subclass_id.clear();
                continue;
            }

            if in_classes {
                // Try prog_if (double tab)
                if let Some(caps) = get_class_progif_re().captures(line) {
                    let prog_if_id = format!("_{}", &caps[1]);
                    if let Some(Value::Object(sub)) = class_obj.get_mut(&subclass_id) {
                        sub.insert(prog_if_id, Value::String(caps[2].to_string()));
                    }
                    continue;
                }

                // Try subclass (single tab)
                if let Some(caps) = get_class_sub_re().captures(line) {
                    subclass_id = format!("_{}", &caps[1]);
                    let mut sub_obj = Map::new();
                    sub_obj.insert(
                        "subclass_name".to_string(),
                        Value::String(caps[2].to_string()),
                    );
                    class_obj.insert(subclass_id.clone(), Value::Object(sub_obj));
                    continue;
                }
            } else {
                // Vendor section

                // Try subvendor/subdevice (double tab)
                if let Some(caps) = get_vdc_subvendor_re().captures(line) {
                    let subvendor = format!("_{}", &caps[1]);
                    let subdevice = format!("_{}", &caps[2]);
                    let subsystem_name = caps[3].to_string();

                    // Get or create device entry
                    if let Some(Value::Object(vobj)) = vendor_obj.get_mut(&device_id) {
                        let sub_entry = vobj
                            .entry(subvendor.clone())
                            .or_insert_with(|| Value::Object(Map::new()));
                        if let Value::Object(sv_obj) = sub_entry {
                            let mut sd_obj = Map::new();
                            sd_obj.insert(
                                "subsystem_name".to_string(),
                                Value::String(subsystem_name),
                            );
                            sv_obj.insert(subdevice, Value::Object(sd_obj));
                        }
                    }
                    continue;
                }

                // Try device (single tab)
                if let Some(caps) = get_vdc_device_re().captures(line) {
                    device_id = format!("_{}", &caps[1]);
                    let mut dev_obj = Map::new();
                    dev_obj.insert(
                        "device_name".to_string(),
                        Value::String(caps[2].to_string()),
                    );
                    vendor_obj.insert(device_id.clone(), Value::Object(dev_obj));
                    continue;
                }

                // Try vendor header (no leading tab)
                if let Some(caps) = get_vdc_header_re().captures(line) {
                    // Flush previous vendor
                    if !vendor_id.is_empty() {
                        vendors.insert(vendor_id.clone(), Value::Object(vendor_obj.clone()));
                        vendor_obj = Map::new();
                    }
                    vendor_id = format!("_{}", &caps[1]);
                    vendor_obj.insert(
                        "vendor_name".to_string(),
                        Value::String(caps[2].to_string()),
                    );
                    device_id.clear();
                    continue;
                }
            }
        }

        // Flush last vendor
        if !vendor_id.is_empty() {
            vendors.insert(vendor_id, Value::Object(vendor_obj));
        }

        // Flush last class
        if !class_id.is_empty() {
            classes.insert(class_id, Value::Object(class_obj));
        }

        if !vendors.is_empty() {
            raw_output.insert("vendors".to_string(), Value::Object(vendors));
        }
        if !classes.is_empty() {
            raw_output.insert("classes".to_string(), Value::Object(classes));
        }

        Ok(ParseOutput::Object(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"# PCI ID database
0001  SafeNet (wrong ID)
	0001  SafeNet USB SuperPro/UltraPro Key

001c  PEAK-System Technik GmbH
	0001  PCAN-PCI CAN-Bus controller
		001c 0005  2 Channel CAN Bus SJC1000 (Optically Isolated)

C 01  Mass storage controller
	01  IDE interface
		00  ISA Compatibility mode-only controller

C 0c  Serial bus controller
	03  USB controller
		40  USB4 Host Interface
"#;

    #[test]
    fn test_pci_ids_basic() {
        let parser = PciIdsParser;
        let result = parser.parse(SAMPLE, false).unwrap();
        match result {
            ParseOutput::Object(obj) => {
                // Check vendors
                let vendors = obj.get("vendors").and_then(|v| v.as_object()).unwrap();
                assert!(vendors.contains_key("_0001"), "vendor _0001 missing");
                let v0001 = vendors["_0001"].as_object().unwrap();
                assert_eq!(
                    v0001["vendor_name"],
                    Value::String("SafeNet (wrong ID)".to_string())
                );
                assert!(v0001.contains_key("_0001"), "device _0001 missing");

                let v001c = vendors.get("_001c").and_then(|v| v.as_object()).unwrap();
                assert_eq!(
                    v001c["vendor_name"],
                    Value::String("PEAK-System Technik GmbH".to_string())
                );
                let dev0001 = v001c.get("_0001").and_then(|v| v.as_object()).unwrap();
                assert_eq!(
                    dev0001["device_name"],
                    Value::String("PCAN-PCI CAN-Bus controller".to_string())
                );
                let sv001c = dev0001.get("_001c").and_then(|v| v.as_object()).unwrap();
                let sd0005 = sv001c.get("_0005").and_then(|v| v.as_object()).unwrap();
                assert_eq!(
                    sd0005["subsystem_name"],
                    Value::String("2 Channel CAN Bus SJC1000 (Optically Isolated)".to_string())
                );

                // Check classes
                let classes = obj.get("classes").and_then(|v| v.as_object()).unwrap();
                assert!(classes.contains_key("_01"));
                let c01 = classes["_01"].as_object().unwrap();
                assert_eq!(
                    c01["class_name"],
                    Value::String("Mass storage controller".to_string())
                );
                let sub01 = c01.get("_01").and_then(|v| v.as_object()).unwrap();
                assert_eq!(
                    sub01["subclass_name"],
                    Value::String("IDE interface".to_string())
                );
                assert_eq!(
                    sub01["_00"],
                    Value::String("ISA Compatibility mode-only controller".to_string())
                );

                let c0c = classes.get("_0c").and_then(|v| v.as_object()).unwrap();
                let sub03 = c0c.get("_03").and_then(|v| v.as_object()).unwrap();
                assert_eq!(
                    sub03["_40"],
                    Value::String("USB4 Host Interface".to_string())
                );
            }
            _ => panic!("expected Object"),
        }
    }

    #[test]
    fn test_pci_ids_empty() {
        let parser = PciIdsParser;
        let result = parser.parse("", false).unwrap();
        match result {
            ParseOutput::Object(obj) => assert!(obj.is_empty()),
            _ => panic!("expected Object"),
        }
    }
}
