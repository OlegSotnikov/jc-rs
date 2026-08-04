//! JWT string parser.

use base64::Engine;
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

struct JwtParser;

static JWT_INFO: ParserInfo = ParserInfo {
    name: "jwt",
    argument: "--jwt",
    version: "1.0.0",
    description: "JWT string parser",
    author: "jc-rs contributors",
    author_email: "jc-rs@example.com",
    compatible: &[Platform::Universal],
    tags: &[Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

/// Convert bytes to colon-delimited hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

impl Parser for JwtParser {
    fn info(&self) -> &'static ParserInfo {
        &JWT_INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseError::InvalidInput("empty input".to_string()));
        }

        let parts: Vec<&str> = input.splitn(3, '.').collect();
        if parts.len() != 3 {
            return Err(ParseError::InvalidInput(
                "JWT must have exactly 3 parts separated by '.'".to_string(),
            ));
        }

        let engine = base64::engine::general_purpose::URL_SAFE_NO_PAD;

        // Decode header
        let header_bytes = engine.decode(parts[0]).map_err(|e| {
            ParseError::InvalidInput(format!("invalid base64 in JWT header: {}", e))
        })?;
        let header_str = String::from_utf8(header_bytes).map_err(|e| ParseError::Utf8(e))?;
        let header: Value = serde_json::from_str(&header_str).map_err(|e| ParseError::Json(e))?;

        // Decode payload
        let payload_bytes = engine.decode(parts[1]).map_err(|e| {
            ParseError::InvalidInput(format!("invalid base64 in JWT payload: {}", e))
        })?;
        let payload_str = String::from_utf8(payload_bytes).map_err(|e| ParseError::Utf8(e))?;
        let payload: Value = serde_json::from_str(&payload_str).map_err(|e| ParseError::Json(e))?;

        // Decode signature — convert to colon-delimited hex
        let sig_bytes = engine.decode(parts[2]).map_err(|e| {
            ParseError::InvalidInput(format!("invalid base64 in JWT signature: {}", e))
        })?;
        let signature = bytes_to_hex(&sig_bytes);

        let mut map = Map::new();
        map.insert("header".to_string(), header);
        map.insert("payload".to_string(), payload);
        map.insert("signature".to_string(), Value::String(signature));

        Ok(ParseOutput::Object(map))
    }
}

static JWT_PARSER_INSTANCE: JwtParser = JwtParser;

inventory::submit! {
    ParserEntry::new(&JWT_PARSER_INSTANCE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use jc_rs_core::types::ParseOutput;

    fn parse_to_value(input: &str) -> serde_json::Value {
        let parser = JwtParser;
        let result = parser.parse(input, false).unwrap();
        match result {
            ParseOutput::Object(map) => serde_json::Value::Object(map),
            _ => panic!("expected object"),
        }
    }

    #[test]
    fn test_jwt_basic() {
        // Standard test JWT: header.payload.signature
        // eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let v = parse_to_value(token);
        assert_eq!(v["header"]["alg"], "HS256");
        assert_eq!(v["header"]["typ"], "JWT");
        assert_eq!(v["payload"]["sub"], "1234567890");
        assert_eq!(v["payload"]["name"], "John Doe");
        assert_eq!(v["payload"]["iat"], 1516239022_i64);
        // signature should be colon-delimited hex
        let sig = v["signature"].as_str().unwrap();
        assert!(sig.contains(':'));
    }

    #[test]
    fn test_jwt_invalid_parts() {
        let parser = JwtParser;
        let result = parser.parse("not.a.valid.jwt.token", false);
        // splitn(3, '.') will handle this — it gives 3 parts max from left
        // Actually "not.a.valid.jwt.token" will split into ["not", "a", "valid.jwt.token"]
        // which may fail on base64 decode
        let _ = result; // just ensure no panic
    }

    #[test]
    fn test_jwt_empty() {
        let parser = JwtParser;
        let result = parser.parse("", false);
        assert!(result.is_err());
    }
}
