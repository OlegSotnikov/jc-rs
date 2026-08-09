//! Parser for `iw dev <device> scan` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct IwScanParser;

static INFO: ParserInfo = ParserInfo {
    name: "iw_scan",
    argument: "--iw-scan",
    version: "1.0.0",
    description: "Converts `iw dev <device> scan` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static IW_SCAN_PARSER: IwScanParser = IwScanParser;
inventory::submit! { ParserEntry::new(&IW_SCAN_PARSER) }

/// jc's key normalisation for this parser: lowercase, drop `*`, `(`, `)` and
/// `,`, turn `-` and spaces into underscores.
fn normalize_key(key: &str) -> String {
    key.to_lowercase()
        .replace(['*', '(', ')', ','], "")
        .replace('-', "_")
        .trim()
        .replace(' ', "_")
}

/// Fields jc renames by moving the unit out of the value and into the key.
/// `None` means the value is not a fixed suffix; see the match below.
const UNIT_FIELDS: &[(&str, &str, Option<&str>)] = &[
    ("tsf", "tsf_usec", None),
    ("sta_channel_width", "sta_channel_width_mhz", Some(" MHz")),
    ("passive_dwell", "passive_dwell_tus", Some(" TUs")),
    ("active_dwell", "active_dwell_tus", Some(" TUs")),
    (
        "channel_width_trigger_scan_interval",
        "channel_width_trigger_scan_interval_s",
        Some(" s"),
    ),
    (
        "scan_passive_total_per_channel",
        "scan_passive_total_per_channel_tus",
        Some(" TUs"),
    ),
    (
        "scan_active_total_per_channel",
        "scan_active_total_per_channel_tus",
        Some(" TUs"),
    ),
    ("beacon_interval", "beacon_interval_tus", Some(" TUs")),
    ("signal", "signal_dbm", Some(" dBm")),
    ("last_seen", "last_seen_ms", Some(" ms ago")),
    (
        "obss_scan_activity_threshold",
        "obss_scan_activity_threshold_percent",
        Some(" %"),
    ),
    ("ds_parameter_set", "ds_parameter_set_channel", None),
    ("max_amsdu_length", "max_amsdu_length_bytes", Some(" bytes")),
    ("power_constraint", "power_constraint_db", Some(" dB")),
    (
        "minimum_rx_ampdu_time_spacing",
        "minimum_rx_ampdu_time_spacing_usec",
        None,
    ),
    (
        "vht_rx_highest_supported",
        "vht_rx_highest_supported_mbps",
        Some(" Mbps"),
    ),
    (
        "vht_tx_highest_supported",
        "vht_tx_highest_supported_mbps",
        Some(" Mbps"),
    ),
];

/// Read the scan into string fields. This is jc's raw form.
fn scan(input: &str) -> Vec<Map<String, Value>> {
    let mut sections: Vec<Map<String, Value>> = Vec::new();
    let mut section: Map<String, Value> = Map::new();
    let mut header = String::new();

    for line in input.lines().filter(|l| !l.is_empty()) {
        if line.starts_with("BSS") {
            if !section.is_empty() {
                sections.push(std::mem::take(&mut section));
            }
            // In `BSS 00:11:22:33:44:55(on wlan0)` the parens become spaces, so
            // the bssid is field 1 and the interface field 3.
            let cleaned = line.replace(['(', ')'], " ");
            let fields: Vec<&str> = cleaned.split_whitespace().collect();
            if let Some(bssid) = fields.get(1) {
                section.insert("bssid".to_string(), Value::String((*bssid).to_string()));
            }
            if let Some(interface) = fields.get(3) {
                section.insert(
                    "interface".to_string(),
                    Value::String((*interface).to_string()),
                );
            }
            header.clear();
            continue;
        }

        // Only indented lines carry fields.
        if !line.starts_with(char::is_whitespace) {
            continue;
        }
        // This field spans two lines; jc skips it rather than mis-key the
        // continuation.
        if line.contains("Maximum RX AMPDU length") {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = normalize_key(key);
        let value = value.trim();

        // A key with no value opens a group whose members carry it as a prefix.
        if value.is_empty() {
            header = format!("{key}_");
        }
        section.insert(format!("{header}{key}"), Value::String(value.to_string()));
    }

    if !section.is_empty() {
        sections.push(section);
    }
    sections
}

/// jc's `_post_parse` plus `_process`: drop empties, unwrap bullets, move units
/// into key names, split the rate lists, then coerce what looks numeric.
fn post_parse(sections: Vec<Map<String, Value>>) -> Vec<Map<String, Value>> {
    sections
        .into_iter()
        .map(|section| {
            let mut item: Map<String, Value> = section
                .into_iter()
                .filter(|(_, v)| !matches!(v, Value::String(s) if s.is_empty()))
                .map(|(k, v)| match v {
                    // `* Parameter version 1` is a bullet, not a value.
                    Value::String(s) if s.starts_with('*') => {
                        (k, Value::String(s[1..].trim().to_string()))
                    }
                    other => (k, other),
                })
                .collect();

            if let Some(Value::String(country)) = item.get("country").cloned() {
                let parts: Vec<&str> = country.split_whitespace().collect();
                if let Some(code) = parts.first() {
                    item.insert("country".to_string(), Value::String((*code).to_string()));
                }
                if let Some(environment) = parts.get(2) {
                    item.insert(
                        "environment".to_string(),
                        Value::String((*environment).to_string()),
                    );
                }
            }

            for (from, to, suffix) in UNIT_FIELDS {
                let Some(Value::String(value)) = item.remove(*from) else {
                    continue;
                };
                let stripped = match suffix {
                    Some(unit) => value.replace(unit, ""),
                    None if *from == "ds_parameter_set" => value.replace("channel ", ""),
                    // The rest keep the number and drop the rest of the phrase.
                    None => value.split_whitespace().next().unwrap_or("").to_string(),
                };
                item.insert((*to).to_string(), Value::String(stripped));
            }

            if let Some(Value::String(rates)) = item.get("supported_rates").cloned() {
                // A rate marked with `*` is one the AP requires.
                let selected: Vec<Value> = rates
                    .split_whitespace()
                    .filter(|r| r.ends_with('*'))
                    .map(|r| Value::String(r.trim_end_matches('*').to_string()))
                    .collect();
                item.insert("selected_rates".to_string(), Value::Array(selected));
                item.insert(
                    "supported_rates".to_string(),
                    Value::Array(
                        rates
                            .replace('*', "")
                            .split_whitespace()
                            .map(|r| Value::String(r.to_string()))
                            .collect(),
                    ),
                );
            }

            if let Some(Value::String(rates)) = item.get("extended_supported_rates").cloned() {
                item.insert(
                    "extended_supported_rates".to_string(),
                    Value::Array(
                        rates
                            .split_whitespace()
                            .map(|r| Value::String(r.to_string()))
                            .collect(),
                    ),
                );
            }

            if let Some(Value::String(capacity)) = item.get("available_admission_capacity").cloned()
            {
                item.insert(
                    "available_admission_capacity".to_string(),
                    Value::String(capacity.replace(" [*32us]", "")),
                );
            }

            item.into_iter().map(|(k, v)| (k, numeric(v))).collect()
        })
        .collect()
}

/// int first, then float, else leave it alone: jc's `_process`, applied to
/// scalars and to list members.
fn numeric(value: Value) -> Value {
    match value {
        Value::String(s) => {
            if let Ok(n) = s.parse::<i64>() {
                return Value::Number(n.into());
            }
            match s.parse::<f64>().ok().and_then(serde_json::Number::from_f64) {
                Some(n) => Value::Number(n),
                None => Value::String(s),
            }
        }
        Value::Array(items) => Value::Array(items.into_iter().map(numeric).collect()),
        other => other,
    }
}

impl Parser for IwScanParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Array(post_parse(scan(input))))
    }

    /// jc's raw form is the scan before any of the renaming or coercion.
    fn parse_raw(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        Ok(ParseOutput::Array(scan(input)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_iw_scan_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iw-scan0.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/iw-scan0.json"
        ))
        .unwrap();
        let result = IwScanParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_iw_scan_empty() {
        let result = IwScanParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_iw_scan_registered() {
        assert!(jc_rs_core::registry::find_parser("iw_scan").is_some());
    }

    #[test]
    fn test_iw_scan_raw_keeps_the_unit_in_the_value() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/iw-scan0.out");
        let ParseOutput::Array(rows) = IwScanParser.parse_raw(input, false).unwrap() else {
            panic!("expected an array");
        };
        assert_eq!(rows[0]["signal"], "-45.00 dBm");
        assert!(!rows[0].contains_key("signal_dbm"));
    }
}
