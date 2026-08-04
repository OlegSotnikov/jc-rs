//! Syslog RFC 5424 string parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct SyslogParser;

static INFO: ParserInfo = ParserInfo {
    name: "syslog",
    argument: "--syslog",
    version: "1.0.0",
    description: "Syslog RFC 5424 string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String, Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static SYSLOG_RE: OnceLock<Regex> = OnceLock::new();
static SD_KV_RE: OnceLock<Regex> = OnceLock::new();

fn get_syslog_re() -> &'static Regex {
    SYSLOG_RE.get_or_init(|| {
        Regex::new(
            r"^(?P<priority><(?:\d|\d{2}|1[1-8]\d|19[01])>)?(?P<version>\d{1,2})?\s*(?P<timestamp>-|(?:[12]\d{3}-(?:0\d|1[012])-(?:[012]\d|3[01])T(?:[01]\d|2[0-4]):(?:[0-5]\d):(?:[0-5]\d|60)(?:\.\d{1,6})?(?:Z|[+-]\d{2}:\d{2})))\s(?P<hostname>\S{1,255})\s(?P<appname>\S{1,48})\s(?P<procid>\S{1,128})\s(?P<msgid>\S{1,32})\s(?P<structureddata>-|(?:\[(?:[^\]\\]|\\.)*\])+)(?:\s(?P<msg>.+))?$"
        ).expect("syslog regex")
    })
}

fn get_sd_kv_re() -> &'static Regex {
    SD_KV_RE.get_or_init(|| Regex::new(r#"(\w+)="([^"]*)""#).expect("sd kv regex"))
}

fn unescape_syslog(s: &str) -> String {
    // Process escape sequences per RFC 5424: \\ -> \, \" -> ", \] -> ]
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some(&'\\') => {
                    chars.next();
                    result.push('\\');
                }
                Some(&'"') => {
                    chars.next();
                    result.push('"');
                }
                Some(&']') => {
                    chars.next();
                    result.push(']');
                }
                _ => {
                    result.push(ch);
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn extract_sd_blocks(s: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    i += 2; // skip escaped char
                } else if bytes[i] == b']' {
                    i += 1;
                    blocks.push(s[start..i].to_string());
                    break;
                } else {
                    i += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    blocks
}

fn parse_structured_data(sd_str: &str) -> Value {
    let blocks = extract_sd_blocks(sd_str);
    let mut result = Vec::new();

    for block in &blocks {
        // Strip outer brackets
        let inner = if block.len() >= 2 {
            &block[1..block.len() - 1]
        } else {
            continue;
        };

        // Extract identity (token before first space)
        let (identity, rest) = match inner.find(' ') {
            Some(pos) => (Some(inner[..pos].to_string()), &inner[pos + 1..]),
            None => (Some(inner.to_string()), ""),
        };

        // Extract kv pairs
        let mut parameters = Map::new();
        for caps in get_sd_kv_re().captures_iter(rest) {
            let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let val = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let unescaped = unescape_syslog(val);
            parameters.insert(key.to_string(), Value::String(unescaped));
        }

        let mut struct_obj = Map::new();
        struct_obj.insert(
            "identity".to_string(),
            match identity {
                Some(id) => Value::String(id),
                None => Value::Null,
            },
        );
        struct_obj.insert("parameters".to_string(), Value::Object(parameters));
        result.push(Value::Object(struct_obj));
    }

    if result.is_empty() {
        Value::Null
    } else {
        Value::Array(result)
    }
}

pub fn parse_syslog_line(line: &str) -> Map<String, Value> {
    let mut map = Map::new();

    let re = get_syslog_re();
    if let Some(caps) = re.captures(line) {
        let null_if_dash = |s: &str| -> Value {
            if s == "-" || s.is_empty() {
                Value::Null
            } else {
                Value::String(s.to_string())
            }
        };

        // priority: strip < > and convert to int
        let priority = caps.name("priority").and_then(|m| {
            let s = m.as_str();
            s[1..s.len() - 1].parse::<i64>().ok()
        });
        map.insert(
            "priority".to_string(),
            match priority {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            },
        );

        // version: convert to int
        let version = caps
            .name("version")
            .and_then(|m| m.as_str().parse::<i64>().ok());
        map.insert(
            "version".to_string(),
            match version {
                Some(n) => Value::Number(n.into()),
                None => Value::Null,
            },
        );

        // timestamp
        let ts_str = caps.name("timestamp").map(|m| m.as_str()).unwrap_or("-");
        let ts_val = null_if_dash(ts_str);
        map.insert("timestamp".to_string(), ts_val);

        // hostname, appname
        map.insert(
            "hostname".to_string(),
            null_if_dash(caps.name("hostname").map(|m| m.as_str()).unwrap_or("-")),
        );
        map.insert(
            "appname".to_string(),
            null_if_dash(caps.name("appname").map(|m| m.as_str()).unwrap_or("-")),
        );

        // proc_id: convert to int if possible
        let procid_str = caps.name("procid").map(|m| m.as_str()).unwrap_or("-");
        let proc_id = if procid_str == "-" {
            Value::Null
        } else {
            match procid_str.parse::<i64>() {
                Ok(n) => Value::Number(n.into()),
                Err(_) => Value::Null,
            }
        };
        map.insert("proc_id".to_string(), proc_id);

        // msg_id
        map.insert(
            "msg_id".to_string(),
            null_if_dash(caps.name("msgid").map(|m| m.as_str()).unwrap_or("-")),
        );

        // structured_data
        let sd_raw = caps
            .name("structureddata")
            .map(|m| m.as_str())
            .unwrap_or("-");
        let structured_data = if sd_raw == "-" {
            Value::Null
        } else {
            parse_structured_data(sd_raw)
        };
        map.insert("structured_data".to_string(), structured_data);

        // message - unescape
        let msg_raw = caps.name("msg").map(|m| m.as_str()).unwrap_or("").trim();
        let msg_val = if msg_raw.is_empty() {
            Value::Null
        } else {
            Value::String(unescape_syslog(msg_raw))
        };
        map.insert("message".to_string(), msg_val);

        // Timestamp epoch fields
        if let Some(Value::String(ts)) = map.get("timestamp") {
            let parsed = parse_timestamp(ts, &[formats::F1300, formats::F1310]);
            map.insert(
                "timestamp_epoch".to_string(),
                match parsed.naive_epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
            map.insert(
                "timestamp_epoch_utc".to_string(),
                match parsed.utc_epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
        }
        // If timestamp is Null, don't add epoch fields (matches jc behavior)
    } else {
        // RFC 5424 didn't match — try RFC 3164 (BSD syslog) format
        let bsd_map = super::syslog_bsd::parse_syslog_bsd_line(line);
        return bsd_map;
    }

    map
}

impl Parser for SyslogParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let records: Vec<Map<String, Value>> = input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_syslog_line)
            .collect();
        Ok(ParseOutput::Array(records))
    }
}

static SYSLOG_PARSER: SyslogParser = SyslogParser;

inventory::submit! {
    ParserEntry::new(&SYSLOG_PARSER)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::registry::find_parser;
    use jc_rs_core::types::ParseOutput;

    fn get_fixture(rel_path: &str) -> String {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let paths = [
            format!("{manifest}/../../tests/fixtures/{rel_path}"),
            format!("{manifest}/../../../tests/fixtures/{rel_path}"),
        ];
        for p in &paths {
            if let Ok(c) = std::fs::read_to_string(p) {
                return c;
            }
        }
        panic!("fixture not found: {rel_path}");
    }

    #[test]
    fn test_syslog_registered() {
        assert!(find_parser("syslog").is_some());
    }

    #[test]
    fn test_syslog_basic() {
        let line =
            "<165>1 2003-08-24T05:14:15.000003-07:00 192.0.2.1 myproc 8710 - - %% It's time.";
        let map = parse_syslog_line(line);
        assert_eq!(map["priority"], serde_json::json!(165));
        assert_eq!(map["version"], serde_json::json!(1));
        assert_eq!(map["hostname"], serde_json::json!("192.0.2.1"));
        assert_eq!(map["appname"], serde_json::json!("myproc"));
        assert_eq!(map["proc_id"], serde_json::json!(8710));
        assert!(map["msg_id"].is_null());
        assert!(map["structured_data"].is_null());
        assert_eq!(map["message"], serde_json::json!("%% It's time."));
        // utc_epoch should be null (non-UTC timezone)
        assert!(map["timestamp_epoch_utc"].is_null());
    }

    #[test]
    fn test_syslog_utc_timestamp() {
        let line = "<34>1 2003-10-11T22:14:15.003Z mymachine.example.com su - ID47 - msg";
        let map = parse_syslog_line(line);
        assert_eq!(map["priority"], serde_json::json!(34));
        assert_eq!(map["msg_id"], serde_json::json!("ID47"));
        // UTC timestamp should have utc_epoch set
        assert!(!map["timestamp_epoch_utc"].is_null());
        assert_eq!(map["timestamp_epoch_utc"], serde_json::json!(1065910455i64));
    }

    #[test]
    fn test_syslog_structured_data() {
        let line = r#"<190>1 2003-10-11T22:14:15.003Z mymachine.example.com evntslog - ID47 [exampleSDID@32473 iut="3" eventSource="Application" eventID="1011"][examplePriority@32473 class="high"]"#;
        let map = parse_syslog_line(line);
        let sd = map["structured_data"].as_array().unwrap();
        assert_eq!(sd.len(), 2);
        assert_eq!(sd[0]["identity"], serde_json::json!("exampleSDID@32473"));
        assert_eq!(sd[0]["parameters"]["iut"], serde_json::json!("3"));
        assert_eq!(
            sd[1]["identity"],
            serde_json::json!("examplePriority@32473")
        );
    }

    #[test]
    fn test_syslog_unparsable() {
        let line = "<190)1 2003-10-11T22:14:15.003Z mymachine evntslog - ID47 [-] msg";
        let map = parse_syslog_line(line);
        assert!(map.contains_key("unparsable"));
    }

    #[test]
    fn test_syslog_fixture() {
        let input = get_fixture("generic/syslog-5424.out");
        let parser = find_parser("syslog").unwrap();
        let result = parser.parse(&input, true).unwrap();
        let arr = match result {
            ParseOutput::Array(v) => v,
            _ => panic!("expected array"),
        };
        assert!(!arr.is_empty());
        // First record
        assert!(arr[0].contains_key("priority"));
        assert!(arr[0].contains_key("hostname"));
        // Some records should be unparsable
        let has_unparsable = arr.iter().any(|r| r.contains_key("unparsable"));
        assert!(has_unparsable);
    }
}
