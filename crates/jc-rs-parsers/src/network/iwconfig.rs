//! Parser for `iwconfig` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};

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
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let re_interface = Regex::new(r#"^(?P<name>[a-zA-Z0-9:._\-]+)\s+(?P<protocol>(?:[a-zA-Z0-9]+\s)*[a-zA-Z0-9.]+)\s+ESSID:"(?P<essid>[^"]+)""#)
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_mode =
            Regex::new(r"Mode:(?P<mode>\w+)").map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_frequency = Regex::new(r"Frequency:(?P<frequency>[0-9.]+)\s(?P<frequency_unit>\w+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_access_point = Regex::new(r"Access Point:\s*(?P<access_point>[0-9A-Fa-f:]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_bit_rate = Regex::new(r"Bit Rate=(?P<bit_rate>[0-9.]+)\s(?P<bit_rate_unit>[\w/]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_tx_power = Regex::new(r"Tx-Power=(?P<tx_power>[-0-9]+)\s(?P<tx_power_unit>\w+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_retry = Regex::new(r"Retry short limit:(?P<retry_short_limit>[0-9/]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_rts = Regex::new(r"RTS thr:(?P<rts_threshold>off|on)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_frag = Regex::new(r"Fragment thr:(?P<fragment_threshold>off|on)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_power = Regex::new(r"Power Management:(?P<power_management>off|on)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_link = Regex::new(r"Link Quality=(?P<link_quality>[0-9/]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_signal =
            Regex::new(r"Signal level=(?P<signal_level>[-0-9]+)\s(?P<signal_level_unit>\w+)")
                .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_rx_nwid = Regex::new(r"Rx invalid nwid:(?P<rx_invalid_nwid>[-0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_rx_crypt = Regex::new(r"Rx invalid crypt:(?P<rx_invalid_crypt>[-0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_rx_frag = Regex::new(r"Rx invalid frag:(?P<rx_invalid_frag>[-0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_tx_retries = Regex::new(r"Tx excessive retries:(?P<tx_excessive_retries>[-0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_invalid = Regex::new(r"Invalid misc:(?P<invalid_misc>[0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;
        let re_missed = Regex::new(r"Missed beacon:(?P<missed_beacon>[0-9]+)")
            .map_err(|e| ParseError::Regex(e.to_string()))?;

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
                    let freq_str = caps.name("frequency").map_or("", |m| m.as_str());
                    let freq_val = freq_str
                        .parse::<f64>()
                        .ok()
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::String(freq_str.to_string()));
                    obj.insert("frequency".to_string(), freq_val);
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
                    let br_str = caps.name("bit_rate").map_or("", |m| m.as_str());
                    let br_val = br_str
                        .parse::<f64>()
                        .ok()
                        .and_then(|f| serde_json::Number::from_f64(f))
                        .map(Value::Number)
                        .unwrap_or(Value::String(br_str.to_string()));
                    obj.insert("bit_rate".to_string(), br_val);
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
                    let tp_str = caps.name("tx_power").map_or("", |m| m.as_str());
                    let tp_val = tp_str
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::String(tp_str.to_string()));
                    obj.insert("tx_power".to_string(), tp_val);
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
                    let r_str = caps.name("retry_short_limit").map_or("", |m| m.as_str());
                    let r_val = r_str
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::String(r_str.to_string()));
                    obj.insert("retry_short_limit".to_string(), r_val);
                }
                if let Some(caps) = re_rts.captures(line) {
                    let v = caps.name("rts_threshold").map_or("off", |m| m.as_str());
                    obj.insert("rts_threshold".to_string(), Value::Bool(v == "on"));
                }
                if let Some(caps) = re_frag.captures(line) {
                    let v = caps
                        .name("fragment_threshold")
                        .map_or("off", |m| m.as_str());
                    obj.insert("fragment_threshold".to_string(), Value::Bool(v == "on"));
                }
                if let Some(caps) = re_power.captures(line) {
                    let v = caps.name("power_management").map_or("off", |m| m.as_str());
                    obj.insert("power_management".to_string(), Value::Bool(v == "on"));
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
                    let sl_str = caps.name("signal_level").map_or("", |m| m.as_str());
                    let sl_val = sl_str
                        .parse::<i64>()
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::String(sl_str.to_string()));
                    obj.insert("signal_level".to_string(), sl_val);
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
                            let v = s
                                .parse::<i64>()
                                .map(|n| Value::Number(n.into()))
                                .unwrap_or(Value::String(s.to_string()));
                            obj.insert($field.to_string(), v);
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

        Ok(ParseOutput::Array(result))
    }
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
