//! Parser for `certbot` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct CertbotParser;

static INFO: ParserInfo = ParserInfo {
    name: "certbot",
    argument: "--certbot",
    version: "1.2.0",
    description: "Converts `certbot` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["certbot"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static CERTBOT_PARSER: CertbotParser = CertbotParser;

inventory::submit! {
    ParserEntry::new(&CERTBOT_PARSER)
}

/// Parse an ISO-8601-like datetime string "2023-05-11 01:33:10+00:00" into a UTC epoch timestamp.
/// Returns None if parsing fails.
fn parse_datetime_to_epoch(s: &str) -> Option<i64> {
    // Try to parse "YYYY-MM-DD HH:MM:SS+HH:MM" or "YYYY-MM-DD HH:MM:SS+00:00"
    // Use a manual parser to avoid dependencies.
    let s = s.trim();

    // Split date and time
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_part = parts[0];
    let time_tz = parts[1];

    // Parse date
    let date_parts: Vec<&str> = date_part.split('-').collect();
    if date_parts.len() != 3 {
        return None;
    }
    let year: i64 = date_parts[0].parse().ok()?;
    let month: i64 = date_parts[1].parse().ok()?;
    let day: i64 = date_parts[2].parse().ok()?;

    // Parse timezone offset: find +/- at end
    let (time_part, tz_offset_secs) = if let Some(pos) = time_tz.rfind('+') {
        let tz = &time_tz[pos + 1..];
        let tz_parts: Vec<&str> = tz.split(':').collect();
        let h: i64 = tz_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let m: i64 = tz_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (&time_tz[..pos], h * 3600 + m * 60)
    } else if let Some(pos) = time_tz.rfind('-') {
        // negative offset
        let tz = &time_tz[pos + 1..];
        let tz_parts: Vec<&str> = tz.split(':').collect();
        let h: i64 = tz_parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let m: i64 = tz_parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (&time_tz[..pos], -(h * 3600 + m * 60))
    } else {
        (time_tz, 0i64)
    };

    // Parse time
    let time_parts: Vec<&str> = time_part.split(':').collect();
    if time_parts.len() != 3 {
        return None;
    }
    let hour: i64 = time_parts[0].parse().ok()?;
    let minute: i64 = time_parts[1].parse().ok()?;
    let second: i64 = time_parts[2].parse().ok()?;

    // Days since epoch calculation
    // Using algorithm from https://howardhinnant.github.io/date_algorithms.html
    let epoch = days_since_epoch(year, month, day)?;
    let utc_epoch = epoch * 86400 + hour * 3600 + minute * 60 + second - tz_offset_secs;

    Some(utc_epoch)
}

fn days_since_epoch(y: i64, m: i64, d: i64) -> Option<i64> {
    // Days since 1970-01-01 using Gregorian calendar
    let y = if m <= 2 { y - 1 } else { y };
    let m = if m <= 2 { m + 9 } else { m - 3 };

    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let day = era * 146097 + doe - 719468;

    Some(day)
}

fn parse_iso(s: &str) -> String {
    // Convert "2023-05-11 01:33:10+00:00" to "2023-05-11T01:33:10+00:00"
    s.trim().replacen(' ', "T", 1)
}

impl Parser for CertbotParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        let mut raw_output: Map<String, Value> = Map::new();
        let mut cert_list: Vec<Map<String, Value>> = Vec::new();
        let mut cert_dict: Option<Map<String, Value>> = None;
        let mut acct_dict: Map<String, Value> = Map::new();

        let is_certificates = input
            .lines()
            .any(|l| l.trim() == "Found the following certs:");

        let cmd_option = if is_certificates {
            "certificates"
        } else {
            "account"
        };

        for line in input.lines() {
            if line.trim().is_empty() {
                continue;
            }

            if cmd_option == "certificates" {
                if line.starts_with("  Certificate Name:") {
                    if let Some(cert) = cert_dict.take() {
                        cert_list.push(cert);
                    }
                    let mut new_cert = Map::new();
                    if let Some(name) = line.split_whitespace().last() {
                        new_cert.insert("name".to_string(), Value::String(name.to_string()));
                    }
                    cert_dict = Some(new_cert);
                    continue;
                }

                if let Some(ref mut cert) = cert_dict {
                    if line.starts_with("    Serial Number:") {
                        if let Some(val) = line.split_whitespace().last() {
                            cert.insert(
                                "serial_number".to_string(),
                                Value::String(val.to_string()),
                            );
                        }
                    } else if line.starts_with("    Key Type:") {
                        if let Some(val) = line.split(": ").nth(1) {
                            cert.insert(
                                "key_type".to_string(),
                                Value::String(val.trim().to_string()),
                            );
                        }
                    } else if line.starts_with("    Domains:") {
                        if let Some(val) = line.split(": ").nth(1) {
                            let domains: Vec<Value> = val
                                .split_whitespace()
                                .map(|d| Value::String(d.to_string()))
                                .collect();
                            cert.insert("domains".to_string(), Value::Array(domains));
                        }
                    } else if line.starts_with("    Expiry Date:") {
                        if let Some(val) = line.splitn(2, ": ").nth(1) {
                            // e.g. "2023-05-11 01:33:10+00:00 (VALID: 63 days)"
                            let parts: Vec<&str> = val.splitn(2, " (").collect();
                            let date_str = parts[0].trim();
                            cert.insert(
                                "expiration_date".to_string(),
                                Value::String(date_str.to_string()),
                            );
                            if parts.len() > 1 {
                                let validity = parts[1]
                                    .trim_end_matches(')')
                                    .replace("VALID: ", "")
                                    .replace("INVALID: ", "");
                                cert.insert(
                                    "validity".to_string(),
                                    Value::String(validity.trim().to_string()),
                                );
                            }
                            // Add epoch fields
                            if let Some(epoch) = parse_datetime_to_epoch(date_str) {
                                cert.insert(
                                    "expiration_date_epoch_utc".to_string(),
                                    Value::Number(epoch.into()),
                                );
                                // naive epoch (local time) - we just use UTC here as approximation
                                // The Python jc parser uses local time for naive epoch
                                // For a portable implementation we use UTC
                                cert.insert(
                                    "expiration_date_epoch".to_string(),
                                    Value::Number(epoch.into()),
                                );
                                cert.insert(
                                    "expiration_date_iso".to_string(),
                                    Value::String(parse_iso(date_str)),
                                );
                            }
                        }
                    } else if line.starts_with("    Certificate Path:") {
                        if let Some(val) = line.split(": ").nth(1) {
                            cert.insert(
                                "certificate_path".to_string(),
                                Value::String(val.trim().to_string()),
                            );
                        }
                    } else if line.starts_with("    Private Key Path:") {
                        if let Some(val) = line.split(": ").nth(1) {
                            cert.insert(
                                "private_key_path".to_string(),
                                Value::String(val.trim().to_string()),
                            );
                        }
                    }
                }
            } else {
                // account mode
                if line.starts_with("Account details for server") {
                    // "Account details for server https://...:":
                    if let Some(server) = line.split_whitespace().last() {
                        let server = server.trim_end_matches(':');
                        acct_dict.insert("server".to_string(), Value::String(server.to_string()));
                    }
                } else if line.starts_with("  Account URL:") {
                    if let Some(val) = line.split_whitespace().last() {
                        acct_dict.insert("url".to_string(), Value::String(val.to_string()));
                    }
                } else if line.starts_with("  Email contact:") {
                    if let Some(val) = line.split_whitespace().last() {
                        acct_dict.insert("email".to_string(), Value::String(val.to_string()));
                    }
                }
            }
        }

        if !acct_dict.is_empty() {
            raw_output.insert("account".to_string(), Value::Object(acct_dict));
        }

        if let Some(cert) = cert_dict.take() {
            cert_list.push(cert);
        }

        if !cert_list.is_empty() {
            raw_output.insert(
                "certificates".to_string(),
                Value::Array(cert_list.into_iter().map(Value::Object).collect()),
            );
        }

        Ok(ParseOutput::Object(raw_output))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certbot_certs() {
        let input = r#"Saving debug log to /var/log/letsencrypt/letsencrypt.log

- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -
Found the following certs:
  Certificate Name: example.com
    Serial Number: 3f7axxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
    Key Type: RSA
    Domains: example.com www.example.com
    Expiry Date: 2023-05-11 01:33:10+00:00 (VALID: 63 days)
    Certificate Path: /etc/letsencrypt/live/example.com/fullchain.pem
    Private Key Path: /etc/letsencrypt/live/example.com/privkey.pem
- - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - - -"#;
        let parser = CertbotParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.contains_key("certificates"));
            if let Some(Value::Array(certs)) = obj.get("certificates") {
                assert_eq!(certs.len(), 1);
                if let Value::Object(cert) = &certs[0] {
                    assert_eq!(
                        cert.get("name"),
                        Some(&Value::String("example.com".to_string()))
                    );
                    assert_eq!(
                        cert.get("key_type"),
                        Some(&Value::String("RSA".to_string()))
                    );
                }
            }
        } else {
            panic!("Expected Object");
        }
    }

    #[test]
    fn test_certbot_empty() {
        let parser = CertbotParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
