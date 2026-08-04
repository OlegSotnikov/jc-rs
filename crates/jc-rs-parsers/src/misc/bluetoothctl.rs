//! Parser for `bluetoothctl` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

pub struct BluetoothctlParser;

static INFO: ParserInfo = ParserInfo {
    name: "bluetoothctl",
    argument: "--bluetoothctl",
    version: "1.5.0",
    description: "`bluetoothctl` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["bluetoothctl"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static BLUETOOTHCTL_PARSER: BluetoothctlParser = BluetoothctlParser;

inventory::submit! {
    ParserEntry::new(&BLUETOOTHCTL_PARSER)
}

/// Parse a controller block. Returns None if the first line doesn't match.
fn parse_controller(lines: &mut Vec<&str>) -> Option<Map<String, Value>> {
    let first = lines.pop()?;

    let head_re =
        Regex::new(r"^Controller (?P<address>([0-9A-F]{2}:){5}[0-9A-F]{2}) (?P<name>.+)$").ok()?;

    let caps = head_re.captures(first)?;
    let address = caps.name("address").map_or("", |m| m.as_str()).to_string();
    let mut name = caps.name("name").map_or("", |m| m.as_str()).to_string();

    if name.ends_with("not available") {
        return None;
    }

    let mut ctrl = Map::new();
    ctrl.insert("manufacturer".to_string(), Value::String(String::new()));
    ctrl.insert("version".to_string(), Value::String(String::new()));
    ctrl.insert("name".to_string(), Value::String(String::new()));
    ctrl.insert("is_default".to_string(), Value::Bool(false));
    ctrl.insert("is_public".to_string(), Value::Bool(false));
    ctrl.insert("is_random".to_string(), Value::Bool(false));
    ctrl.insert("address".to_string(), Value::String(address));
    ctrl.insert("alias".to_string(), Value::String(String::new()));
    ctrl.insert("class".to_string(), Value::String(String::new()));
    ctrl.insert("powered".to_string(), Value::String(String::new()));
    ctrl.insert("power_state".to_string(), Value::String(String::new()));
    ctrl.insert("discoverable".to_string(), Value::String(String::new()));
    ctrl.insert(
        "discoverable_timeout".to_string(),
        Value::String(String::new()),
    );
    ctrl.insert("pairable".to_string(), Value::String(String::new()));
    ctrl.insert("modalias".to_string(), Value::String(String::new()));
    ctrl.insert("discovering".to_string(), Value::String(String::new()));
    ctrl.insert("uuids".to_string(), Value::Array(Vec::new()));

    if name.ends_with("[default]") {
        ctrl.insert("is_default".to_string(), Value::Bool(true));
        name = name.replace("[default]", "").trim().to_string();
    } else if name.ends_with("(public)") {
        ctrl.insert("is_public".to_string(), Value::Bool(true));
        name = name.replace("(public)", "").trim().to_string();
    } else if name.ends_with("(random)") {
        ctrl.insert("is_random".to_string(), Value::Bool(true));
        name = name.replace("(random)", "").trim().to_string();
    }
    ctrl.insert("name".to_string(), Value::String(name));

    let line_re = Regex::new(
        r"(?x)
        ^\s*Manufacturer:\s*(?P<manufacturer>.+)
        |^\s*Version:\s*(?P<version>.+)
        |^\s*Name:\s*(?P<name>.+)
        |^\s*Alias:\s*(?P<alias>.+)
        |^\s*Class:\s*(?P<class>.+)
        |^\s*Powered:\s*(?P<powered>.+)
        |^\s*PowerState:\s*(?P<power_state>.+)
        |^\s*Discoverable:\s*(?P<discoverable>.+)
        |^\s*DiscoverableTimeout:\s*(?P<discoverable_timeout>.+)
        |^\s*Pairable:\s*(?P<pairable>.+)
        |^\s*Modalias:\s*(?P<modalias>.+)
        |^\s*Discovering:\s*(?P<discovering>.+)
        |^\s*UUID:\s*(?P<uuid>.+)
        ",
    )
    .ok()?;

    while let Some(line) = lines.last() {
        if let Some(caps) = line_re.captures(line) {
            lines.pop();

            if let Some(m) = caps.name("manufacturer") {
                ctrl.insert(
                    "manufacturer".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("version") {
                ctrl.insert("version".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("name") {
                ctrl.insert("name".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("alias") {
                ctrl.insert("alias".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("class") {
                ctrl.insert("class".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("powered") {
                ctrl.insert("powered".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("power_state") {
                ctrl.insert(
                    "power_state".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("discoverable") {
                ctrl.insert(
                    "discoverable".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("discoverable_timeout") {
                ctrl.insert(
                    "discoverable_timeout".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("pairable") {
                ctrl.insert(
                    "pairable".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("modalias") {
                ctrl.insert(
                    "modalias".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("discovering") {
                ctrl.insert(
                    "discovering".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("uuid") {
                if let Some(Value::Array(uuids)) = ctrl.get_mut("uuids") {
                    uuids.push(Value::String(m.as_str().to_string()));
                }
            }
        } else {
            // Line doesn't match any pattern → stop
            break;
        }
    }

    Some(ctrl)
}

/// Parse a device block. Returns None if the first line doesn't match.
fn parse_device(lines: &mut Vec<&str>, quiet: bool) -> Option<Map<String, Value>> {
    let first = lines.pop()?;

    let head_re =
        Regex::new(r"^Device (?P<address>([0-9A-F]{2}:){5}[0-9A-F]{2}) (?P<name>.+)$").ok()?;

    let caps = head_re.captures(first)?;
    let address = caps.name("address").map_or("", |m| m.as_str()).to_string();
    let mut name = caps.name("name").map_or("", |m| m.as_str()).to_string();

    if name.ends_with("not available") {
        return None;
    }

    let mut dev = Map::new();
    dev.insert("name".to_string(), Value::String(String::new()));
    dev.insert("is_public".to_string(), Value::Bool(false));
    dev.insert("is_random".to_string(), Value::Bool(false));
    dev.insert("address".to_string(), Value::String(address));
    dev.insert("alias".to_string(), Value::String(String::new()));
    dev.insert("appearance".to_string(), Value::String(String::new()));
    dev.insert("class".to_string(), Value::String(String::new()));
    dev.insert("icon".to_string(), Value::String(String::new()));
    dev.insert("paired".to_string(), Value::String(String::new()));
    dev.insert("bonded".to_string(), Value::String(String::new()));
    dev.insert("trusted".to_string(), Value::String(String::new()));
    dev.insert("blocked".to_string(), Value::String(String::new()));
    dev.insert("connected".to_string(), Value::String(String::new()));
    dev.insert("legacy_pairing".to_string(), Value::String(String::new()));
    dev.insert("cable_pairing".to_string(), Value::String(String::new()));
    dev.insert("rssi".to_string(), Value::from(0i64));
    dev.insert("txpower".to_string(), Value::from(0i64));
    dev.insert("uuids".to_string(), Value::Array(Vec::new()));
    dev.insert("modalias".to_string(), Value::String(String::new()));
    dev.insert("battery_percentage".to_string(), Value::from(0i64));

    if name.ends_with("(public)") {
        dev.insert("is_public".to_string(), Value::Bool(true));
        name = name.replace("(public)", "").trim().to_string();
    } else if name.ends_with("(random)") {
        dev.insert("is_random".to_string(), Value::Bool(true));
        name = name.replace("(random)", "").trim().to_string();
    }
    dev.insert("name".to_string(), Value::String(name));

    let line_re = Regex::new(
        r"(?x)
        ^\s*Name:\s*(?P<name>.+)
        |^\s*Alias:\s*(?P<alias>.+)
        |^\s*Appearance:\s*(?P<appearance>.+)
        |^\s*Class:\s*(?P<class>.+)
        |^\s*Icon:\s*(?P<icon>.+)
        |^\s*Paired:\s*(?P<paired>.+)
        |^\s*Bonded:\s*(?P<bonded>.+)
        |^\s*Trusted:\s*(?P<trusted>.+)
        |^\s*Blocked:\s*(?P<blocked>.+)
        |^\s*Connected:\s*(?P<connected>.+)
        |^\s*LegacyPairing:\s*(?P<legacy_pairing>.+)
        |^\s*Modalias:\s*(?P<modalias>.+)
        |^\s*RSSI:\s*(?P<rssi>.+)
        |^\s*TxPower:\s*(?P<txpower>.+)
        |^\s*Battery\s+Percentage:\s*0[xX][0-9a-fA-F]*\s*\((?P<battery_percentage>[0-9]+)\)
        |^\s*UUID:\s*(?P<uuid>.+)
        |^\s*CablePairing:\s*(?P<cable_pairing>.+)
        ",
    )
    .ok()?;

    while let Some(line) = lines.last() {
        if let Some(caps) = line_re.captures(line) {
            lines.pop();

            if let Some(m) = caps.name("name") {
                dev.insert("name".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("alias") {
                dev.insert("alias".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("appearance") {
                dev.insert(
                    "appearance".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("class") {
                dev.insert("class".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("icon") {
                dev.insert("icon".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("paired") {
                dev.insert("paired".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("bonded") {
                dev.insert("bonded".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("trusted") {
                dev.insert("trusted".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("blocked") {
                dev.insert("blocked".to_string(), Value::String(m.as_str().to_string()));
            } else if let Some(m) = caps.name("connected") {
                dev.insert(
                    "connected".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("legacy_pairing") {
                dev.insert(
                    "legacy_pairing".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("cable_pairing") {
                dev.insert(
                    "cable_pairing".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("rssi") {
                let rssi_str = m.as_str();
                match rssi_str.parse::<i64>() {
                    Ok(v) => {
                        dev.insert("rssi".to_string(), Value::from(v));
                    }
                    Err(_) => {
                        if !quiet {
                            eprintln!("bluetoothctl: rssi - {} is not int-able", rssi_str);
                        }
                    }
                }
            } else if let Some(m) = caps.name("txpower") {
                let tx_str = m.as_str();
                match tx_str.parse::<i64>() {
                    Ok(v) => {
                        dev.insert("txpower".to_string(), Value::from(v));
                    }
                    Err(_) => {
                        if !quiet {
                            eprintln!("bluetoothctl: txpower - {} is not int-able", tx_str);
                        }
                    }
                }
            } else if let Some(m) = caps.name("uuid") {
                if let Some(Value::Array(uuids)) = dev.get_mut("uuids") {
                    uuids.push(Value::String(m.as_str().to_string()));
                }
            } else if let Some(m) = caps.name("modalias") {
                dev.insert(
                    "modalias".to_string(),
                    Value::String(m.as_str().to_string()),
                );
            } else if let Some(m) = caps.name("battery_percentage") {
                let bp_str = m.as_str();
                match bp_str.parse::<i64>() {
                    Ok(v) => {
                        dev.insert("battery_percentage".to_string(), Value::from(v));
                    }
                    Err(_) => {
                        if !quiet {
                            eprintln!(
                                "bluetoothctl: battery_percentage - {} is not int-able",
                                bp_str
                            );
                        }
                    }
                }
            }
        } else {
            break;
        }
    }

    Some(dev)
}

impl Parser for BluetoothctlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Determine type from the first non-empty line
        let first_line = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("");

        let is_controller = first_line.starts_with("Controller ");
        let is_device = first_line.starts_with("Device ");

        if !is_controller && !is_device {
            return Ok(ParseOutput::Array(vec![]));
        }

        // Reverse lines into a stack (so pop() gives us the first line)
        let mut line_stack: Vec<&str> = input.lines().collect();
        line_stack.reverse();

        let mut result = Vec::new();

        loop {
            // Skip empty lines from the stack top
            while line_stack
                .last()
                .map(|l| l.trim().is_empty())
                .unwrap_or(false)
            {
                line_stack.pop();
            }

            if line_stack.is_empty() {
                break;
            }

            let element = if is_controller {
                parse_controller(&mut line_stack).map(|m| m)
            } else {
                parse_device(&mut line_stack, quiet).map(|m| m)
            };

            if let Some(obj) = element {
                result.push(obj);
            } else {
                break;
            }
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = BluetoothctlParser;
        let result = parser.parse(input, true).unwrap();
        let expected: Vec<serde_json::Value> = serde_json::from_str(expected_json).unwrap();

        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), expected.len(), "record count mismatch");
            for (i, (got, exp)) in arr.iter().zip(expected.iter()).enumerate() {
                assert_eq!(
                    serde_json::Value::Object(got.clone()),
                    *exp,
                    "mismatch at row {}",
                    i
                );
            }
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_bluetoothctl_controller() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_controller.out"),
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_controller.json"),
        );
    }

    #[test]
    fn test_bluetoothctl_controller_2() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_controller_2.out"),
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_controller_2.json"),
        );
    }

    #[test]
    fn test_bluetoothctl_controller_with_manufacturer() {
        parse_fixture(
            include_str!(
                "../../../../tests/fixtures/generic/bluetoothctl_controller_with_manufacturer.out"
            ),
            include_str!(
                "../../../../tests/fixtures/generic/bluetoothctl_controller_with_manufacturer.json"
            ),
        );
    }

    #[test]
    fn test_bluetoothctl_device() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_device.out"),
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_device.json"),
        );
    }

    #[test]
    fn test_bluetoothctl_device_with_battery() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_device_with_battery.out"),
            include_str!(
                "../../../../tests/fixtures/generic/bluetoothctl_device_with_battery.json"
            ),
        );
    }

    #[test]
    fn test_bluetoothctl_device_random() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_device_random.out"),
            include_str!("../../../../tests/fixtures/generic/bluetoothctl_device_random.json"),
        );
    }

    #[test]
    fn test_bluetoothctl_empty() {
        let parser = BluetoothctlParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert!(arr.is_empty());
        } else {
            panic!("Expected Array");
        }
    }
}
