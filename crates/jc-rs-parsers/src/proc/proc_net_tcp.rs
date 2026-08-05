//! Parser for `/proc/net/tcp` and `/proc/net/tcp6`

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::ParserEntry;
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use serde_json::{Map, Value};
use std::net::{Ipv4Addr, Ipv6Addr};

pub struct ProcNetTcpParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc_net_tcp",
    argument: "--proc-net-tcp",
    version: "1.0.0",
    description: "Converts `/proc/net/tcp` and `/proc/net/tcp6` files to JSON",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &["/proc/net/tcp", "/proc/net/tcp6"],
    streaming: false,
    hidden: true,
    deprecated: false,
};

static INSTANCE: ProcNetTcpParser = ProcNetTcpParser;
inventory::submit! { ParserEntry::new(&INSTANCE) }

/// Convert a hex address string to a human-readable IP string.
/// 8-char hex → IPv4 (little-endian u32)
/// 32-char hex → IPv6 (4 little-endian u32 groups)
fn hex_to_ip(hexaddr: &str) -> String {
    match hexaddr.len() {
        8 => {
            let n = u32::from_str_radix(hexaddr, 16).unwrap_or(0);
            // Treat as little-endian: swap bytes to get network-order
            let swapped = n.swap_bytes();
            let addr = Ipv4Addr::from(swapped);
            addr.to_string()
        }
        32 => {
            // 4 groups of 8 hex chars, each is a little-endian u32 that needs byte-reversal
            let mut bytes = [0u8; 16];
            for group in 0..4 {
                let chunk = &hexaddr[group * 8..(group + 1) * 8];
                let n = u32::from_str_radix(chunk, 16).unwrap_or(0);
                // byte-reverse each 4-byte group
                let reversed = n.swap_bytes();
                let b = reversed.to_be_bytes();
                bytes[group * 4..(group + 1) * 4].copy_from_slice(&b);
            }
            let addr = Ipv6Addr::from(bytes);
            addr.to_string()
        }
        _ => hexaddr.to_string(),
    }
}

/// Compute opposite-endian IPv6 address: treat raw 32-char hex directly as
/// 8 groups of 4 hex chars (no byte swap), then compress.
fn hex_to_ip_opposite_endian(hexaddr: &str) -> String {
    if hexaddr.len() != 32 {
        return hexaddr.to_string();
    }
    let mut bytes = [0u8; 16];
    for i in 0..16 {
        let s = &hexaddr[i * 2..i * 2 + 2];
        bytes[i] = u8::from_str_radix(s, 16).unwrap_or(0);
    }
    let addr = Ipv6Addr::from(bytes);
    addr.to_string()
}

impl Parser for ProcNetTcpParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, _quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Ok(ParseOutput::Array(vec![]));
        }

        let mut results: Vec<Map<String, Value>> = Vec::new();

        // skip header line
        for line in input.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 12 {
                continue;
            }

            let mut map = Map::new();

            // entry: remove trailing ':'
            let entry = parts[0].trim_end_matches(':');
            map.insert("entry".to_string(), Value::String(entry.to_string()));

            // local_address: "HEXIP:HEXPORT"
            let local = parts[1];
            let (local_ip_hex, local_port_hex) = split_addr_port(local);
            let local_ip = hex_to_ip(local_ip_hex);
            let local_port = i64::from_str_radix(local_port_hex, 16).unwrap_or(0);
            map.insert("local_address".to_string(), Value::String(local_ip));
            map.insert("local_port".to_string(), Value::Number(local_port.into()));

            // remote_address
            let remote = parts[2];
            let (remote_ip_hex, remote_port_hex) = split_addr_port(remote);
            let remote_ip = hex_to_ip(remote_ip_hex);
            let remote_port = i64::from_str_radix(remote_port_hex, 16).unwrap_or(0);
            map.insert("remote_address".to_string(), Value::String(remote_ip));
            map.insert("remote_port".to_string(), Value::Number(remote_port.into()));

            // state
            map.insert("state".to_string(), Value::String(parts[3].to_string()));

            // tx_queue:rx_queue has the format "XXXXXXXX:XXXXXXXX"
            let queues = parts[4];
            let (tx, rx) = queues.split_once(':').unwrap_or((queues, "00000000"));
            map.insert("tx_queue".to_string(), Value::String(tx.to_string()));
            map.insert("rx_queue".to_string(), Value::String(rx.to_string()));

            // timer_active:jiffies has the format "XX:XXXXXXXX"
            let timer_field = parts[5];
            let (timer_hex, jiffies) = timer_field.split_once(':').unwrap_or(("00", "00000000"));
            let timer_active: i64 = i64::from_str_radix(timer_hex, 16).unwrap_or(0);
            map.insert(
                "timer_active".to_string(),
                Value::Number(timer_active.into()),
            );
            map.insert(
                "jiffies_until_timer_expires".to_string(),
                Value::String(jiffies.to_string()),
            );

            // unrecovered_rto_timeouts
            map.insert(
                "unrecovered_rto_timeouts".to_string(),
                Value::String(parts[6].to_string()),
            );

            // uid
            let uid: i64 = parts[7].parse().unwrap_or(0);
            map.insert("uid".to_string(), Value::Number(uid.into()));

            // unanswered_0_window_probes
            let probes: i64 = parts[8].parse().unwrap_or(0);
            map.insert(
                "unanswered_0_window_probes".to_string(),
                Value::Number(probes.into()),
            );

            // inode
            let inode: i64 = parts[9].parse().unwrap_or(0);
            map.insert("inode".to_string(), Value::Number(inode.into()));

            // sock_ref_count
            let ref_count: i64 = parts[10].parse().unwrap_or(0);
            map.insert(
                "sock_ref_count".to_string(),
                Value::Number(ref_count.into()),
            );

            // sock_mem_loc
            map.insert(
                "sock_mem_loc".to_string(),
                Value::String(parts[11].to_string()),
            );

            // optional fields (not always present)
            if parts.len() > 12 {
                let retransmit: i64 = parts[12].parse().unwrap_or(0);
                map.insert(
                    "retransmit_timeout".to_string(),
                    Value::Number(retransmit.into()),
                );
                let soft_tick: i64 = parts[13].parse().unwrap_or(0);
                map.insert(
                    "soft_clock_tick".to_string(),
                    Value::Number(soft_tick.into()),
                );
                let ack_quick: i64 = parts[14].parse().unwrap_or(0);
                map.insert(
                    "ack_quick_pingpong".to_string(),
                    Value::Number(ack_quick.into()),
                );
                let cong_window: i64 = parts[15].parse().unwrap_or(0);
                map.insert(
                    "sending_congestion_window".to_string(),
                    Value::Number(cong_window.into()),
                );
                let slow_start: i64 = parts[16].parse().unwrap_or(0);
                map.insert(
                    "slow_start_size_threshold".to_string(),
                    Value::Number(slow_start.into()),
                );
            }

            // For IPv6 addresses, add opposite_endian fields
            if local_ip_hex.len() == 32 {
                let opp_local = hex_to_ip_opposite_endian(local_ip_hex);
                let opp_remote = hex_to_ip_opposite_endian(remote_ip_hex);
                map.insert(
                    "opposite_endian_local_address".to_string(),
                    Value::String(opp_local),
                );
                map.insert(
                    "opposite_endian_remote_address".to_string(),
                    Value::String(opp_remote),
                );
            }

            results.push(map);
        }

        Ok(ParseOutput::Array(results))
    }
}

/// Split "HEXIP:HEXPORT" into (&hex_ip, &hex_port).
/// For IPv4: ip is 8 chars, port is 4 chars.
/// For IPv6: ip is 32 chars, port is 4 chars.
fn split_addr_port(s: &str) -> (&str, &str) {
    // The last ':' separates ip from port
    match s.rfind(':') {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s, "0000"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_net_tcp() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_tcp");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_tcp.json"
        ))
        .unwrap();
        let result = ProcNetTcpParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }

    #[test]
    fn test_proc_net_tcp6() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/net_tcp6");
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/linux-proc/net_tcp6.json"
        ))
        .unwrap();
        let result = ProcNetTcpParser.parse(input, false).unwrap();
        let got = serde_json::to_value(result).unwrap();
        assert_eq!(got, expected);
    }
}
