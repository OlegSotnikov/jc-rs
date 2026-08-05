//! Parser for `curl --head` / `curl -v` command output.
//!
//! Strips curl verbose prefix characters (`> `, `< `, `* `) and delegates
//! to the HTTP headers parser.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};

use super::http_headers::HttpHeadersParser;

pub struct CurlHeadParser;

static INFO: ParserInfo = ParserInfo {
    name: "curl_head",
    argument: "--curl-head",
    version: "1.1.0",
    description: "Converts `curl --head` or `curl -v` output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::Windows],
    tags: &[Tag::Command],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CURL_HEAD_PARSER: CurlHeadParser = CurlHeadParser;
inventory::submit! { ParserEntry::new(&CURL_HEAD_PARSER) }

static HTTP_HEADERS_DELEGATE: HttpHeadersParser = HttpHeadersParser;

/// Strip curl verbose-mode prefixes and filter out informational lines.
/// - Lines starting with `> ` are request header lines → strip prefix
/// - Lines starting with `< ` are response header lines → strip prefix
/// - Lines starting with `* ` are informational → skip
/// - Lines starting with `{ ` or `} ` are timing → skip
/// - Plain lines (no prefix) are kept as-is (e.g., from `curl --head`)
fn strip_curl_prefixes(input: &str) -> String {
    let mut out = String::new();
    for line in input.lines() {
        if line.starts_with("* ") || line.starts_with("{ ") || line.starts_with("} ") {
            // Informational / timing: skip
            continue;
        } else if line.starts_with("> ") {
            out.push_str(&line[2..]);
            out.push('\n');
        } else if line.starts_with("< ") {
            out.push_str(&line[2..]);
            out.push('\n');
        } else {
            // Plain output (curl --head doesn't add prefixes)
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

impl Parser for CurlHeadParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let cleaned = strip_curl_prefixes(input);
        HTTP_HEADERS_DELEGATE.parse(&cleaned, quiet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::traits::Parser;

    #[test]
    fn test_curl_head_example_com_golden() {
        let input =
            include_str!("../../../../tests/fixtures/generic/curl_head--ILvs-example-com.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/curl_head--ILvs-example-com.json"
        ))
        .unwrap();
        let result = CurlHeadParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_curl_head_google_com_golden() {
        let input =
            include_str!("../../../../tests/fixtures/generic/curl_head--ILvs-google-com.out");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/generic/curl_head--ILvs-google-com.json"
        ))
        .unwrap();
        let result = CurlHeadParser.parse(input, false).unwrap();
        let actual = serde_json::to_value(result).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_curl_head_empty() {
        let result = CurlHeadParser.parse("", false).unwrap();
        assert!(matches!(result, ParseOutput::Array(v) if v.is_empty()));
    }

    #[test]
    fn test_curl_head_registered() {
        assert!(jc_rs_core::registry::find_parser("curl_head").is_some());
    }
}
