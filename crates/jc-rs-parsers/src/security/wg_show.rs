//! Parser for `wg show all dump` command output.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};

pub struct WgShowParser;

static INFO: ParserInfo = ParserInfo {
    name: "wg_show",
    argument: "--wg-show",
    version: "1.0.0",
    description: "Converts `wg show all dump` command output to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux, Platform::Darwin, Platform::FreeBSD],
    tags: &[Tag::Command],
    magic_commands: &["wg show"],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static WG_SHOW_PARSER: WgShowParser = WgShowParser;

inventory::submit! {
    ParserEntry::new(&WG_SHOW_PARSER)
}

/// Convert ordered peer entries to the processed peers array (insertion order preserved).
fn build_peers_array(peers: impl Iterator<Item = (String, Value)>) -> Vec<Value> {
    peers
        .map(|(peer_key, peer_data)| {
            let mut peer_obj = Map::new();
            peer_obj.insert("public_key".to_string(), Value::String(peer_key));
            if let Value::Object(pd) = peer_data {
                peer_obj.insert(
                    "preshared_key".to_string(),
                    pd.get("preshared_key").cloned().unwrap_or(Value::Null),
                );
                peer_obj.insert(
                    "endpoint".to_string(),
                    pd.get("endpoint").cloned().unwrap_or(Value::Null),
                );
                peer_obj.insert(
                    "latest_handshake".to_string(),
                    pd.get("latest_handshake")
                        .cloned()
                        .unwrap_or(Value::Number(0.into())),
                );
                peer_obj.insert(
                    "transfer_rx".to_string(),
                    pd.get("transfer_rx")
                        .cloned()
                        .unwrap_or(Value::Number(0.into())),
                );
                peer_obj.insert(
                    "transfer_sx".to_string(),
                    pd.get("transfer_sx")
                        .cloned()
                        .unwrap_or(Value::Number(0.into())),
                );
                peer_obj.insert(
                    "persistent_keepalive".to_string(),
                    pd.get("persistent_keepalive")
                        .cloned()
                        .unwrap_or(Value::Number((-1i64).into())),
                );
                peer_obj.insert(
                    "allowed_ips".to_string(),
                    pd.get("allowed_ips")
                        .cloned()
                        .unwrap_or(Value::Array(Vec::new())),
                );
            }
            Value::Object(peer_obj)
        })
        .collect()
}

impl Parser for WgShowParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(Vec::new()));
        }

        let mut result: Vec<Map<String, Value>> = Vec::new();
        let mut current_device: Option<String> = None;
        let mut device_data: Map<String, Value> = Map::new();
        // Vec preserves insertion order (Map uses hash ordering)
        let mut peers: Vec<(String, Value)> = Vec::new();

        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();

            if fields.len() == 5 {
                // Device line: device private_key public_key listen_port fwmark
                let (device, private_key, public_key, listen_port, fwmark) =
                    (fields[0], fields[1], fields[2], fields[3], fields[4]);

                if let Some(dev) = current_device.take() {
                    let mut d = device_data.clone();
                    d.insert("device".to_string(), Value::String(dev));
                    let peers_arr: Vec<Value> = build_peers_array(peers.drain(..));
                    d.insert("peers".to_string(), Value::Array(peers_arr));
                    result.push(d);
                    device_data = Map::new();
                    peers = Vec::new();
                }

                current_device = Some(device.to_string());
                device_data.insert(
                    "private_key".to_string(),
                    if private_key == "(none)" {
                        Value::Null
                    } else {
                        Value::String(private_key.to_string())
                    },
                );
                device_data.insert(
                    "public_key".to_string(),
                    if public_key == "(none)" {
                        Value::Null
                    } else {
                        Value::String(public_key.to_string())
                    },
                );
                device_data.insert(
                    "listen_port".to_string(),
                    if listen_port == "0" {
                        Value::Null
                    } else if let Ok(n) = listen_port.parse::<i64>() {
                        Value::Number(n.into())
                    } else {
                        Value::Null
                    },
                );
                device_data.insert(
                    "fwmark".to_string(),
                    if fwmark == "off" {
                        Value::Null
                    } else if let Ok(n) = fwmark.parse::<i64>() {
                        Value::Number(n.into())
                    } else {
                        Value::Null
                    },
                );
            } else if fields.len() == 9 {
                // Peer line: interface public_key preshared_key endpoint allowed_ips
                //            latest_handshake transfer_rx transfer_tx persistent_keepalive
                let (
                    _interface,
                    public_key,
                    preshared_key,
                    endpoint,
                    allowed_ips,
                    latest_handshake,
                    transfer_rx,
                    transfer_tx,
                    persistent_keepalive,
                ) = (
                    fields[0], fields[1], fields[2], fields[3], fields[4], fields[5], fields[6],
                    fields[7], fields[8],
                );

                let mut peer: Map<String, Value> = Map::new();
                peer.insert(
                    "preshared_key".to_string(),
                    if preshared_key == "(none)" {
                        Value::Null
                    } else {
                        Value::String(preshared_key.to_string())
                    },
                );
                peer.insert(
                    "endpoint".to_string(),
                    if endpoint == "(none)" {
                        Value::Null
                    } else {
                        Value::String(endpoint.to_string())
                    },
                );
                if let Ok(n) = latest_handshake.parse::<i64>() {
                    peer.insert("latest_handshake".to_string(), Value::Number(n.into()));
                }
                if let Ok(n) = transfer_rx.parse::<i64>() {
                    peer.insert("transfer_rx".to_string(), Value::Number(n.into()));
                }
                if let Ok(n) = transfer_tx.parse::<i64>() {
                    peer.insert("transfer_sx".to_string(), Value::Number(n.into()));
                }
                peer.insert(
                    "persistent_keepalive".to_string(),
                    if persistent_keepalive == "off" {
                        Value::Number((-1i64).into())
                    } else if let Ok(n) = persistent_keepalive.parse::<i64>() {
                        Value::Number(n.into())
                    } else {
                        Value::Number((-1i64).into())
                    },
                );

                let ips: Vec<Value> = if allowed_ips == "(none)" {
                    Vec::new()
                } else {
                    allowed_ips
                        .split(',')
                        .map(|s| Value::String(s.to_string()))
                        .collect()
                };
                peer.insert("allowed_ips".to_string(), Value::Array(ips));

                peers.push((public_key.to_string(), Value::Object(peer)));
            }
        }

        // Save last device
        if let Some(dev) = current_device.take() {
            let mut d = device_data.clone();
            d.insert("device".to_string(), Value::String(dev));
            let peers_arr: Vec<Value> = build_peers_array(peers.drain(..));
            d.insert("peers".to_string(), Value::Array(peers_arr));
            result.push(d);
        }

        Ok(ParseOutput::Array(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wg_show_basic() {
        let input = "wg0\taEbVdvHSEp3oofHDNVCsUoaRSxk1Og8/pTLof5yF+1M=\tOIxbQszw1chdO5uigAxpsl4fc/h04yMYafl72gUbakM=\t51820\toff\nwg0\tsQFGAhSdx0aC7DmTFojzBOW8Ccjv1XV5+N9FnkZu5zc=\t(none)\t79.134.136.199:40036\t10.10.0.2/32\t1728809756\t1378724\t406524\toff";
        let parser = WgShowParser;
        let result = parser.parse(input, false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 1);
            assert_eq!(
                arr[0].get("device"),
                Some(&Value::String("wg0".to_string()))
            );
            assert_eq!(
                arr[0].get("listen_port"),
                Some(&Value::Number(51820i64.into()))
            );
            assert_eq!(arr[0].get("fwmark"), Some(&Value::Null));
        } else {
            panic!("Expected Array");
        }
    }

    #[test]
    fn test_wg_show_empty() {
        let parser = WgShowParser;
        let result = parser.parse("", false).unwrap();
        if let ParseOutput::Array(arr) = result {
            assert_eq!(arr.len(), 0);
        } else {
            panic!("Expected Array");
        }
    }
}
