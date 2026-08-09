//! Parser for X.509 Certificate Revocation List files (PEM and DER).

use crate::security::der::{
    TAG_BITSTRING, TAG_GENERALIZEDTIME, TAG_INTEGER, TAG_SEQUENCE, TAG_UTCTIME, bytes_to_hex,
    decode_pem, epoch_to_iso, parse_algorithm_identifier, parse_extensions, parse_name,
    parse_sequence_items, parse_serial_number, parse_time, parse_tlv,
};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct X509CrlParser;

static INFO: ParserInfo = ParserInfo {
    name: "x509_crl",
    argument: "--x509-crl",
    version: "1.0.0",
    description: "Converts X.509 CRL PEM and DER files to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[
        Platform::Linux,
        Platform::Darwin,
        Platform::FreeBSD,
        Platform::Universal,
    ],
    tags: &[Tag::File, Tag::String],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static X509_CRL_PARSER: X509CrlParser = X509CrlParser;

inventory::submit! {
    ParserEntry::new(&X509_CRL_PARSER)
}

fn parse_crl(der: &[u8]) -> Option<Map<String, Value>> {
    // CertificateList ::= SEQUENCE { tbsCertList, signatureAlgorithm, signature }
    let (crl_tlv, _) = parse_tlv(der)?;
    if crl_tlv.tag != TAG_SEQUENCE {
        return None;
    }

    let items = parse_sequence_items(crl_tlv.value);
    if items.len() < 3 {
        return None;
    }

    let tbs = parse_tbs_cert_list(items[0].value)?;
    let sig_algo = parse_algorithm_identifier(items[1].value);
    let sig_value = if items[2].tag == TAG_BITSTRING && items[2].value.len() > 1 {
        bytes_to_hex(&items[2].value[1..])
    } else {
        bytes_to_hex(items[2].value)
    };

    let mut crl = Map::new();
    crl.insert("tbs_cert_list".to_string(), Value::Object(tbs));
    crl.insert("signature_algorithm".to_string(), Value::Object(sig_algo));
    crl.insert("signature".to_string(), Value::String(sig_value));

    Some(crl)
}

fn parse_tbs_cert_list(data: &[u8]) -> Option<Map<String, Value>> {
    let mut tbs = Map::new();
    let items = parse_sequence_items(data);
    let mut idx = 0;

    // Version (optional)
    if idx < items.len()
        && items[idx].tag == TAG_INTEGER
        && items[idx].value.len() == 1
        && items[idx].value[0] <= 1
    {
        let v = items[idx].value[0];
        let version = match v {
            0 => "v1",
            1 => "v2",
            _ => "v1",
        };
        tbs.insert("version".to_string(), Value::String(version.to_string()));
        idx += 1;
    } else {
        tbs.insert("version".to_string(), Value::String("v1".to_string()));
    }

    // Signature algorithm
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let sig_algo = parse_algorithm_identifier(items[idx].value);
        tbs.insert("signature".to_string(), Value::Object(sig_algo));
        idx += 1;
    }

    // Issuer
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let issuer = parse_name(items[idx].value);
        tbs.insert("issuer".to_string(), Value::Object(issuer));
        idx += 1;
    }

    // thisUpdate
    if idx < items.len() && (items[idx].tag == TAG_UTCTIME || items[idx].tag == TAG_GENERALIZEDTIME)
    {
        if let Some(epoch) = parse_time(items[idx].tag, items[idx].value) {
            tbs.insert("this_update".to_string(), Value::Number(epoch.into()));
            tbs.insert(
                "this_update_iso".to_string(),
                Value::String(epoch_to_iso(epoch)),
            );
        }
        idx += 1;
    }

    // nextUpdate (optional)
    if idx < items.len() && (items[idx].tag == TAG_UTCTIME || items[idx].tag == TAG_GENERALIZEDTIME)
    {
        if let Some(epoch) = parse_time(items[idx].tag, items[idx].value) {
            tbs.insert("next_update".to_string(), Value::Number(epoch.into()));
            tbs.insert(
                "next_update_iso".to_string(),
                Value::String(epoch_to_iso(epoch)),
            );
        }
        idx += 1;
    }

    // revokedCertificates (optional SEQUENCE OF)
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let revoked = parse_revoked_certificates(items[idx].value);
        tbs.insert("revoked_certificates".to_string(), Value::Array(revoked));
        idx += 1;
    } else {
        tbs.insert("revoked_certificates".to_string(), Value::Array(Vec::new()));
    }

    // crlExtensions [0] (optional)
    while idx < items.len() {
        if items[idx].tag == 0xA0 {
            let ext_items = parse_sequence_items(items[idx].value);
            if !ext_items.is_empty() {
                let extensions = parse_extensions(ext_items[0].value);
                tbs.insert("crl_extensions".to_string(), Value::Array(extensions));
            }
        }
        idx += 1;
    }

    if !tbs.contains_key("crl_extensions") {
        tbs.insert("crl_extensions".to_string(), Value::Array(Vec::new()));
    }

    Some(tbs)
}

fn parse_revoked_certificates(data: &[u8]) -> Vec<Value> {
    let mut result = Vec::new();
    let items = parse_sequence_items(data);

    for item in &items {
        if item.tag != TAG_SEQUENCE {
            continue;
        }
        let cert_items = parse_sequence_items(item.value);
        if cert_items.is_empty() {
            continue;
        }

        let mut cert = Map::new();

        let mut idx = 0;

        // userCertificate: INTEGER
        if idx < cert_items.len() && cert_items[idx].tag == TAG_INTEGER {
            let (hex, dec_str) = parse_serial_number(cert_items[idx].value);
            // Try to parse as integer
            if let Ok(n) = dec_str.parse::<i64>() {
                cert.insert("user_certificate".to_string(), Value::Number(n.into()));
            } else {
                cert.insert("user_certificate".to_string(), Value::String(hex));
            }
            idx += 1;
        }

        // revocationDate: Time
        if idx < cert_items.len()
            && (cert_items[idx].tag == TAG_UTCTIME || cert_items[idx].tag == TAG_GENERALIZEDTIME)
        {
            if let Some(epoch) = parse_time(cert_items[idx].tag, cert_items[idx].value) {
                cert.insert("revocation_date".to_string(), Value::Number(epoch.into()));
                cert.insert(
                    "revocation_date_iso".to_string(),
                    Value::String(epoch_to_iso(epoch)),
                );
            }
            idx += 1;
        }

        // crlEntryExtensions (optional)
        if idx < cert_items.len() && cert_items[idx].tag == TAG_SEQUENCE {
            let extensions = parse_extensions(cert_items[idx].value);
            if !extensions.is_empty() {
                cert.insert("crl_entry_extensions".to_string(), Value::Array(extensions));
            }
        }

        result.push(Value::Object(cert));
    }

    result
}

impl Parser for X509CrlParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Object(Map::new()));
        }

        // Try PEM first
        let pem_crls = decode_pem(input, "X509 CRL");

        if !pem_crls.is_empty()
            && let Some(crl) = parse_crl(&pem_crls[0])
        {
            return Ok(ParseOutput::Object(crl));
        }

        // Try as raw DER
        let der = input.as_bytes();
        if let Some(crl) = parse_crl(der) {
            return Ok(ParseOutput::Object(crl));
        }

        Ok(ParseOutput::Object(Map::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x509_crl_empty() {
        let parser = X509CrlParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Object(obj) = result {
            assert!(obj.is_empty());
        } else {
            panic!("Expected Object");
        }
    }
}
