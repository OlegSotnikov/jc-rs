//! Parser for X.509 Certificate files (PEM and DER).

use crate::security::der::{
    TAG_BITSTRING, TAG_INTEGER, TAG_SEQUENCE, bytes_to_hex, decode_pem, parse_algorithm_identifier,
    parse_extensions, parse_name, parse_sequence_items, parse_serial_number, parse_spki, parse_tlv,
    parse_validity,
};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct X509CertParser;

static INFO: ParserInfo = ParserInfo {
    name: "x509_cert",
    argument: "--x509-cert",
    version: "1.4.0",
    description: "Converts X.509 PEM and DER certificate files to JSON",
    author: "jc-rs contributors",
    author_email: "",
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

static X509_CERT_PARSER: X509CertParser = X509CertParser;

inventory::submit! {
    ParserEntry::new(&X509_CERT_PARSER)
}

fn parse_certificate(der: &[u8]) -> Option<Map<String, Value>> {
    // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signature }
    let (cert_tlv, _) = parse_tlv(der)?;
    if cert_tlv.tag != TAG_SEQUENCE {
        return None;
    }

    let items = parse_sequence_items(cert_tlv.value);
    if items.len() < 3 {
        return None;
    }

    // TBSCertificate
    let tbs = parse_tbs_certificate(items[0].value)?;

    // signatureAlgorithm
    let sig_algo = parse_algorithm_identifier(items[1].value);

    // signature value (BIT STRING)
    let sig_value = if items[2].tag == TAG_BITSTRING && items[2].value.len() > 1 {
        bytes_to_hex(&items[2].value[1..]) // skip unused bits byte
    } else {
        bytes_to_hex(items[2].value)
    };

    let mut cert = Map::new();
    cert.insert("tbs_certificate".to_string(), Value::Object(tbs));
    cert.insert("signature_algorithm".to_string(), Value::Object(sig_algo));
    cert.insert("signature_value".to_string(), Value::String(sig_value));

    Some(cert)
}

fn parse_tbs_certificate(data: &[u8]) -> Option<Map<String, Value>> {
    let mut tbs = Map::new();
    let items = parse_sequence_items(data);

    let mut idx = 0;

    // Version [0] EXPLICIT INTEGER (optional, default v1)
    let version = if idx < items.len() && items[idx].tag == 0xA0 {
        let ver_items = parse_sequence_items(items[idx].value);
        let v = if !ver_items.is_empty() && ver_items[0].tag == TAG_INTEGER {
            if ver_items[0].value.is_empty() {
                0u8
            } else {
                ver_items[0].value[0]
            }
        } else {
            0u8
        };
        idx += 1;
        match v {
            0 => "v1",
            1 => "v2",
            2 => "v3",
            _ => "v1",
        }
    } else {
        "v1"
    };
    tbs.insert("version".to_string(), Value::String(version.to_string()));

    // Serial number
    if idx < items.len() && items[idx].tag == TAG_INTEGER {
        let (hex, dec_str) = parse_serial_number(items[idx].value);
        tbs.insert("serial_number".to_string(), Value::String(hex));
        tbs.insert("serial_number_str".to_string(), Value::String(dec_str));
        idx += 1;
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

    // Validity
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let validity = parse_validity(items[idx].value);
        tbs.insert("validity".to_string(), Value::Object(validity));
        idx += 1;
    }

    // Subject
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let subject = parse_name(items[idx].value);
        tbs.insert("subject".to_string(), Value::Object(subject));
        idx += 1;
    }

    // SubjectPublicKeyInfo
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let spki = parse_spki(items[idx].value);
        tbs.insert("subject_public_key_info".to_string(), Value::Object(spki));
        idx += 1;
    }

    // Optional: issuerUniqueID [1], subjectUniqueID [2], extensions [3]
    tbs.insert("issuer_unique_id".to_string(), Value::Null);
    tbs.insert("subject_unique_id".to_string(), Value::Null);

    while idx < items.len() {
        let tag = items[idx].tag;
        if tag == 0xA1 {
            // issuerUniqueID
            let hex = bytes_to_hex(items[idx].value);
            tbs.insert("issuer_unique_id".to_string(), Value::String(hex));
        } else if tag == 0xA2 {
            // subjectUniqueID
            let hex = bytes_to_hex(items[idx].value);
            tbs.insert("subject_unique_id".to_string(), Value::String(hex));
        } else if tag == 0xA3 {
            // extensions
            let ext_items = parse_sequence_items(items[idx].value);
            if !ext_items.is_empty() {
                let extensions = parse_extensions(ext_items[0].value);
                tbs.insert("extensions".to_string(), Value::Array(extensions));
            }
        }
        idx += 1;
    }

    if !tbs.contains_key("extensions") {
        tbs.insert("extensions".to_string(), Value::Array(Vec::new()));
    }

    Some(tbs)
}

impl Parser for X509CertParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results = Vec::new();

        // Try PEM first
        let pem_certs = decode_pem(input, "CERTIFICATE");

        if !pem_certs.is_empty() {
            for der in pem_certs {
                if let Some(cert) = parse_certificate(&der) {
                    results.push(cert);
                }
            }
        } else {
            // Try as raw DER binary input
            // Convert input to bytes - it might be binary data passed as string
            let der = input.as_bytes();
            if let Some(cert) = parse_certificate(der) {
                results.push(cert);
            }
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x509_cert_empty() {
        let parser = X509CertParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_x509_cert_pem() {
        // Simple PEM certificate
        let input = r#"-----BEGIN CERTIFICATE-----
MIICvDCCAaQCAQAwdzELMAkGA1UEBhMCVVMxDTALBgNVBAgMBFV0YWgxDzANBgNV
BAcMBkxpbmRvbjEWMBQGA1UECgwNRGlnaUNlcnQgSW5jLjERMA8GA1UECwwIRGln
-----END CERTIFICATE-----"#;
        let parser = X509CertParser;
        // This will fail to parse (truncated cert), but should not crash
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }
}
