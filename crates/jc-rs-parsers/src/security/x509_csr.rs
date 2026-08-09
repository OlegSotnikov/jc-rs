//! Parser for X.509 Certificate Signing Request files (PEM and DER).

use crate::security::der::{
    TAG_BITSTRING, TAG_BMPSTRING, TAG_IA5STRING, TAG_SEQUENCE, TAG_UTF8STRING, bytes_to_hex,
    decode_pem, parse_algorithm_identifier, parse_extensions, parse_name, parse_sequence_items,
    parse_spki, parse_tlv,
};
use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct X509CsrParser;

static INFO: ParserInfo = ParserInfo {
    name: "x509_csr",
    argument: "--x509-csr",
    version: "1.0.0",
    description: "Converts X.509 CSR PEM and DER files to JSON",
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

static X509_CSR_PARSER: X509CsrParser = X509CsrParser;

inventory::submit! {
    ParserEntry::new(&X509_CSR_PARSER)
}

fn parse_csr(der: &[u8]) -> Option<Map<String, Value>> {
    // CertificationRequest ::= SEQUENCE {
    //   certificationRequestInfo,
    //   signatureAlgorithm,
    //   signature
    // }
    let (csr_tlv, _) = parse_tlv(der)?;
    if csr_tlv.tag != TAG_SEQUENCE {
        return None;
    }

    let items = parse_sequence_items(csr_tlv.value);
    if items.len() < 3 {
        return None;
    }

    let cri = parse_certification_request_info(items[0].value)?;
    let sig_algo = parse_algorithm_identifier(items[1].value);
    let sig_value = if items[2].tag == TAG_BITSTRING && items[2].value.len() > 1 {
        bytes_to_hex(&items[2].value[1..])
    } else {
        bytes_to_hex(items[2].value)
    };

    let mut csr = Map::new();
    csr.insert("certification_request_info".to_string(), Value::Object(cri));
    csr.insert("signature_algorithm".to_string(), Value::Object(sig_algo));
    csr.insert("signature".to_string(), Value::String(sig_value));

    Some(csr)
}

fn parse_certification_request_info(data: &[u8]) -> Option<Map<String, Value>> {
    let mut cri = Map::new();
    let items = parse_sequence_items(data);
    let mut idx = 0;

    // Version: INTEGER (must be 0 = v1)
    if idx < items.len() {
        cri.insert("version".to_string(), Value::String("v1".to_string()));
        idx += 1;
    }

    // Subject
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let subject = parse_name(items[idx].value);
        cri.insert("subject".to_string(), Value::Object(subject));
        idx += 1;
    }

    // SubjectPKInfo
    if idx < items.len() && items[idx].tag == TAG_SEQUENCE {
        let spki = parse_spki(items[idx].value);
        cri.insert("subject_pk_info".to_string(), Value::Object(spki));
        idx += 1;
    }

    // Attributes [0] IMPLICIT SET OF Attribute (optional)
    cri.insert("attributes".to_string(), Value::Array(Vec::new()));
    if idx < items.len() {
        // Tag 0xA0 = [0] IMPLICIT
        if items[idx].tag == 0xA0 {
            let attrs = parse_csr_attributes(items[idx].value);
            cri.insert("attributes".to_string(), Value::Array(attrs));
        }
    }

    Some(cri)
}

fn parse_csr_attributes(data: &[u8]) -> Vec<Value> {
    use crate::security::der::parse_string;
    const TAG_PRINTABLESTRING: u8 = 0x13;
    const TAG_INTEGER: u8 = 0x02;

    let mut result = Vec::new();
    let items = parse_sequence_items(data);

    for item in &items {
        if item.tag != TAG_SEQUENCE {
            continue;
        }
        let attr_items = parse_sequence_items(item.value);
        if attr_items.len() < 2 {
            continue;
        }

        // First: OID (attribute type)
        if attr_items[0].tag != 0x06 {
            continue;
        }

        let oid = crate::security::der::parse_oid(attr_items[0].value);
        let name = crate::security::der::oid_to_name(&oid);
        let type_str = if name.is_empty() {
            oid.clone()
        } else {
            name.to_string()
        };

        // Second: SET of values
        let mut attr_obj = Map::new();
        attr_obj.insert("type".to_string(), Value::String(type_str.clone()));

        if attr_items[1].tag == 0x31 {
            // SET
            let value_items = parse_sequence_items(attr_items[1].value);
            let values: Vec<Value> = value_items
                .iter()
                .map(|v| {
                    match type_str.as_str() {
                        "extension_request" if v.tag == TAG_SEQUENCE => {
                            Value::Array(parse_extensions(v.value))
                        }
                        "microsoft_os_version" => {
                            // IA5String or UTF8String → plain string
                            Value::String(parse_string(v.tag, v.value))
                        }
                        "microsoft_request_client_info" if v.tag == TAG_SEQUENCE => {
                            // SEQUENCE { INTEGER clientid, UTF8String machine, UTF8String user, UTF8String process }
                            let seq = parse_sequence_items(v.value);
                            let mut obj = Map::new();
                            let clientid = if !seq.is_empty() && seq[0].tag == TAG_INTEGER {
                                let mut n: i64 = 0;
                                for &b in seq[0].value {
                                    n = (n << 8) | b as i64;
                                }
                                Value::Number(n.into())
                            } else {
                                Value::Null
                            };
                            obj.insert("clientid".to_string(), clientid);
                            obj.insert(
                                "machinename".to_string(),
                                if seq.len() > 1 {
                                    Value::String(parse_string(seq[1].tag, seq[1].value))
                                } else {
                                    Value::Null
                                },
                            );
                            obj.insert(
                                "username".to_string(),
                                if seq.len() > 2 {
                                    Value::String(parse_string(seq[2].tag, seq[2].value))
                                } else {
                                    Value::Null
                                },
                            );
                            obj.insert(
                                "processname".to_string(),
                                if seq.len() > 3 {
                                    Value::String(parse_string(seq[3].tag, seq[3].value))
                                } else {
                                    Value::Null
                                },
                            );
                            Value::Object(obj)
                        }
                        "microsoft_enrollment_csp_provider" if v.tag == TAG_SEQUENCE => {
                            // SEQUENCE { INTEGER keyspec, BMPString cspname, BIT STRING signature }
                            let seq = parse_sequence_items(v.value);
                            let mut obj = Map::new();
                            let keyspec = if !seq.is_empty() && seq[0].tag == TAG_INTEGER {
                                let mut n: i64 = 0;
                                for &b in seq[0].value {
                                    n = (n << 8) | b as i64;
                                }
                                Value::Number(n.into())
                            } else {
                                Value::Null
                            };
                            obj.insert("keyspec".to_string(), keyspec);
                            obj.insert(
                                "cspname".to_string(),
                                if seq.len() > 1 {
                                    Value::String(parse_string(seq[1].tag, seq[1].value))
                                } else {
                                    Value::Null
                                },
                            );
                            // BIT STRING: if empty or just a 0x00 padding byte, signature = []
                            let sig = if seq.len() > 2 && seq[2].tag == TAG_BITSTRING {
                                // BIT STRING content starts with unused-bits count byte
                                let bs = seq[2].value;
                                let payload = if bs.len() > 1 { &bs[1..] } else { &bs[..0] };
                                if payload.is_empty() {
                                    Value::Array(Vec::new())
                                } else {
                                    Value::Array(
                                        payload
                                            .iter()
                                            .map(|&b| Value::Number((b as i64).into()))
                                            .collect(),
                                    )
                                }
                            } else {
                                Value::Array(Vec::new())
                            };
                            obj.insert("signature".to_string(), sig);
                            Value::Object(obj)
                        }
                        _ => {
                            // Generic: decode string if possible, otherwise hex
                            match v.tag {
                                TAG_UTF8STRING | TAG_PRINTABLESTRING | TAG_IA5STRING
                                | TAG_BMPSTRING => Value::String(parse_string(v.tag, v.value)),
                                _ => Value::String(bytes_to_hex(v.value)),
                            }
                        }
                    }
                })
                .collect();
            attr_obj.insert("values".to_string(), Value::Array(values));
        }

        result.push(Value::Object(attr_obj));
    }

    result
}

impl Parser for X509CsrParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut results = Vec::new();

        // Try PEM first - handle both standard and Windows "NEW CERTIFICATE REQUEST" headers
        let mut pem_csrs = decode_pem(input, "CERTIFICATE REQUEST");
        if pem_csrs.is_empty() {
            pem_csrs = decode_pem(input, "NEW CERTIFICATE REQUEST");
        }

        if !pem_csrs.is_empty() {
            for der in pem_csrs {
                if let Some(csr) = parse_csr(&der) {
                    results.push(csr);
                }
            }
        } else {
            // Try raw DER
            let der = input.as_bytes();
            if let Some(csr) = parse_csr(der) {
                results.push(csr);
            }
        }

        Ok(ParseOutput::Array(results))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x509_csr_empty() {
        let parser = X509CsrParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}
