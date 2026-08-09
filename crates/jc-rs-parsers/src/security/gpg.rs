//! Parser for `gpg --with-colons` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct GpgParser;

static INFO: ParserInfo = ParserInfo {
    name: "gpg",
    argument: "--gpg",
    version: "1.0.0",
    description: "Converts `gpg --with-colons` command output to JSON",
    author: "jc-rs contributors",
    author_email: "os@g1sw.com",
    compatible: &[Platform::Linux],
    tags: &[Tag::Command],
    magic_commands: &["gpg --with-colons"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static GPG_PARSER: GpgParser = GpgParser;

inventory::submit! {
    ParserEntry::new(&GPG_PARSER)
}

fn list_get(values: &[&str], index: usize) -> Value {
    if index < values.len() && !values[index].is_empty() {
        Value::String(values[index].to_string())
    } else {
        Value::Null
    }
}

impl Parser for GpgParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let values: Vec<&str> = line.split(':').collect();
            let record_type = if !values.is_empty() { values[0] } else { "" };

            let obj: Map<String, Value> = match record_type {
                "pkd" => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("pkd".to_string()));
                    m.insert("index".to_string(), list_get(&values, 1));
                    m.insert("bits".to_string(), list_get(&values, 2));
                    m.insert("value".to_string(), list_get(&values, 3));
                    m
                }
                "tfs" => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("tfs".to_string()));
                    m.insert("version".to_string(), list_get(&values, 1));
                    m.insert("validity".to_string(), list_get(&values, 2));
                    m.insert("signature_count".to_string(), list_get(&values, 3));
                    m.insert("encryption_count".to_string(), list_get(&values, 4));
                    m.insert("policy".to_string(), list_get(&values, 5));
                    m.insert("signature_first_seen".to_string(), list_get(&values, 6));
                    m.insert(
                        "signature_most_recent_seen".to_string(),
                        list_get(&values, 7),
                    );
                    m.insert("encryption_first_done".to_string(), list_get(&values, 8));
                    m.insert(
                        "encryption_most_recent_done".to_string(),
                        list_get(&values, 9),
                    );
                    m
                }
                "tru" => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("tru".to_string()));
                    m.insert("staleness_reason".to_string(), list_get(&values, 1));
                    m.insert("trust_model".to_string(), list_get(&values, 2));
                    m.insert("trust_db_created".to_string(), list_get(&values, 3));
                    m.insert("trust_db_expires".to_string(), list_get(&values, 4));
                    m.insert("marginally_trusted_users".to_string(), list_get(&values, 5));
                    m.insert("completely_trusted_users".to_string(), list_get(&values, 6));
                    m.insert("cert_chain_max_depth".to_string(), list_get(&values, 7));
                    m
                }
                "skp" => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("skp".to_string()));
                    m.insert("subpacket_number".to_string(), list_get(&values, 1));
                    m.insert("hex_flags".to_string(), list_get(&values, 2));
                    m.insert("subpacket_length".to_string(), list_get(&values, 3));
                    m.insert("subpacket_data".to_string(), list_get(&values, 4));
                    m
                }
                "cfg" => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), Value::String("cfg".to_string()));
                    let subtype = if values.len() > 1 { values[1] } else { "" };
                    match subtype {
                        "version" => {
                            m.insert("version".to_string(), list_get(&values, 2));
                        }
                        "pubkey" => {
                            m.insert("pubkey".to_string(), list_get(&values, 2));
                        }
                        "cipher" => {
                            m.insert("cipher".to_string(), list_get(&values, 2));
                        }
                        "digest" => {
                            m.insert("digest".to_string(), list_get(&values, 2));
                        }
                        "compress" => {
                            m.insert("compress".to_string(), list_get(&values, 2));
                        }
                        "group" => {
                            m.insert("group".to_string(), list_get(&values, 2));
                            m.insert("members".to_string(), list_get(&values, 3));
                        }
                        "curve" => {
                            m.insert("curve_names".to_string(), list_get(&values, 2));
                        }
                        _ => {}
                    }
                    m
                }
                _ => {
                    let mut m = Map::new();
                    m.insert("type".to_string(), list_get(&values, 0));
                    m.insert("validity".to_string(), list_get(&values, 1));
                    m.insert("key_length".to_string(), list_get(&values, 2));
                    m.insert("pub_key_alg".to_string(), list_get(&values, 3));
                    m.insert("key_id".to_string(), list_get(&values, 4));
                    m.insert("creation_date".to_string(), list_get(&values, 5));
                    m.insert("expiration_date".to_string(), list_get(&values, 6));
                    m.insert("certsn_uidhash_trustinfo".to_string(), list_get(&values, 7));
                    m.insert("owner_trust".to_string(), list_get(&values, 8));
                    m.insert("user_id".to_string(), list_get(&values, 9));
                    m.insert("signature_class".to_string(), list_get(&values, 10));
                    m.insert("key_capabilities".to_string(), list_get(&values, 11));
                    m.insert("cert_fingerprint_other".to_string(), list_get(&values, 12));
                    m.insert("flag".to_string(), list_get(&values, 13));
                    m.insert("token_sn".to_string(), list_get(&values, 14));
                    m.insert("hash_alg".to_string(), list_get(&values, 15));
                    m.insert("curve_name".to_string(), list_get(&values, 16));
                    m.insert("compliance_flags".to_string(), list_get(&values, 17));
                    m.insert("last_update_date".to_string(), list_get(&values, 18));
                    m.insert("origin".to_string(), list_get(&values, 19));
                    m.insert("comment".to_string(), list_get(&values, 20));
                    m
                }
            };

            result.push(obj);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpg_basic() {
        let input = "pub:f:1024:17:6C7EE1B8621CC013:899817715:1055898235::m:::scESC:\nfpr:::::::::ECAF7590EB3443B5C7CF3ACB6C7EE1B8621CC013:";
        let parser = GpgParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 2);
            assert_eq!(arr[0].get("type"), Some(&Value::String("pub".to_string())));
            assert_eq!(
                arr[0].get("validity"),
                Some(&Value::String("f".to_string()))
            );
            assert_eq!(
                arr[0].get("key_length"),
                Some(&Value::String("1024".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_gpg_special_types() {
        let input = "pkd:0:1024:B665B1435F4C2FF26ABB:\ntfs:f1:f2:f3:f4:f5:f6:f7:f8:f9:\ntru:o:0:1166697654:1:3:1:5\nskp:f1:f2:f3:f4:\ncfg:version:1.3.5\ncfg:pubkey:1;2;3;16;17\ncfg:curve:ed25519;nistp256";
        let parser = GpgParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 7);
            // pkd
            assert_eq!(arr[0].get("type"), Some(&Value::String("pkd".to_string())));
            assert_eq!(arr[0].get("index"), Some(&Value::String("0".to_string())));
            // tfs
            assert_eq!(arr[1].get("type"), Some(&Value::String("tfs".to_string())));
            // tru
            assert_eq!(arr[2].get("type"), Some(&Value::String("tru".to_string())));
            // cfg version
            assert_eq!(
                arr[4].get("version"),
                Some(&Value::String("1.3.5".to_string()))
            );
            // cfg curve
            assert_eq!(
                arr[6].get("curve_names"),
                Some(&Value::String("ed25519;nistp256".to_string()))
            );
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_gpg_empty() {
        let parser = GpgParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}
