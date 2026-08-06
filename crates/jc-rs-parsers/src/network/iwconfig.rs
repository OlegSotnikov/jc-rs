//! Parser for `iwconfig` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

/// Every pattern `iwconfig` parsing needs, compiled once for the process
/// rather than rebuilt on each invocation.
struct IwconfigRegexes {
    interface: Regex,
    mode: Regex,
    frequency: Regex,
    access_point: Regex,
    bit_rate: Regex,
    tx_power: Regex,
    retry: Regex,
    rts: Regex,
    frag: Regex,
    power: Regex,
    link: Regex,
    signal: Regex,
    rx_nwid: Regex,
    rx_crypt: Regex,
    rx_frag: Regex,
    tx_retries: Regex,
    invalid: Regex,
    missed: Regex,
}

static RE: LazyLock<IwconfigRegexes> = LazyLock::new(|| {
    IwconfigRegexes {
    interface: Regex::new(r#"^(?P<name>[a-zA-Z0-9:._\-]+)\s+(?P<protocol>(?:[a-zA-Z0-9]+\s)*[a-zA-Z0-9.]+)\s+ESSID:"(?P<essid>[^"]+)""#).expect("interface pattern is valid"),
    mode: Regex::new(r"Mode:(?P<mode>\w+)").expect("mode pattern is valid"),
    frequency: Regex::new(r"Frequency:(?P<frequency>[0-9.]+)\s(?P<frequency_unit>\w+)").expect("frequency pattern is valid"),
    access_point: Regex::new(r"Access Point:\s*(?P<access_point>[0-9A-Fa-f:]+)").expect("access_point pattern is valid"),
    bit_rate: Regex::new(r"Bit Rate=(?P<bit_rate>[0-9.]+)\s(?P<bit_rate_unit>[\w/]+)").expect("bit_rate pattern is valid"),
    tx_power: Regex::new(r"Tx-Power=(?P<tx_power>[-0-9]+)\s(?P<tx_power_unit>\w+)").expect("tx_power pattern is valid"),
    retry: Regex::new(r"Retry short limit:(?P<retry_short_limit>[0-9/]+)").expect("retry pattern is valid"),
    rts: Regex::new(r"RTS thr:(?P<rts_threshold>off|on)").expect("rts pattern is valid"),
    frag: Regex::new(r"Fragment thr:(?P<fragment_threshold>off|on)").expect("frag pattern is valid"),
    power: Regex::new(r"Power Management:(?P<power_management>off|on)").expect("power pattern is valid"),
    link: Regex::new(r"Link Quality=(?P<link_quality>[0-9/]+)").expect("link pattern is valid"),
    signal: Regex::new(r"Signal level=(?P<signal_level>[-0-9]+)\s(?P<signal_level_unit>\w+)").expect("signal pattern is valid"),
    rx_nwid: Regex::new(r"Rx invalid nwid:(?P<rx_invalid_nwid>[-0-9]+)").expect("rx_nwid pattern is valid"),
    rx_crypt: Regex::new(r"Rx invalid crypt:(?P<rx_invalid_crypt>[-0-9]+)").expect("rx_crypt pattern is valid"),
    rx_frag: Regex::new(r"Rx invalid frag:(?P<rx_invalid_frag>[-0-9]+)").expect("rx_frag pattern is valid"),
    tx_retries: Regex::new(r"Tx excessive retries:(?P<tx_excessive_retries>[-0-9]+)").expect("tx_retries pattern is valid"),
    invalid: Regex::new(r"Invalid misc:(?P<invalid_misc>[0-9]+)").expect("invalid pattern is valid"),
    missed: Regex::new(r"Missed beacon:(?P<missed_beacon>[0-9]+)").expect("missed pattern is valid"),
}
});

pub struct IwconfigParser;

static INFO: ParserInfo = ParserInfo {
    name: "iwconfig",
    argument: "--iwconfig",
    version: "1.2.0",
    description: "Converts `iwconfig` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["iwconfig"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IWCONFIG_PARSER: IwconfigParser = IwconfigParser;
inventory::submit! { ParserEntry::new(&IWCONFIG_PARSER) }

impl Parser for IwconfigParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let mut rows = scan(input)?;
        for row in &mut rows {
            process_row(row);
        }
        Ok(ParseOutput::Array(rows))
    }

    /// jc's raw form leaves every value as the string `iwconfig` printed;
    /// `_process` is what turns them into numbers and booleans.
    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Array(scan(input)?))
    }
}

/// jc's int/float/bool lists, applied after scanning.
fn process_row(row: &mut Map<String, Value>) {
    const INT_FIELDS: &[&str] = &[
        "signal_level",
        "rx_invalid_nwid",
        "rx_invalid_crypt",
        "rx_invalid_frag",
        "tx_excessive_retries",
        "invalid_misc",
        "missed_beacon",
        "tx_power",
        "retry_short_limit",
    ];
    const FLOAT_FIELDS: &[&str] = &["frequency", "bit_rate"];
    const BOOL_FIELDS: &[&str] = &["rts_threshold", "fragment_threshold", "power_management"];

    for (key, value) in row.iter_mut() {
        let Value::String(text) = value else {
            continue;
        };
        if INT_FIELDS.contains(&key.as_str()) {
            if let Ok(n) = text.parse::<i64>() {
                *value = Value::Number(n.into());
            }
        } else if FLOAT_FIELDS.contains(&key.as_str()) {
            if let Some(n) = text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
            {
                *value = Value::Number(n);
            }
        } else if BOOL_FIELDS.contains(&key.as_str()) {
            *value = Value::Bool(text == "on");
        }
    }
}

fn scan(input: &str) -> Result<Vec<Map<String, Value>>, ParseError> {
    if input.trim().is_empty() {
        return Ok(Vec::new());
    }

    let re_interface = &RE.interface;
    let re_mode = &RE.mode;
    let re_frequency = &RE.frequency;
    let re_access_point = &RE.access_point;
    let re_bit_rate = &RE.bit_rate;
    let re_tx_power = &RE.tx_power;
    let re_retry = &RE.retry;
    let re_rts = &RE.rts;
    let re_frag = &RE.frag;
    let re_power = &RE.power;
    let re_link = &RE.link;
    let re_signal = &RE.signal;
    let re_rx_nwid = &RE.rx_nwid;
    let re_rx_crypt = &RE.rx_crypt;
    let re_rx_frag = &RE.rx_frag;
    let re_tx_retries = &RE.tx_retries;
    let re_invalid = &RE.invalid;
    let re_missed = &RE.missed;

    let mut result: Vec<Map<String, Value>> = Vec::new();
    let mut current: Option<Map<String, Value>> = None;

    for line in input.lines() {
        // Check for new interface line
        if let Some(caps) = re_interface.captures(line) {
            if let Some(iface) = current.take() {
                result.push(iface);
            }
            let mut obj = Map::new();
            obj.insert(
                "name".to_string(),
                Value::String(caps.name("name").map_or("", |m| m.as_str()).to_string()),
            );
            obj.insert(
                "protocol".to_string(),
                Value::String(
                    caps.name("protocol")
                        .map_or("", |m| m.as_str())
                        .trim()
                        .to_string(),
                ),
            );
            obj.insert(
                "essid".to_string(),
                Value::String(caps.name("essid").map_or("", |m| m.as_str()).to_string()),
            );
            current = Some(obj);
            // Also check the same line for other fields
        }

        if let Some(ref mut obj) = current {
            if let Some(caps) = re_mode.captures(line) {
                obj.insert(
                    "mode".to_string(),
                    Value::String(caps.name("mode").map_or("", |m| m.as_str()).to_string()),
                );
            }
            if let Some(caps) = re_frequency.captures(line) {
                obj.insert(
                    "frequency".to_string(),
                    Value::String(
                        caps.name("frequency")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
                obj.insert(
                    "frequency_unit".to_string(),
                    Value::String(
                        caps.name("frequency_unit")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_access_point.captures(line) {
                obj.insert(
                    "access_point".to_string(),
                    Value::String(
                        caps.name("access_point")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_bit_rate.captures(line) {
                obj.insert(
                    "bit_rate".to_string(),
                    Value::String(caps.name("bit_rate").map_or("", |m| m.as_str()).to_string()),
                );
                obj.insert(
                    "bit_rate_unit".to_string(),
                    Value::String(
                        caps.name("bit_rate_unit")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_tx_power.captures(line) {
                obj.insert(
                    "tx_power".to_string(),
                    Value::String(caps.name("tx_power").map_or("", |m| m.as_str()).to_string()),
                );
                obj.insert(
                    "tx_power_unit".to_string(),
                    Value::String(
                        caps.name("tx_power_unit")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_retry.captures(line) {
                obj.insert(
                    "retry_short_limit".to_string(),
                    Value::String(
                        caps.name("retry_short_limit")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_rts.captures(line) {
                let v = caps.name("rts_threshold").map_or("off", |m| m.as_str());
                obj.insert("rts_threshold".to_string(), Value::String(v.to_string()));
            }
            if let Some(caps) = re_frag.captures(line) {
                let v = caps
                    .name("fragment_threshold")
                    .map_or("off", |m| m.as_str());
                obj.insert(
                    "fragment_threshold".to_string(),
                    Value::String(v.to_string()),
                );
            }
            if let Some(caps) = re_power.captures(line) {
                let v = caps.name("power_management").map_or("off", |m| m.as_str());
                obj.insert("power_management".to_string(), Value::String(v.to_string()));
            }
            if let Some(caps) = re_link.captures(line) {
                obj.insert(
                    "link_quality".to_string(),
                    Value::String(
                        caps.name("link_quality")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            if let Some(caps) = re_signal.captures(line) {
                obj.insert(
                    "signal_level".to_string(),
                    Value::String(
                        caps.name("signal_level")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
                obj.insert(
                    "signal_level_unit".to_string(),
                    Value::String(
                        caps.name("signal_level_unit")
                            .map_or("", |m| m.as_str())
                            .to_string(),
                    ),
                );
            }
            macro_rules! parse_int_field {
                ($re:expr, $name:expr, $field:expr) => {
                    if let Some(caps) = $re.captures(line) {
                        let s = caps.name($name).map_or("", |m| m.as_str());
                        obj.insert($field.to_string(), Value::String(s.to_string()));
                    }
                };
            }
            parse_int_field!(re_rx_nwid, "rx_invalid_nwid", "rx_invalid_nwid");
            parse_int_field!(re_rx_crypt, "rx_invalid_crypt", "rx_invalid_crypt");
            parse_int_field!(re_rx_frag, "rx_invalid_frag", "rx_invalid_frag");
            parse_int_field!(
                re_tx_retries,
                "tx_excessive_retries",
                "tx_excessive_retries"
            );
            parse_int_field!(re_invalid, "invalid_misc", "invalid_misc");
            parse_int_field!(re_missed, "missed_beacon", "missed_beacon");
        }
    }

    if let Some(iface) = current {
        result.push(iface);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_iwconfig_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/iwconfig.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/iwconfig.json"
        ))
        .unwrap();
        let result = IwconfigParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iwconfig_many_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/iwconfig-many.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/iwconfig-many.json"
        ))
        .unwrap();
        let result = IwconfigParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iwconfig_empty() {
        let result = IwconfigParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_iwconfig_registered() {
        assert!(jc_rs_core::registry::find_parser("iwconfig").is_some());
    }
}
