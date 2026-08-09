//! Streaming parser for `traceroute` and `traceroute6` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::{LineParser, Parser, Record, StreamingParser, parse_via_session};
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

pub struct TracerouteStreamParser;

static INFO: ParserInfo = ParserInfo {
    name: "traceroute_s",
    argument: "--traceroute-s",
    version: "1.9.0",
    description: "Streaming parser for `traceroute` and `traceroute6` command output",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command, Tag::Streaming],
    magic_commands: &[],
    streaming: true,
    hidden: false,
    deprecated: false,
};

static TRACEROUTE_STREAM_PARSER: TracerouteStreamParser = TracerouteStreamParser;
inventory::submit! { ParserEntry::new(&TRACEROUTE_STREAM_PARSER) }

// Re-use the get_probes logic from traceroute
static RE_PROBE_ASN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[AS(\d+)\]").expect("valid asn pattern"));

static RE_PROBE_NAME_IP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\S+)\s+\((\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}|[0-9a-fA-F:]+)\)+")
        .expect("valid name/ip pattern")
});

static RE_PROBE_IP_ONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\s+([^(])").expect("valid ip pattern")
});

static RE_PROBE_BSD_IPV6: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[A-Fa-f0-9]{1,4}:){7}[A-Fa-f0-9]{1,4}\b").expect("valid bsd ipv6 pattern")
});

/// Compressed IPv6, which the BSD pattern above cannot match because it insists
/// on all eight groups. Without it every `2605:9000:402:6a01::1` probe came out
/// with a null ip.
static RE_PROBE_IPV6_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(([a-f0-9]*:)+[a-f0-9]+)").expect("valid ipv6 pattern"));

static RE_PROBE_RTT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:(\d+(?:\.?\d+)?)\s+ms|(\s+\*\s+))\s*(!\S*)?").expect("valid rtt pattern")
});

/// One probe under construction. Cloned wholesale when a hop reports several
/// round-trip times for the same host: jc deep-copies the previous probe and
/// swaps in the new time, which is what carries the ASN and name across.
#[derive(Clone, Default)]
struct Probe {
    annotation: Option<String>,
    asn: Option<i64>,
    ip: Option<String>,
    name: Option<String>,
    rtt: Option<f64>,
}

impl Probe {
    fn is_empty(&self) -> bool {
        self.annotation.is_none()
            && self.asn.is_none()
            && self.ip.is_none()
            && self.name.is_none()
            && self.rtt.is_none()
    }

    fn into_record(self) -> Map<String, Value> {
        let mut obj = Map::with_capacity(5);
        obj.insert(
            "annotation".to_string(),
            self.annotation.map_or(Value::Null, Value::String),
        );
        obj.insert(
            "asn".to_string(),
            self.asn.map_or(Value::Null, |n| Value::Number(n.into())),
        );
        obj.insert("ip".to_string(), self.ip.map_or(Value::Null, Value::String));
        obj.insert(
            "name".to_string(),
            self.name.map_or(Value::Null, Value::String),
        );
        obj.insert(
            "rtt".to_string(),
            self.rtt
                .and_then(serde_json::Number::from_f64)
                .map_or(Value::Null, Value::Number),
        );
        obj
    }
}

/// Every token in a hop line, in the order it appears.
struct Token {
    start: usize,
    kind: TokenKind,
    first: String,
    second: Option<String>,
    annotation: Option<String>,
}

#[derive(PartialEq)]
enum TokenKind {
    Asn,
    NameIp,
    IpOnly,
    Ipv6,
    Rtt,
}

fn get_probes(hop_string: &str) -> Vec<Map<String, Value>> {
    let mut tokens: Vec<Token> = Vec::new();

    let mut push = |kind: TokenKind, start: usize, first: String, second: Option<String>| {
        tokens.push(Token {
            start,
            kind,
            first,
            second,
            annotation: None,
        });
    };

    for cap in RE_PROBE_ASN.captures_iter(hop_string) {
        let m = cap.get(0).expect("group 0 always matches");
        push(TokenKind::Asn, m.start(), cap[1].to_string(), None);
    }
    for cap in RE_PROBE_NAME_IP.captures_iter(hop_string) {
        let m = cap.get(0).expect("group 0 always matches");
        push(
            TokenKind::NameIp,
            m.start(),
            cap[1].to_string(),
            Some(cap[2].to_string()),
        );
    }
    for cap in RE_PROBE_IP_ONLY.captures_iter(hop_string) {
        let m = cap.get(0).expect("group 0 always matches");
        push(TokenKind::IpOnly, m.start(), cap[1].to_string(), None);
    }
    for m in RE_PROBE_BSD_IPV6.find_iter(hop_string) {
        push(TokenKind::Ipv6, m.start(), m.as_str().to_string(), None);
    }
    for cap in RE_PROBE_IPV6_ONLY.captures_iter(hop_string) {
        let m = cap.get(0).expect("group 0 always matches");
        push(TokenKind::Ipv6, m.start(), cap[1].to_string(), None);
    }
    for cap in RE_PROBE_RTT.captures_iter(hop_string) {
        let m = cap.get(0).expect("group 0 always matches");
        tokens.push(Token {
            start: m.start(),
            kind: TokenKind::Rtt,
            first: cap.get(1).map_or(String::new(), |g| g.as_str().to_string()),
            second: cap.get(2).map(|g| g.as_str().to_string()),
            annotation: cap.get(3).map(|g| g.as_str().to_string()),
        });
    }

    tokens.sort_by_key(|t| t.start);

    let mut probes: Vec<Map<String, Value>> = Vec::new();
    let mut probe = Probe::default();
    let mut last_probe = Probe::default();
    let mut last_was_rtt = false;

    for token in tokens {
        match token.kind {
            TokenKind::Asn => probe.asn = token.first.parse().ok(),
            TokenKind::NameIp => {
                probe.name = Some(token.first);
                probe.ip = token.second;
            }
            TokenKind::IpOnly | TokenKind::Ipv6 => probe.ip = Some(token.first),
            TokenKind::Rtt => {
                let rtt = (!token.first.is_empty())
                    .then(|| token.first.parse::<f64>().ok())
                    .flatten();

                // A second time for the same host repeats the whole probe.
                if last_was_rtt {
                    probe = last_probe.clone();
                }
                probe.rtt = rtt;
                probe.annotation = token.annotation.filter(|a| !a.is_empty());

                if !probe.is_empty() {
                    probes.push(probe.clone().into_record());
                }
                last_probe = std::mem::take(&mut probe);
            }
        }
        last_was_rtt = matches!(token.kind, TokenKind::Rtt);
    }

    probes
}

static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"traceroute6? to (\S+)\s+\((\d+\.\d+\.\d+\.\d+|[0-9a-fA-F:]+)\)")
        .expect("valid traceroute header pattern")
});

static HOPS_BYTES_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d+) hops max, (\d+) byte packets").expect("valid hops/bytes pattern")
});

static HOP_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\d+)?\s+(.+)$").expect("valid hop pattern"));

impl Parser for TracerouteStreamParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }
        parse_via_session(self, input, quiet)
    }

    fn as_streaming(&self) -> Option<&dyn StreamingParser> {
        Some(self)
    }
}

impl StreamingParser for TracerouteStreamParser {
    fn session(&self) -> Box<dyn LineParser> {
        Box::new(TracerouteSession::default())
    }
}

/// A hop's probes can wrap onto continuation lines, so a hop is only complete
/// when the next numbered hop starts (or the input ends).
#[derive(Default)]
struct TracerouteSession {
    current_hop: Option<(i64, String)>,
}

impl TracerouteSession {
    fn hop_record(hop: (i64, String)) -> Record {
        let (index, text) = hop;
        let probes = get_probes(&text);
        let mut record = Record::with_capacity(3);
        record.insert("type".to_string(), Value::String("hop".to_string()));
        record.insert("hop".to_string(), Value::Number(index.into()));
        record.insert(
            "probes".to_string(),
            Value::Array(probes.into_iter().map(Value::Object).collect()),
        );
        record
    }
}

impl LineParser for TracerouteSession {
    fn parse_line(&mut self, line: &str, _quiet: bool) -> Result<Option<Record>, ParseError> {
        if line.trim().is_empty()
            || line.contains("traceroute: Warning:")
            || line.contains("traceroute6: Warning:")
        {
            return Ok(None);
        }

        if line.starts_with("traceroute") {
            let mut header = Record::with_capacity(5);
            header.insert("type".to_string(), Value::String("header".to_string()));

            if let Some(caps) = HEADER_RE.captures(line) {
                header.insert(
                    "destination_name".to_string(),
                    Value::String(caps.get(1).map_or("", |m| m.as_str()).to_string()),
                );
                header.insert(
                    "destination_ip".to_string(),
                    Value::String(caps.get(2).map_or("", |m| m.as_str()).to_string()),
                );
            }
            if let Some(caps) = HOPS_BYTES_RE.captures(line) {
                let number = |i: usize| -> Value {
                    caps.get(i)
                        .and_then(|m| m.as_str().parse::<i64>().ok())
                        .map_or(Value::Null, |n| Value::Number(n.into()))
                };
                header.insert("max_hops".to_string(), number(1));
                header.insert("data_bytes".to_string(), number(2));
            }
            return Ok(Some(header));
        }

        let Some(caps) = HOP_RE.captures(line) else {
            return Ok(None);
        };
        let text = caps.get(2).map_or("", |m| m.as_str());

        match caps.get(1) {
            Some(hop_num) => {
                let finished = self.current_hop.take();
                self.current_hop = Some((
                    hop_num.as_str().parse::<i64>().unwrap_or(0),
                    text.to_string(),
                ));
                Ok(finished.map(Self::hop_record))
            }
            None => {
                // Continuation of the hop currently being built.
                if let Some((_, ref mut hop_text)) = self.current_hop {
                    hop_text.push(' ');
                    hop_text.push_str(text);
                }
                Ok(None)
            }
        }
    }

    fn finalize(&mut self, _quiet: bool) -> Result<Option<Record>, ParseError> {
        Ok(self.current_hop.take().map(Self::hop_record))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_traceroute_s_ipv4_golden() {
        let input = include_str!("../../../../tests/fixtures/generic/traceroute-n-ipv4.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/traceroute-n-ipv4-streaming.json"
        ))
        .unwrap();
        let result = TracerouteStreamParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_traceroute_s_empty() {
        let result = TracerouteStreamParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_traceroute_s_registered() {
        assert!(jc_rs_core::registry::find_parser("traceroute_s").is_some());
    }
}
