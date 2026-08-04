//! Parser for `amixer sget` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::convert_to_int;
use serde_json::{Map, Value};

pub struct AmixerParser;

static INFO: ParserInfo = ParserInfo {
    name: "amixer",
    argument: "--amixer",
    version: "1.0.0",
    description: "`amixer` command parser",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["amixer"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static AMIXER_PARSER: AmixerParser = AmixerParser;

inventory::submit! {
    ParserEntry::new(&AMIXER_PARSER)
}

/// Parse channel info like "Playback 87 [100%] [0.00dB] [on]"
/// Returns (playback_value, percentage_int, db_float, status_bool)
fn parse_channel_info(channel_info: &str) -> Option<(i64, i64, f64, bool)> {
    let parts: Vec<&str> = channel_info.split_whitespace().collect();
    // parts[0] = "Playback" or "Capture"
    // parts[1] = value
    // parts[2] = "[percentage%]"
    // parts[3] = "[db]"
    // parts[4] = "[on/off]"
    if parts.len() < 4 {
        return None;
    }

    let playback_value = convert_to_int(parts[1])?;
    let percentage_str = parts[2].trim_matches(|c| c == '[' || c == ']');
    let percentage_str = percentage_str.trim_end_matches('%');
    let percentage = convert_to_int(percentage_str)?;

    let (db_val, status_val) = if parts.len() >= 5 && parts[3].to_lowercase().contains("db") {
        let db_str = parts[3]
            .trim_matches(|c| c == '[' || c == ']')
            .to_lowercase()
            .replace("db", "");
        let db: f64 = db_str.parse().unwrap_or(0.0);
        let status = parts[4].trim_matches(|c| c == '[' || c == ']') == "on";
        (db, status)
    } else if parts.len() >= 4 {
        let status = parts[3].trim_matches(|c| c == '[' || c == ']') == "on";
        (0.0, status)
    } else {
        (0.0, false)
    };

    Some((playback_value, percentage, db_val, status_val))
}

impl Parser for AmixerParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut obj = Map::new();
        let mut playback_channels: Option<Vec<Value>> = None;

        let lines: Vec<&str> = input.lines().collect();
        if lines.is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // First line: Simple mixer control 'Master',0
        let first_line = lines[0].trim();
        if first_line.starts_with("Simple mixer control") {
            let parts: Vec<&str> = first_line.splitn(3, '\'').collect();
            if parts.len() >= 2 {
                obj.insert(
                    "control_name".to_string(),
                    Value::String(parts[1].to_string()),
                );
            }
        }

        for line in &lines[1..] {
            let line = line.trim();

            if line.starts_with("Capabilities:") {
                let caps_str = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
                let caps: Vec<Value> = caps_str
                    .split_whitespace()
                    .map(|s| Value::String(s.to_string()))
                    .collect();
                obj.insert("capabilities".to_string(), Value::Array(caps));
            } else if line.starts_with("Playback channels:") {
                let chans_str = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
                let chans: Vec<Value> = chans_str
                    .split(" - ")
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                playback_channels = Some(chans);
            } else if line.starts_with("Limits:") {
                // Limits: Playback 0 - 87  OR  Limits: Capture 0 - 63
                let limits_str = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
                let parts: Vec<&str> = limits_str.splitn(3, " - ").collect();
                if parts.len() >= 2 {
                    // parts[0] = "Playback 0" or "Capture 0"
                    let min_parts: Vec<&str> = parts[0].split_whitespace().collect();
                    let min_val = min_parts.last().copied().unwrap_or("0");
                    let max_val = parts[1].trim();
                    let mut limits = Map::new();
                    limits.insert(
                        "playback_min".to_string(),
                        convert_to_int(min_val)
                            .map(Value::from)
                            .unwrap_or(Value::Null),
                    );
                    limits.insert(
                        "playback_max".to_string(),
                        convert_to_int(max_val)
                            .map(Value::from)
                            .unwrap_or(Value::Null),
                    );
                    obj.insert("limits".to_string(), Value::Object(limits));
                }
            } else if line.starts_with("Mono:")
                || line.starts_with("Front Left:")
                || line.starts_with("Front Right:")
            {
                let colon_pos = line.find(':').unwrap_or(line.len());
                let channel_name = line[..colon_pos].trim().to_lowercase().replace(' ', "_");
                let channel_info = line[colon_pos + 1..].trim();

                // Skip empty channel lines (e.g., "Mono:")
                if channel_info.is_empty() {
                    continue;
                }

                if let Some((pv, pct, db, status)) = parse_channel_info(channel_info) {
                    let mut ch_obj = Map::new();
                    ch_obj.insert("playback_value".to_string(), Value::from(pv));
                    ch_obj.insert("percentage".to_string(), Value::from(pct));
                    ch_obj.insert("db".to_string(), Value::from(db));
                    ch_obj.insert("status".to_string(), Value::Bool(status));
                    obj.insert(channel_name, Value::Object(ch_obj));
                }
            }
        }

        // Always include playback_channels (default to empty array)
        obj.insert(
            "playback_channels".to_string(),
            Value::Array(playback_channels.unwrap_or_default()),
        );

        Ok(ParseOutput::Object(obj))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture(input: &str, expected_json: &str) {
        let parser = AmixerParser;
        let result = parser.parse(input, false).unwrap();
        let expected: serde_json::Value = serde_json::from_str(expected_json).unwrap();

        if let ParseOutput::Object(obj) = result {
            let got = serde_json::Value::Object(obj);
            assert_eq!(got, expected);
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_amixer_master() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-22.04/amixer-control-master.out"),
            include_str!(
                "../../../../tests/fixtures/ubuntu-22.04/amixer-control-master-processed.json"
            ),
        );
    }

    #[test]
    fn test_amixer_capture() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-22.04/amixer-control-capture.out"),
            include_str!(
                "../../../../tests/fixtures/ubuntu-22.04/amixer-control-capture-processed.json"
            ),
        );
    }

    #[test]
    fn test_amixer_headphone() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-22.04/amixer-control-headphone.out"),
            include_str!(
                "../../../../tests/fixtures/ubuntu-22.04/amixer-control-headphone-processed.json"
            ),
        );
    }

    #[test]
    fn test_amixer_speakers() {
        parse_fixture(
            include_str!("../../../../tests/fixtures/ubuntu-22.04/amixer-control-speakers.out"),
            include_str!(
                "../../../../tests/fixtures/ubuntu-22.04/amixer-control-speakers-processed.json"
            ),
        );
    }

    #[test]
    fn test_amixer_empty() {
        let parser = AmixerParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
