//! Parser for `timedatectl` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use serde_json::{Map, Value};

pub struct TimedatectlParser;

static INFO: ParserInfo = ParserInfo {
    name: "timedatectl",
    argument: "--timedatectl",
    version: "1.8.0",
    description: "Converts `timedatectl status` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["timedatectl", "timedatectl status"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TIMEDATECTL_PARSER: TimedatectlParser = TimedatectlParser;

inventory::submit! {
    ParserEntry::new(&TIMEDATECTL_PARSER)
}

impl Parser for TimedatectlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let out = parse_timedatectl(input);
        Ok(ParseOutput::Object(out))
    }
}

const VALID_FIELDS: &[&str] = &[
    "local time",
    "universal time",
    "rtc time",
    "time zone",
    "ntp enabled",
    "ntp synchronized",
    "rtc in local tz",
    "dst active",
    "system clock synchronized",
    "ntp service",
    "systemd-timesyncd.service active",
    "server",
    "poll interval",
    "leap",
    "version",
    "stratum",
    "reference",
    "precision",
    "root distance",
    "offset",
    "delay",
    "jitter",
    "packet count",
    "frequency",
];

fn parse_timedatectl(input: &str) -> Map<String, Value> {
    let mut out = Map::new();

    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_lowercase();
            let val = line[colon_pos + 1..].trim().to_string();

            if VALID_FIELDS.contains(&key.as_str()) {
                let keyname = key.replace(' ', "_");

                // Handle unit extraction for offset/delay/jitter/frequency
                match key.as_str() {
                    "offset" | "delay" | "jitter" => {
                        // Extract numeric value and unit (last 2 chars)
                        let unit = if val.len() >= 2 {
                            val[val.len() - 2..].to_string()
                        } else {
                            String::new()
                        };
                        // Try to parse float from first part
                        let num_str: String = val
                            .chars()
                            .take_while(|c| {
                                c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+'
                            })
                            .collect();
                        if let Ok(f) = num_str.parse::<f64>() {
                            out.insert(
                                keyname.clone(),
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::String(val.clone())),
                            );
                            out.insert(keyname + "_unit", Value::String(unit));
                        } else {
                            out.insert(keyname, Value::String(val));
                        }
                    }
                    "frequency" => {
                        let unit = if val.len() >= 3 {
                            val[val.len() - 3..].to_string()
                        } else {
                            String::new()
                        };
                        let num_str: String = val
                            .chars()
                            .take_while(|c| {
                                c.is_ascii_digit() || *c == '.' || *c == '-' || *c == '+'
                            })
                            .collect();
                        if let Ok(f) = num_str.parse::<f64>() {
                            out.insert(
                                keyname.clone(),
                                serde_json::Number::from_f64(f)
                                    .map(Value::Number)
                                    .unwrap_or(Value::String(val.clone())),
                            );
                            out.insert(keyname + "_unit", Value::String(unit));
                        } else {
                            out.insert(keyname, Value::String(val));
                        }
                    }
                    "ntp enabled"
                    | "ntp synchronized"
                    | "rtc in local tz"
                    | "dst active"
                    | "system clock synchronized"
                    | "systemd-timesyncd.service active" => {
                        let b = matches!(val.to_lowercase().as_str(), "yes" | "true" | "1");
                        out.insert(keyname, Value::Bool(b));
                    }
                    "version" | "stratum" | "packet count" => {
                        if let Ok(n) = val.parse::<i64>() {
                            out.insert(keyname, Value::Number(n.into()));
                        } else {
                            out.insert(keyname, Value::String(val));
                        }
                    }
                    _ => {
                        out.insert(keyname, Value::String(val));
                    }
                }
            }
        }
    }

    // Compute epoch_utc from universal_time (e.g., "Wed 2020-03-11 00:53:21 UTC")
    if let Some(Value::String(ut)) = out.get("universal_time") {
        // Strip day-of-week prefix (e.g., "Wed ")
        let ts_str = if let Some(pos) = ut.find(char::is_numeric) {
            &ut[pos..]
        } else {
            ut.as_str()
        };
        // The field is UTC by definition, so the stamp must be read as UTC.
        // `naive_epoch` reads it as local, which put epoch_utc out by the
        // machine's offset.
        let parsed = parse_timestamp(ts_str, &[formats::F7300]);
        if let Some(e) = parsed.utc_epoch.or(parsed.naive_epoch) {
            out.insert("epoch_utc".to_string(), Value::Number(e.into()));
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timedatectl_basic() {
        let input = "               Local time: Tue 2020-03-10 17:53:21 PDT\n\
                     Universal time: Wed 2020-03-11 00:53:21 UTC\n\
                           RTC time: Wed 2020-03-11 00:53:21\n\
                          Time zone: America/Los_Angeles (PDT, -0700)\n\
                        NTP enabled: yes\n\
                   NTP synchronized: yes\n\
                    RTC in local TZ: no\n\
                         DST active: yes";

        let parser = TimedatectlParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert_eq!(
                obj.get("local_time"),
                Some(&Value::String("Tue 2020-03-10 17:53:21 PDT".to_string()))
            );
            assert_eq!(obj.get("ntp_enabled"), Some(&Value::Bool(true)));
            assert_eq!(obj.get("rtc_in_local_tz"), Some(&Value::Bool(false)));
            assert_eq!(obj.get("dst_active"), Some(&Value::Bool(true)));
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_timedatectl_empty() {
        let parser = TimedatectlParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
