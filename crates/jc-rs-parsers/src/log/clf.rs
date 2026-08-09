//! Common Log Format (CLF) parser for Apache/Nginx access log files.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use jc_rs_utils::{parse_timestamp, timestamp::formats};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::OnceLock;

struct ClfParser;

static CLF_INFO: ParserInfo = ParserInfo {
    name: "clf",
    argument: "--clf",
    version: "1.0.0",
    description: "Common and Combined Log Format file parser",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::Command, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CLF_RE: OnceLock<Regex> = OnceLock::new();
static REQUEST_RE: OnceLock<Regex> = OnceLock::new();

fn get_clf_re() -> &'static Regex {
    CLF_RE.get_or_init(|| {
        Regex::new(
            r#"(?x)
            ^(?P<host>-|\S+)\s
            (?P<ident>-|\S+)\s
            (?P<authuser>-|\S+)\s
            \[
            (?P<date>
                (?P<day>\d+)/
                (?P<month>\S{3})/
                (?P<year>\d{4}):
                (?P<hour>\d{2}):
                (?P<minute>\d{2}):
                (?P<second>\d{2})\s
                (?P<tz>\S+)
            )
            \]\s
            "(?P<request>.*?)"\s
            (?P<status>-|\d{3})\s
            (?P<bytes>-|\d+)\s?
            (?:"(?P<referer>.*?)"\s?)?
            (?:"(?P<user_agent>.*?)"\s?)?
            (?P<extra>.*)
            "#,
        )
        .expect("clf regex compile error")
    })
}

fn get_request_re() -> &'static Regex {
    REQUEST_RE.get_or_init(|| {
        Regex::new(
            r"^(?P<request_method>\S+)\s(?P<request_url>.*?)(?:\s(?P<request_version>HTTPS?/[\d.]+))?$"
        )
        .expect("request regex compile error")
    })
}

fn null_if_dash(s: &str) -> Value {
    if s == "-" || s.is_empty() {
        Value::Null
    } else {
        Value::String(s.to_string())
    }
}

fn parse_int_or_null(s: &str) -> Value {
    if s == "-" || s.is_empty() {
        Value::Null
    } else if let Ok(n) = s.parse::<i64>() {
        Value::Number(n.into())
    } else {
        Value::Null
    }
}

pub fn parse_clf_line(line: &str) -> Map<String, Value> {
    let mut map = Map::new();
    let clf_re = get_clf_re();
    let request_re = get_request_re();

    if let Some(caps) = clf_re.captures(line) {
        let host = caps.name("host").map(|m| m.as_str()).unwrap_or("");
        let ident = caps.name("ident").map(|m| m.as_str()).unwrap_or("");
        let authuser = caps.name("authuser").map(|m| m.as_str()).unwrap_or("");
        let date = caps.name("date").map(|m| m.as_str()).unwrap_or("");
        let day = caps.name("day").map(|m| m.as_str()).unwrap_or("");
        let month = caps.name("month").map(|m| m.as_str()).unwrap_or("");
        let year = caps.name("year").map(|m| m.as_str()).unwrap_or("");
        let hour = caps.name("hour").map(|m| m.as_str()).unwrap_or("");
        let minute = caps.name("minute").map(|m| m.as_str()).unwrap_or("");
        let second = caps.name("second").map(|m| m.as_str()).unwrap_or("");
        let tz = caps.name("tz").map(|m| m.as_str()).unwrap_or("");
        let request = caps.name("request").map(|m| m.as_str()).unwrap_or("");
        let status = caps.name("status").map(|m| m.as_str()).unwrap_or("");
        let bytes = caps.name("bytes").map(|m| m.as_str()).unwrap_or("");
        let referer = caps.name("referer").map(|m| m.as_str()).unwrap_or("");
        let user_agent = caps.name("user_agent").map(|m| m.as_str()).unwrap_or("");
        let extra = caps.name("extra").map(|m| m.as_str()).unwrap_or("");

        map.insert("host".to_string(), null_if_dash(host));
        map.insert("ident".to_string(), null_if_dash(ident));
        map.insert("authuser".to_string(), null_if_dash(authuser));
        map.insert(
            "date".to_string(),
            if date.is_empty() {
                Value::Null
            } else {
                Value::String(date.to_string())
            },
        );
        map.insert("day".to_string(), parse_int_or_null(day));
        map.insert(
            "month".to_string(),
            if month.is_empty() {
                Value::Null
            } else {
                Value::String(month.to_string())
            },
        );
        map.insert("year".to_string(), parse_int_or_null(year));
        map.insert("hour".to_string(), parse_int_or_null(hour));
        map.insert("minute".to_string(), parse_int_or_null(minute));
        map.insert("second".to_string(), parse_int_or_null(second));
        map.insert("tz".to_string(), null_if_dash(tz));
        map.insert(
            "request".to_string(),
            if request.is_empty() {
                Value::Null
            } else {
                Value::String(request.to_string())
            },
        );
        map.insert("status".to_string(), parse_int_or_null(status));
        map.insert("bytes".to_string(), parse_int_or_null(bytes));
        map.insert("referer".to_string(), null_if_dash(referer));
        map.insert("user_agent".to_string(), null_if_dash(user_agent));
        map.insert("extra".to_string(), null_if_dash(extra));

        // Parse request into method/url/version
        if !request.is_empty() {
            if let Some(req_caps) = request_re.captures(request) {
                map.insert(
                    "request_method".to_string(),
                    Value::String(
                        req_caps
                            .name("request_method")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                );
                map.insert(
                    "request_url".to_string(),
                    Value::String(
                        req_caps
                            .name("request_url")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string(),
                    ),
                );
                let req_version = req_caps
                    .name("request_version")
                    .map(|m| m.as_str())
                    .unwrap_or("");
                map.insert(
                    "request_version".to_string(),
                    if req_version.is_empty() {
                        Value::Null
                    } else {
                        Value::String(req_version.to_string())
                    },
                );
            } else {
                map.insert("request_method".to_string(), Value::Null);
                map.insert("request_url".to_string(), Value::Null);
                map.insert("request_version".to_string(), Value::Null);
            }
        } else {
            map.insert("request_method".to_string(), Value::Null);
            map.insert("request_url".to_string(), Value::Null);
            map.insert("request_version".to_string(), Value::Null);
        }

        // Compute epoch timestamps from date field
        if !date.is_empty() {
            let ts = parse_timestamp(date, &[formats::F1800]);
            map.insert(
                "epoch".to_string(),
                match ts.naive_epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
            map.insert(
                "epoch_utc".to_string(),
                match ts.utc_epoch {
                    Some(e) => Value::Number(e.into()),
                    None => Value::Null,
                },
            );
        } else {
            map.insert("epoch".to_string(), Value::Null);
            map.insert("epoch_utc".to_string(), Value::Null);
        }
    } else {
        // Unparsable line
        map.insert("unparsable".to_string(), Value::String(line.to_string()));
    }

    map
}

impl Parser for ClfParser {
    fn info(&self) -> &'static ParserInfo {
        &CLF_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let records: Vec<Map<String, Value>> = input
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_clf_line)
            .collect();

        Ok(ParseOutput::Array(records))
    }
}

static CLF_PARSER_INSTANCE: ClfParser = ClfParser;

inventory::submit! {
    ParserEntry::new(&CLF_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_array(input: &str) -> Vec<serde_json::Value> {
        let parser = ClfParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Array(arr) => arr.into_iter().map(serde_json::Value::Object).collect(),
            _ => panic!("expected array"),
        }
    }

    #[test]
    fn test_clf_basic() {
        let line = r#"127.0.0.1 user-identifier frank [10/Oct/2000:13:55:36 -0700] "GET /apache_pb.gif HTTP/1.0" 200 2326"#;
        let arr = parse_to_array(line);
        assert_eq!(arr.len(), 1);
        let v = &arr[0];
        assert_eq!(v["host"], "127.0.0.1");
        assert_eq!(v["ident"], "user-identifier");
        assert_eq!(v["authuser"], "frank");
        assert_eq!(v["day"], 10);
        assert_eq!(v["month"], "Oct");
        assert_eq!(v["year"], 2000);
        assert_eq!(v["status"], 200);
        assert_eq!(v["bytes"], 2326);
        assert_eq!(v["request_method"], "GET");
        assert_eq!(v["request_url"], "/apache_pb.gif");
        assert_eq!(v["request_version"], "HTTP/1.0");
    }

    #[test]
    fn test_clf_dash_fields() {
        let line = r#"1.1.1.2 - - [11/Nov/2016:03:04:55 +0100] "GET /" 200 83"#;
        let arr = parse_to_array(line);
        let v = &arr[0];
        assert!(v["ident"].is_null());
        assert!(v["authuser"].is_null());
    }

    #[test]
    fn test_clf_combined_format() {
        let line = r#"1.1.1.2 - - [11/Nov/2016:03:04:55 +0100] "GET / HTTP/1.1" 200 83 "http://example.com" "Mozilla/5.0""#;
        let arr = parse_to_array(line);
        let v = &arr[0];
        assert_eq!(v["referer"], "http://example.com");
        assert_eq!(v["user_agent"], "Mozilla/5.0");
    }

    #[test]
    fn test_clf_unparsable() {
        let line = "this is not a valid log line";
        let arr = parse_to_array(line);
        let v = &arr[0];
        assert!(v.get("unparsable").is_some());
    }
}
