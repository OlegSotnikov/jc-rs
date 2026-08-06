//! Parser for `tracepath` and `tracepath6` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

// Compiled once for the process. Each of these sat inside a function the
// parser calls per line or per record, so the pattern was rebuilt from
// source for every one of them.
static RE_RE_TTL_HOST: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s?(\d+)(\??): +(no reply|\S+)").expect("RE_RE_TTL_HOST is a valid pattern")
});
static RE_RE_PMTU: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" pmtu (\d+)").expect("RE_RE_PMTU is a valid pattern"));
static RE_RE_REPLY_MS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" (\d+\.\d+)ms").expect("RE_RE_REPLY_MS is a valid pattern"));
static RE_RE_ASYMM: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" asymm +(\d+)").expect("RE_RE_ASYMM is a valid pattern"));
static RE_RE_REACHED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r" reached").expect("RE_RE_REACHED is a valid pattern"));
static RE_RE_SUMMARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\s+Resume: pmtu (\d+)(?:\s+hops (\d+))?(?:\s+back (\d+))?")
        .expect("RE_RE_SUMMARY is a valid pattern")
});

pub struct TracepathParser;

static INFO: ParserInfo = ParserInfo {
    name: "tracepath",
    argument: "--tracepath",
    version: "1.4.0",
    description: "Converts `tracepath` and `tracepath6` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["tracepath", "tracepath6"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static TRACEPATH_PARSER: TracepathParser = TracepathParser;
inventory::submit! { ParserEntry::new(&TRACEPATH_PARSER) }

fn str_to_int_opt(s: &str) -> Option<i64> {
    s.trim().parse::<i64>().ok()
}

fn str_to_float_opt(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok()
}

impl Parser for TracepathParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // Alternation order matters: `\S+` first would match just "no" in
        // "no reply" and report it as a hostname.
        let re_ttl_host = &*RE_RE_TTL_HOST;
        let re_pmtu = &*RE_RE_PMTU;
        let re_reply_ms = &*RE_RE_REPLY_MS;
        let re_asymm = &*RE_RE_ASYMM;
        let re_reached = &*RE_RE_REACHED;
        let re_summary = &*RE_RE_SUMMARY;

        let mut hops: Vec<Value> = Vec::new();
        let mut top_pmtu: Option<i64> = None;
        let mut forward_hops: Option<i64> = None;
        let mut return_hops: Option<i64> = None;

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            // Check for summary line
            if let Some(caps) = re_summary.captures(line) {
                top_pmtu = caps.get(1).and_then(|m| str_to_int_opt(m.as_str()));
                forward_hops = caps.get(2).and_then(|m| str_to_int_opt(m.as_str()));
                return_hops = caps.get(3).and_then(|m| str_to_int_opt(m.as_str()));
                continue;
            }

            // Check for hop line
            if let Some(caps) = re_ttl_host.captures(line) {
                let ttl = str_to_int_opt(caps.get(1).map_or("", |m| m.as_str()));
                let guess = caps.get(2).map_or("", |m| m.as_str()) == "?";
                let host_str = caps.get(3).map_or("", |m| m.as_str());
                let host = if host_str == "no reply" {
                    None
                } else {
                    Some(host_str.to_string())
                };

                let reply_ms = re_reply_ms
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .and_then(|m| str_to_float_opt(m.as_str()));

                let pmtu = re_pmtu
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .and_then(|m| str_to_int_opt(m.as_str()));

                // If this line contains the overall pmtu (e.g., "pmtu 1500" at start),
                // and we haven't seen a ttl_host yet, store as top-level pmtu
                if top_pmtu.is_none()
                    && let Some(p) = &pmtu
                {
                    // Only set if this is a header-style line (no hop info yet)
                    if ttl.is_none() {
                        top_pmtu = Some(*p);
                        continue;
                    }
                }

                let asymm = re_asymm
                    .captures(line)
                    .and_then(|c| c.get(1))
                    .and_then(|m| str_to_int_opt(m.as_str()));

                let reached = re_reached.is_match(line);

                let mut hop = Map::new();
                hop.insert(
                    "ttl".to_string(),
                    ttl.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
                );
                hop.insert("guess".to_string(), Value::Bool(guess));
                hop.insert(
                    "host".to_string(),
                    host.map(Value::String).unwrap_or(Value::Null),
                );
                hop.insert(
                    "reply_ms".to_string(),
                    reply_ms
                        .and_then(serde_json::Number::from_f64)
                        .map(Value::Number)
                        .unwrap_or(Value::Null),
                );
                hop.insert(
                    "pmtu".to_string(),
                    pmtu.map(|n| Value::Number(n.into())).unwrap_or(Value::Null),
                );
                hop.insert(
                    "asymmetric_difference".to_string(),
                    asymm
                        .map(|n| Value::Number(n.into()))
                        .unwrap_or(Value::Null),
                );
                hop.insert("reached".to_string(), Value::Bool(reached));

                hops.push(Value::Object(hop));
            } else if line.contains("pmtu") && hops.is_empty() {
                // Header-style pmtu line like "     Too many hops: pmtu 1500"
                if let Some(caps) = re_pmtu.captures(line) {
                    top_pmtu = caps.get(1).and_then(|m| str_to_int_opt(m.as_str()));
                }
            }
        }

        // If we never found a summary line, try to get pmtu from first hop
        if top_pmtu.is_none()
            && let Some(first_hop) = hops.first()
            && let Some(obj) = first_hop.as_object()
            && let Some(p) = obj.get("pmtu")
        {
            top_pmtu = p.as_i64();
        }

        let mut result = Map::new();
        result.insert(
            "pmtu".to_string(),
            top_pmtu
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        result.insert(
            "forward_hops".to_string(),
            forward_hops
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        result.insert(
            "return_hops".to_string(),
            return_hops
                .map(|n| Value::Number(n.into()))
                .unwrap_or(Value::Null),
        );
        result.insert("hops".to_string(), Value::Array(hops));

        Ok(ParseOutput::Object(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_tracepath_centos_golden() {
        let input = include_str!("../../../../tests/fixtures/centos-7.7/tracepath.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/centos-7.7/tracepath.json"
        ))
        .unwrap();
        let result = TracepathParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_tracepath_empty() {
        let result = TracepathParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Object(m) if m.is_empty()));
    }

    #[test]
    fn test_tracepath_registered() {
        assert!(jc_rs_core::registry::find_parser("tracepath").is_some());
    }
}
