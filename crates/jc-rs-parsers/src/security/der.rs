//! Minimal ASN.1/DER parser for X.509 certificates, CRLs, and CSRs.
//!
//! This is a simplified implementation that handles the most common
//! fields used in X.509 certificates. It does not attempt to be a
//! complete ASN.1 implementation.

use serde_json::{Map, Value};

// ASN.1 tag constants
pub const TAG_BOOLEAN: u8 = 0x01;
pub const TAG_INTEGER: u8 = 0x02;
pub const TAG_BITSTRING: u8 = 0x03;
pub const TAG_OCTETSTRING: u8 = 0x04;
pub const TAG_NULL: u8 = 0x05;
pub const TAG_OID: u8 = 0x06;
pub const TAG_UTF8STRING: u8 = 0x0C;
pub const TAG_SEQUENCE: u8 = 0x30;
pub const TAG_SET: u8 = 0x31;
pub const TAG_PRINTABLESTRING: u8 = 0x13;
pub const TAG_IA5STRING: u8 = 0x16;
pub const TAG_UTCTIME: u8 = 0x17;
pub const TAG_GENERALIZEDTIME: u8 = 0x18;
pub const TAG_BMPSTRING: u8 = 0x1E;
pub const TAG_CONTEXT: u8 = 0xA0; // context-specific constructed [0]

/// Well-known OID to name mappings
pub fn oid_to_name(oid: &str) -> &'static str {
    match oid {
        "2.5.4.3" => "common_name",
        "2.5.4.4" => "surname",
        "2.5.4.5" => "serial_number",
        "2.5.4.6" => "country_name",
        "2.5.4.7" => "locality_name",
        "2.5.4.8" => "state_or_province_name",
        "2.5.4.9" => "street_address",
        "2.5.4.10" => "organization_name",
        "2.5.4.11" => "organizational_unit_name",
        "2.5.4.12" => "title",
        "2.5.4.20" => "telephone_number",
        "1.2.840.113549.1.9.1" => "email_address",

        // Signature algorithms
        "1.2.840.113549.1.1.1" => "rsa",
        "1.2.840.113549.1.1.5" => "sha1_rsa",
        "1.2.840.113549.1.1.11" => "sha256_rsa",
        "1.2.840.113549.1.1.12" => "sha384_rsa",
        "1.2.840.113549.1.1.13" => "sha512_rsa",
        "1.2.840.113549.1.1.14" => "sha224_rsa",
        "1.2.840.10040.4.3" => "sha1_dsa",
        "1.2.840.10045.4.3.1" => "sha224_ecdsa",
        "1.2.840.10045.4.3.2" => "sha256_ecdsa",
        "1.2.840.10045.4.3.3" => "sha384_ecdsa",
        "1.2.840.10045.4.3.4" => "sha512_ecdsa",
        "1.2.840.10045.2.1" => "ec",
        "1.3.132.0.34" => "secp384r1",
        "1.3.132.0.35" => "secp521r1",
        "1.2.840.10045.3.1.7" => "secp256r1",
        "1.3.132.0.10" => "secp256k1",

        // Extensions
        "2.5.29.9" => "subject_directory_attributes",
        "2.5.29.14" => "key_identifier",
        "2.5.29.15" => "key_usage",
        "2.5.29.17" => "subject_alt_name",
        "2.5.29.18" => "issuer_alt_name",
        "2.5.29.19" => "basic_constraints",
        "2.5.29.20" => "crl_number",
        "2.5.29.21" => "crl_reason",
        "2.5.29.24" => "invalidity_date",
        "2.5.29.31" => "crl_distribution_points",
        "2.5.29.32" => "certificate_policies",
        "2.5.29.35" => "authority_key_identifier",
        "2.5.29.36" => "policy_constraints",
        "2.5.29.37" => "extended_key_usage",
        "1.3.6.1.5.5.7.1.1" => "authority_information_access",
        "1.3.6.1.4.1.11129.2.4.2" => "signed_certificate_timestamp_list",
        "2.16.840.1.113730.1.1" => "netscape_certificate_type",
        "2.16.840.1.113730.1.13" => "2.16.840.1.113730.1.13",

        // CSR attributes (PKCS#9)
        "1.2.840.113549.1.9.14" => "extension_request",

        // Microsoft OIDs
        "1.3.6.1.4.1.311.13.2.3" => "microsoft_os_version",
        "1.3.6.1.4.1.311.21.20" => "microsoft_request_client_info",
        "1.3.6.1.4.1.311.13.2.2" => "microsoft_enrollment_csp_provider",

        _ => "",
    }
}

/// Convert bytes to colon-delimited hex string
pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(":")
}

/// Convert bytes to string using \xNN escapes for non-ASCII bytes (mimics Python's behavior).
/// Valid ASCII bytes are kept as-is; bytes >= 0x80 are rendered as \xNN.
fn bytes_to_python_repr(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &b in bytes {
        if b.is_ascii() {
            s.push(b as char);
        } else {
            s.push_str(&format!("\\x{:02x}", b));
        }
    }
    s
}

/// DER TLV (Tag-Length-Value)
#[derive(Debug, Clone)]
pub struct Tlv<'a> {
    pub tag: u8,
    pub value: &'a [u8],
}

/// Parse one TLV from a byte slice, returning the TLV and the remaining bytes.
pub fn parse_tlv(data: &[u8]) -> Option<(Tlv<'_>, &[u8])> {
    if data.is_empty() {
        return None;
    }
    let tag = data[0];
    let (len, len_bytes) = parse_length(&data[1..])?;
    let header_len = 1 + len_bytes;
    if data.len() < header_len + len {
        return None;
    }
    let value = &data[header_len..header_len + len];
    let remaining = &data[header_len + len..];
    Some((Tlv { tag, value }, remaining))
}

/// Parse DER length encoding. Returns (length, bytes_consumed).
pub fn parse_length(data: &[u8]) -> Option<(usize, usize)> {
    if data.is_empty() {
        return None;
    }
    let first = data[0];
    if first < 0x80 {
        Some((first as usize, 1))
    } else {
        let num_bytes = (first & 0x7F) as usize;
        if data.len() < 1 + num_bytes {
            return None;
        }
        let mut len = 0usize;
        for i in 0..num_bytes {
            len = (len << 8) | data[1 + i] as usize;
        }
        Some((len, 1 + num_bytes))
    }
}

/// Parse all TLVs from a byte slice.
pub fn parse_sequence_items(data: &[u8]) -> Vec<Tlv<'_>> {
    let mut result = Vec::new();
    let mut remaining = data;
    while !remaining.is_empty() {
        if let Some((tlv, rest)) = parse_tlv(remaining) {
            result.push(tlv);
            remaining = rest;
        } else {
            break;
        }
    }
    result
}

/// Parse an OID from DER encoding.
pub fn parse_oid(data: &[u8]) -> String {
    if data.is_empty() {
        return String::new();
    }
    let mut components = Vec::new();
    let first = data[0];
    components.push((first / 40).to_string());
    components.push((first % 40).to_string());

    let mut i = 1;
    while i < data.len() {
        let mut value: u64 = 0;
        loop {
            if i >= data.len() {
                break;
            }
            let byte = data[i];
            i += 1;
            value = (value << 7) | (byte & 0x7F) as u64;
            if byte & 0x80 == 0 {
                break;
            }
        }
        components.push(value.to_string());
    }

    components.join(".")
}

/// Parse an integer from DER encoding, returning colon-delimited hex if multi-byte,
/// or a number if fits in i64.
pub fn parse_integer_hex(data: &[u8]) -> String {
    bytes_to_hex(data)
}

/// Parse a string value (UTF8String, PrintableString, IA5String, etc.)
pub fn parse_string(tag: u8, data: &[u8]) -> String {
    match tag {
        TAG_UTF8STRING | TAG_PRINTABLESTRING | TAG_IA5STRING => {
            String::from_utf8_lossy(data).to_string()
        }
        TAG_BMPSTRING => {
            // UTF-16 BE
            let chars: Vec<u16> = data
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| (c[0] as u16) << 8 | c[1] as u16)
                .collect();
            String::from_utf16_lossy(&chars)
        }
        _ => String::from_utf8_lossy(data).to_string(),
    }
}

/// Parse a UTCTime or GeneralizedTime string into epoch seconds (UTC).
pub fn parse_time(tag: u8, data: &[u8]) -> Option<i64> {
    let s = String::from_utf8_lossy(data);
    let s = s.trim_end_matches('Z');

    if tag == TAG_UTCTIME {
        // Format: YYMMDDHHMMSS
        if s.len() < 12 {
            return None;
        }
        let yy: i64 = s[..2].parse().ok()?;
        let mm: i64 = s[2..4].parse().ok()?;
        let dd: i64 = s[4..6].parse().ok()?;
        let hh: i64 = s[6..8].parse().ok()?;
        let mi: i64 = s[8..10].parse().ok()?;
        let ss: i64 = s[10..12].parse().ok()?;
        let year = if yy >= 50 { 1900 + yy } else { 2000 + yy };
        let days = days_since_epoch(year, mm, dd)?;
        Some(days * 86400 + hh * 3600 + mi * 60 + ss)
    } else {
        // GeneralizedTime: YYYYMMDDHHMMSS
        if s.len() < 14 {
            return None;
        }
        let year: i64 = s[..4].parse().ok()?;
        let mm: i64 = s[4..6].parse().ok()?;
        let dd: i64 = s[6..8].parse().ok()?;
        let hh: i64 = s[8..10].parse().ok()?;
        let mi: i64 = s[10..12].parse().ok()?;
        let ss: i64 = s[12..14].parse().ok()?;
        let days = days_since_epoch(year, mm, dd)?;
        Some(days * 86400 + hh * 3600 + mi * 60 + ss)
    }
}

/// Format epoch as ISO-8601 UTC string.
pub fn epoch_to_iso(epoch: i64) -> String {
    let secs = epoch;
    let days_since_1970 = secs.div_euclid(86400);
    let time_of_day = secs.rem_euclid(86400);

    let h = time_of_day / 3600;
    let m = (time_of_day % 3600) / 60;
    let s = time_of_day % 60;

    // Convert days since epoch to Y-M-D
    // Using algorithm from https://howardhinnant.github.io/date_algorithms.html
    let z = days_since_1970 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m_val = if mp < 10 { mp + 3 } else { mp - 9 };
    let y_val = if m_val <= 2 { y + 1 } else { y };

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}+00:00",
        y_val, m_val, d, h, m, s
    )
}

fn days_since_epoch(y: i64, m: i64, d: i64) -> Option<i64> {
    let y = if m <= 2 { y - 1 } else { y };
    let m = if m <= 2 { m + 9 } else { m - 3 };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let doy = (153 * m + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

/// Decode a PEM file and return the DER bytes for each certificate.
pub fn decode_pem(data: &str, expected_label: &str) -> Vec<Vec<u8>> {
    let mut results = Vec::new();
    let mut collecting = false;
    let mut b64 = String::new();

    let begin_marker = format!("-----BEGIN {}-----", expected_label);
    let end_marker = format!("-----END {}-----", expected_label);

    for line in data.lines() {
        let line = line.trim();
        if line == begin_marker {
            collecting = true;
            b64.clear();
        } else if line == end_marker {
            collecting = false;
            if let Ok(bytes) = base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                b64.replace('\n', ""),
            ) {
                results.push(bytes);
            }
            b64.clear();
        } else if collecting {
            b64.push_str(line);
        }
    }

    results
}

/// Parse an X.509 Name (SEQUENCE of SET of SEQUENCE of OID, Value)
pub fn parse_name(data: &[u8]) -> Map<String, Value> {
    let mut result = Map::new();
    let rdns = parse_sequence_items(data);

    for rdn_tlv in &rdns {
        if rdn_tlv.tag != TAG_SET {
            continue;
        }
        let atvs = parse_sequence_items(rdn_tlv.value);
        for atv_tlv in &atvs {
            if atv_tlv.tag != TAG_SEQUENCE {
                continue;
            }
            let atv_items = parse_sequence_items(atv_tlv.value);
            if atv_items.len() < 2 {
                continue;
            }
            if atv_items[0].tag != TAG_OID {
                continue;
            }
            let oid = parse_oid(atv_items[0].value);
            let name = oid_to_name(&oid);
            let key = if name.is_empty() {
                oid.clone()
            } else {
                name.to_string()
            };
            let value_str = parse_string(atv_items[1].tag, atv_items[1].value);

            // Handle multiple values for same key (like multiple OUs)
            if let Some(existing) = result.get(&key) {
                match existing {
                    Value::Array(arr) => {
                        let mut arr = arr.clone();
                        arr.push(Value::String(value_str));
                        result.insert(key, Value::Array(arr));
                    }
                    Value::String(s) => {
                        let existing_str = s.clone();
                        result.insert(
                            key,
                            Value::Array(vec![
                                Value::String(existing_str),
                                Value::String(value_str),
                            ]),
                        );
                    }
                    _ => {
                        result.insert(key, Value::String(value_str));
                    }
                }
            } else {
                result.insert(key, Value::String(value_str));
            }
        }
    }

    result
}

/// Parse an AlgorithmIdentifier SEQUENCE (OID + optional params)
pub fn parse_algorithm_identifier(data: &[u8]) -> Map<String, Value> {
    let mut result = Map::new();
    let items = parse_sequence_items(data);

    if items.is_empty() {
        return result;
    }

    if items[0].tag == TAG_OID {
        let oid = parse_oid(items[0].value);
        let name = oid_to_name(&oid);
        let algo_str = if name.is_empty() {
            oid
        } else {
            name.to_string()
        };
        result.insert("algorithm".to_string(), Value::String(algo_str));
    }

    if items.len() > 1 && items[1].tag != TAG_NULL {
        // Parameters present
        if items[1].tag == TAG_OID {
            let oid = parse_oid(items[1].value);
            let name = oid_to_name(&oid);
            let param_str = if name.is_empty() {
                oid
            } else {
                name.to_string()
            };
            result.insert("parameters".to_string(), Value::String(param_str));
        } else {
            result.insert("parameters".to_string(), Value::Null);
        }
    } else {
        result.insert("parameters".to_string(), Value::Null);
    }

    result
}

/// Parse a validity structure (2 times)
pub fn parse_validity(data: &[u8]) -> Map<String, Value> {
    let mut result = Map::new();
    let items = parse_sequence_items(data);
    if items.len() >= 2 {
        if let Some(epoch) = parse_time(items[0].tag, items[0].value) {
            result.insert("not_before".to_string(), Value::Number(epoch.into()));
            result.insert(
                "not_before_iso".to_string(),
                Value::String(epoch_to_iso(epoch)),
            );
        }
        if let Some(epoch) = parse_time(items[1].tag, items[1].value) {
            result.insert("not_after".to_string(), Value::Number(epoch.into()));
            result.insert(
                "not_after_iso".to_string(),
                Value::String(epoch_to_iso(epoch)),
            );
        }
    }
    result
}

/// Parse SubjectPublicKeyInfo
pub fn parse_spki(data: &[u8]) -> Map<String, Value> {
    let mut result = Map::new();
    let items = parse_sequence_items(data);
    if items.is_empty() {
        return result;
    }

    let algo = parse_algorithm_identifier(items[0].value);
    result.insert("algorithm".to_string(), Value::Object(algo.clone()));

    // Parse the public key bit string
    if items.len() > 1 && items[1].tag == TAG_BITSTRING {
        let key_bytes = if items[1].value.len() > 1 {
            &items[1].value[1..] // skip unused bits byte
        } else {
            &items[1].value[..]
        };

        let algo_str = algo.get("algorithm").and_then(|v| v.as_str()).unwrap_or("");

        if algo_str == "rsa" {
            // RSA public key: SEQUENCE { INTEGER modulus, INTEGER exponent }
            if let Some((seq_tlv, _)) = parse_tlv(key_bytes) {
                if seq_tlv.tag == TAG_SEQUENCE {
                    let rsa_items = parse_sequence_items(seq_tlv.value);
                    if rsa_items.len() >= 2 {
                        // Strip leading 0x00 sign byte before hex encoding (jc behavior)
                        let modulus_bytes =
                            if !rsa_items[0].value.is_empty() && rsa_items[0].value[0] == 0 {
                                &rsa_items[0].value[1..]
                            } else {
                                rsa_items[0].value
                            };
                        let modulus_hex = bytes_to_hex(modulus_bytes);
                        let exponent = parse_big_int_as_u64(rsa_items[1].value);

                        let mut pk = Map::new();
                        pk.insert("modulus".to_string(), Value::String(modulus_hex));
                        if let Some(exp) = exponent {
                            pk.insert("public_exponent".to_string(), Value::Number(exp.into()));
                        }
                        result.insert("public_key".to_string(), Value::Object(pk));
                    }
                }
            }
        } else {
            // EC or other: just hex-encode the key bytes
            result.insert(
                "public_key".to_string(),
                Value::String(bytes_to_hex(key_bytes)),
            );
        }
    }

    result
}

/// Parse a large integer DER value as a u64 (for public exponent, etc.)
pub fn parse_big_int_as_u64(data: &[u8]) -> Option<u64> {
    // Skip leading zero byte if present (sign byte)
    let data = if !data.is_empty() && data[0] == 0 {
        &data[1..]
    } else {
        data
    };
    if data.len() > 8 {
        return None;
    }
    let mut value = 0u64;
    for &b in data {
        value = (value << 8) | b as u64;
    }
    Some(value)
}

/// Map Extended Key Usage OIDs to human-readable names (matching jc).
fn eku_oid_to_name(oid: &str) -> &'static str {
    match oid {
        "1.3.6.1.5.5.7.3.1" => "server_auth",
        "1.3.6.1.5.5.7.3.2" => "client_auth",
        "1.3.6.1.5.5.7.3.3" => "code_signing",
        "1.3.6.1.5.5.7.3.4" => "email_protection",
        "1.3.6.1.5.5.7.3.8" => "time_stamping",
        "1.3.6.1.5.5.7.3.9" => "ocsp_signing",
        _ => "",
    }
}

/// Convert big-endian byte slice to decimal string using base-256 to base-10 conversion.
fn bytes_to_decimal_str(data: &[u8]) -> String {
    // Represent the number as decimal digits (little-endian for easier manipulation)
    let mut digits: Vec<u8> = vec![0]; // starts as 0
    for &byte in data {
        // Multiply digits by 256
        let mut carry: u32 = 0;
        for d in digits.iter_mut() {
            let val = (*d as u32) * 256 + carry;
            *d = (val % 10) as u8;
            carry = val / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
        // Add byte value
        let mut carry: u32 = byte as u32;
        for d in digits.iter_mut() {
            let val = (*d as u32) + carry;
            *d = (val % 10) as u8;
            carry = val / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    // digits is little-endian, reverse to get the string
    digits.iter().rev().map(|&d| (b'0' + d) as char).collect()
}

/// Parse serial number from DER integer bytes.
/// Returns (hex_string, decimal_string_or_hex_if_negative).
pub fn parse_serial_number(data: &[u8]) -> (String, String) {
    // Check if negative (high bit of first byte set after removing sign byte)
    let is_negative = !data.is_empty() && data[0] & 0x80 != 0;

    if is_negative {
        let hex = bytes_to_hex(data);
        let dec_str = format!("(Negative){}", hex);
        (format!("(Negative){}", hex), dec_str)
    } else {
        // Remove leading zero
        let data = if !data.is_empty() && data[0] == 0 {
            &data[1..]
        } else {
            data
        };
        let hex = bytes_to_hex(data);

        // Calculate decimal representation for the decimal string
        // For small numbers (fit in u128)
        if data.len() <= 16 {
            let mut val: u128 = 0;
            for &b in data {
                val = val.wrapping_shl(8) | b as u128;
            }
            (hex, val.to_string())
        } else {
            // Big integer decimal conversion via repeated multiply-add on decimal digits
            let dec_str = bytes_to_decimal_str(data);
            (hex, dec_str)
        }
    }
}

/// Parse X.509 extensions
pub fn parse_extensions(data: &[u8]) -> Vec<Value> {
    let mut result = Vec::new();
    let items = parse_sequence_items(data);

    for item in &items {
        if item.tag != TAG_SEQUENCE {
            continue;
        }
        let ext_items = parse_sequence_items(item.value);
        if ext_items.is_empty() {
            continue;
        }

        let mut ext_obj = Map::new();

        // First item: OID
        if ext_items[0].tag != TAG_OID {
            continue;
        }
        let oid = parse_oid(ext_items[0].value);
        let name = oid_to_name(&oid);
        let ext_id = if name.is_empty() {
            oid.clone()
        } else {
            name.to_string()
        };
        ext_obj.insert("extn_id".to_string(), Value::String(ext_id.clone()));

        // Check for critical boolean
        let (critical, value_idx) = if ext_items.len() > 2 && ext_items[1].tag == TAG_BOOLEAN {
            let is_critical = !ext_items[1].value.is_empty() && ext_items[1].value[0] != 0;
            (is_critical, 2)
        } else {
            (false, 1)
        };
        ext_obj.insert("critical".to_string(), Value::Bool(critical));

        // Extension value is in an OCTET STRING
        let extn_value =
            if value_idx < ext_items.len() && ext_items[value_idx].tag == TAG_OCTETSTRING {
                parse_extension_value(&ext_id, ext_items[value_idx].value)
            } else {
                Value::Null
            };
        ext_obj.insert("extn_value".to_string(), extn_value.clone());

        // Add _iso sibling for invalidity_date
        if ext_id == "invalidity_date" {
            if let Value::Number(n) = &extn_value {
                if let Some(epoch) = n.as_i64() {
                    ext_obj.insert(
                        "extn_value_iso".to_string(),
                        Value::String(epoch_to_iso(epoch)),
                    );
                }
            }
        }

        result.push(Value::Object(ext_obj));
    }

    result
}

/// Parse an extension value based on the extension ID.
fn parse_extension_value(ext_id: &str, data: &[u8]) -> Value {
    match ext_id {
        "basic_constraints" => {
            // SEQUENCE { OPTIONAL BOOLEAN ca, OPTIONAL INTEGER pathLen }
            if let Some((seq_tlv, _)) = parse_tlv(data) {
                if seq_tlv.tag == TAG_SEQUENCE {
                    let items = parse_sequence_items(seq_tlv.value);
                    let mut bc = Map::new();
                    let mut ca = false;
                    let mut path_len = Value::Null;

                    for item in &items {
                        if item.tag == TAG_BOOLEAN {
                            ca = !item.value.is_empty() && item.value[0] != 0;
                        } else if item.tag == TAG_INTEGER {
                            if let Some(n) = parse_big_int_as_u64(item.value) {
                                path_len = Value::Number(n.into());
                            }
                        }
                    }
                    bc.insert("ca".to_string(), Value::Bool(ca));
                    bc.insert("path_len_constraint".to_string(), path_len);
                    return Value::Object(bc);
                }
            }
            Value::Null
        }
        "key_usage" => {
            // BIT STRING with named bits
            if let Some((bs_tlv, _)) = parse_tlv(data) {
                if bs_tlv.tag == TAG_BITSTRING && bs_tlv.value.len() >= 2 {
                    let unused_bits = bs_tlv.value[0] as usize;
                    let bits = &bs_tlv.value[1..];
                    let usage_names = [
                        "digital_signature",
                        "non_repudiation",
                        "key_encipherment",
                        "data_encipherment",
                        "key_agreement",
                        "key_cert_sign",
                        "crl_sign",
                        "encipher_only",
                        "decipher_only",
                    ];
                    let mut usages = Vec::new();
                    let total_bits = bits.len() * 8 - unused_bits;
                    for (i, name) in usage_names.iter().enumerate() {
                        if i >= total_bits {
                            break;
                        }
                        let byte_idx = i / 8;
                        let bit_idx = 7 - (i % 8);
                        if byte_idx < bits.len() && (bits[byte_idx] >> bit_idx) & 1 == 1 {
                            usages.push(name.to_string());
                        }
                    }
                    // Sort alphabetically to match jc's output order
                    usages.sort_unstable();
                    let usages: Vec<Value> = usages.into_iter().map(Value::String).collect();
                    return Value::Array(usages);
                }
            }
            Value::Array(Vec::new())
        }
        "extended_key_usage" => {
            // SEQUENCE OF OID
            if let Some((seq_tlv, _)) = parse_tlv(data) {
                if seq_tlv.tag == TAG_SEQUENCE {
                    let items = parse_sequence_items(seq_tlv.value);
                    let usages: Vec<Value> = items
                        .iter()
                        .filter(|i| i.tag == 0x06)
                        .map(|i| {
                            let oid = parse_oid(i.value);
                            let name = eku_oid_to_name(&oid);
                            if name.is_empty() {
                                Value::String(oid)
                            } else {
                                Value::String(name.to_string())
                            }
                        })
                        .collect();
                    return Value::Array(usages);
                }
            }
            Value::Array(Vec::new())
        }
        "key_identifier" => {
            // OCTET STRING
            if let Some((oct_tlv, _)) = parse_tlv(data) {
                if oct_tlv.tag == TAG_OCTETSTRING {
                    return Value::String(bytes_to_hex(oct_tlv.value));
                }
            }
            Value::String(bytes_to_hex(data))
        }
        "authority_key_identifier" => {
            // SEQUENCE { [0] keyIdentifier OPTIONAL, [1] authorityCertIssuer OPTIONAL, [2] authorityCertSerialNumber OPTIONAL }
            if let Some((seq_tlv, _)) = parse_tlv(data) {
                if seq_tlv.tag == TAG_SEQUENCE {
                    let items = parse_sequence_items(seq_tlv.value);
                    let mut aki = Map::new();
                    aki.insert("key_identifier".to_string(), Value::Null);
                    aki.insert("authority_cert_issuer".to_string(), Value::Null);
                    aki.insert("authority_cert_serial_number".to_string(), Value::Null);

                    for item in &items {
                        let tag_class = item.tag & 0xC0;
                        let tag_num = item.tag & 0x1F;
                        if tag_class == 0x80 || tag_class == 0xA0 {
                            // Context-specific
                            if tag_num == 0 {
                                aki.insert(
                                    "key_identifier".to_string(),
                                    Value::String(bytes_to_hex(item.value)),
                                );
                            }
                        }
                    }
                    return Value::Object(aki);
                }
            }
            Value::Null
        }
        "subject_alt_name" => {
            // SEQUENCE OF GeneralName
            if let Some((seq_tlv, _)) = parse_tlv(data) {
                if seq_tlv.tag == TAG_SEQUENCE {
                    let names: Vec<Value> = parse_sequence_items(seq_tlv.value)
                        .iter()
                        .filter_map(|item| {
                            let tag_num = item.tag & 0x1F;
                            match tag_num {
                                1 => {
                                    // Email: no prefix, use \xNN escapes for invalid UTF-8 bytes
                                    Some(Value::String(bytes_to_python_repr(item.value)))
                                }
                                2 => Some(Value::String(format!(
                                    "dns:{}",
                                    String::from_utf8_lossy(item.value)
                                ))),
                                7 => {
                                    // IP address
                                    if item.value.len() == 4 {
                                        Some(Value::String(format!(
                                            "ip:{}.{}.{}.{}",
                                            item.value[0],
                                            item.value[1],
                                            item.value[2],
                                            item.value[3]
                                        )))
                                    } else {
                                        Some(Value::String(format!(
                                            "ip:{}",
                                            bytes_to_hex(item.value)
                                        )))
                                    }
                                }
                                _ => None,
                            }
                        })
                        .collect();
                    return Value::Array(names);
                }
            }
            Value::Array(Vec::new())
        }
        "crl_reason" => {
            // ENUMERATED - decode to reason name string
            if let Some((enum_tlv, _)) = parse_tlv(data) {
                if enum_tlv.tag == 0x0A && !enum_tlv.value.is_empty() {
                    let reason_code = enum_tlv.value[0];
                    let reason_name = match reason_code {
                        0 => "unspecified",
                        1 => "key_compromise",
                        2 => "ca_compromise",
                        3 => "affiliation_changed",
                        4 => "superseded",
                        5 => "cessation_of_operation",
                        6 => "certificate_hold",
                        8 => "remove_from_crl",
                        9 => "privilege_withdrawn",
                        10 => "aa_compromise",
                        _ => "unspecified",
                    };
                    return Value::String(reason_name.to_string());
                }
            }
            Value::String(bytes_to_hex(data))
        }
        "crl_number" => {
            // INTEGER - decode as integer value
            if let Some((int_tlv, _)) = parse_tlv(data) {
                if int_tlv.tag == TAG_INTEGER {
                    // Strip leading zero sign byte
                    let d = if !int_tlv.value.is_empty() && int_tlv.value[0] == 0 {
                        &int_tlv.value[1..]
                    } else {
                        int_tlv.value
                    };
                    if d.len() <= 8 {
                        let mut n = 0u64;
                        for &b in d {
                            n = (n << 8) | b as u64;
                        }
                        return Value::Number(n.into());
                    }
                }
            }
            Value::String(bytes_to_hex(data))
        }
        "invalidity_date" => {
            // GeneralizedTime - decode as epoch integer + add _iso sibling via separate handling
            if let Some((time_tlv, _)) = parse_tlv(data) {
                if time_tlv.tag == TAG_GENERALIZEDTIME {
                    if let Some(epoch) = parse_time(time_tlv.tag, time_tlv.value) {
                        return Value::Number(epoch.into());
                    }
                }
            }
            Value::String(bytes_to_hex(data))
        }
        "netscape_certificate_type" => {
            // BIT STRING with named bits
            if let Some((bs_tlv, _)) = parse_tlv(data) {
                if bs_tlv.tag == TAG_BITSTRING && bs_tlv.value.len() >= 2 {
                    let unused_bits = bs_tlv.value[0] as usize;
                    let bits = &bs_tlv.value[1..];
                    let cert_type_names = [
                        "ssl_client",
                        "ssl_server",
                        "smime",
                        "object_signing",
                        "reserved",
                        "ssl_ca",
                        "smime_ca",
                        "object_signing_ca",
                    ];
                    let mut types = Vec::new();
                    let total_bits = bits.len() * 8 - unused_bits;
                    for (i, name) in cert_type_names.iter().enumerate() {
                        if i >= total_bits {
                            break;
                        }
                        let byte_idx = i / 8;
                        let bit_idx = 7 - (i % 8);
                        if byte_idx < bits.len() && (bits[byte_idx] >> bit_idx) & 1 == 1 {
                            types.push(Value::String(name.to_string()));
                        }
                    }
                    return Value::Array(types);
                }
            }
            Value::Array(Vec::new())
        }
        _ => {
            // Default: return hex-encoded value
            Value::String(bytes_to_hex(data))
        }
    }
}
