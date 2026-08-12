//! Parser for X.509 Certificate files (PEM and DER).

use crate::security::der::{
    TAG_BITSTRING, TAG_GENERALIZEDTIME, TAG_INTEGER, TAG_OID, TAG_SEQUENCE, TAG_UTCTIME, Tlv,
    bytes_to_hex, decode_pem, parse_algorithm_identifier, parse_extensions, parse_name,
    parse_sequence_items, parse_serial_number, parse_spki, parse_tlv, parse_validity,
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

static X509_CERT_PARSER: X509CertParser = X509CertParser;

inventory::submit! {
    ParserEntry::new(&X509_CERT_PARSER)
}

fn parse_all_tlvs(data: &[u8]) -> Option<Vec<Tlv<'_>>> {
    let mut items = Vec::new();
    let mut remaining = data;
    while !remaining.is_empty() {
        let (item, rest) = parse_tlv(remaining)?;
        items.push(item);
        remaining = rest;
    }
    Some(items)
}

fn is_algorithm_identifier(item: &Tlv<'_>) -> bool {
    if item.tag != TAG_SEQUENCE {
        return false;
    }
    let Some(parts) = parse_all_tlvs(item.value) else {
        return false;
    };
    matches!(parts.len(), 1 | 2) && parts[0].tag == TAG_OID && !parts[0].value.is_empty()
}

fn is_validity(item: &Tlv<'_>) -> bool {
    if item.tag != TAG_SEQUENCE {
        return false;
    }
    let Some(times) = parse_all_tlvs(item.value) else {
        return false;
    };
    times.len() == 2
        && times.iter().all(|time| {
            matches!(time.tag, TAG_UTCTIME | TAG_GENERALIZEDTIME) && !time.value.is_empty()
        })
}

fn is_subject_public_key_info(item: &Tlv<'_>) -> bool {
    if item.tag != TAG_SEQUENCE {
        return false;
    }
    let Some(parts) = parse_all_tlvs(item.value) else {
        return false;
    };
    parts.len() == 2
        && is_algorithm_identifier(&parts[0])
        && parts[1].tag == TAG_BITSTRING
        && parts[1].value.len() > 1
        && parts[1].value[0] <= 7
}

fn has_certificate_structure(der: &[u8]) -> bool {
    let Some((certificate, trailing)) = parse_tlv(der) else {
        return false;
    };
    if certificate.tag != TAG_SEQUENCE || !trailing.is_empty() {
        return false;
    }

    let Some(parts) = parse_all_tlvs(certificate.value) else {
        return false;
    };
    if parts.len() != 3
        || parts[0].tag != TAG_SEQUENCE
        || !is_algorithm_identifier(&parts[1])
        || parts[2].tag != TAG_BITSTRING
        || parts[2].value.len() <= 1
        || parts[2].value[0] != 0
    {
        return false;
    }

    let Some(tbs) = parse_all_tlvs(parts[0].value) else {
        return false;
    };
    let mut idx = 0;
    if tbs.first().is_some_and(|item| item.tag == 0xA0) {
        let Some(version) = parse_all_tlvs(tbs[0].value) else {
            return false;
        };
        if version.len() != 1
            || version[0].tag != TAG_INTEGER
            || version[0].value.len() != 1
            || version[0].value[0] > 2
        {
            return false;
        }
        idx += 1;
    }

    if tbs.len() < idx + 6
        || tbs[idx].tag != TAG_INTEGER
        || tbs[idx].value.is_empty()
        || !is_algorithm_identifier(&tbs[idx + 1])
        || tbs[idx + 2].tag != TAG_SEQUENCE
        || !is_validity(&tbs[idx + 3])
        || tbs[idx + 4].tag != TAG_SEQUENCE
        || !is_subject_public_key_info(&tbs[idx + 5])
    {
        return false;
    }
    idx += 6;

    // TBSCertificate permits these optional fields, in this order, after SPKI.
    let mut last_optional = 0;
    for item in &tbs[idx..] {
        let rank = match item.tag {
            0x81 | 0xA1 => 1, // issuerUniqueID
            0x82 | 0xA2 => 2, // subjectUniqueID
            0xA3 => {
                let Some(extension_wrapper) = parse_all_tlvs(item.value) else {
                    return false;
                };
                if extension_wrapper.len() != 1 || extension_wrapper[0].tag != TAG_SEQUENCE {
                    return false;
                }
                3
            }
            _ => return false,
        };
        if rank <= last_optional {
            return false;
        }
        last_optional = rank;
    }

    true
}

fn parse_certificate(der: &[u8]) -> Option<Map<String, Value>> {
    if !has_certificate_structure(der) {
        return None;
    }

    // Certificate ::= SEQUENCE { tbsCertificate, signatureAlgorithm, signature }
    let (cert_tlv, _) = parse_tlv(der)?;

    let items = parse_sequence_items(cert_tlv.value);

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

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/../../tests/fixtures/generic/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

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

    #[test]
    fn accepts_certificate_der_fixtures() {
        for name in [
            "x509-ca-cert.der",
            "x509-cert-bad-email2.der",
            "x509-string-serialnumber.der",
        ] {
            assert!(
                parse_certificate(&fixture(name)).is_some(),
                "rejected {name}"
            );
        }
    }

    #[test]
    fn rejects_other_x509_der_structures() {
        for name in ["x509-csr.der", "x509-crl.der"] {
            assert!(
                parse_certificate(&fixture(name)).is_none(),
                "accepted {name}"
            );
        }
    }
}
