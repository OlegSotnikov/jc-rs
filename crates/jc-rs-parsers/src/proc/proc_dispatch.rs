//! Meta-parser for `/proc` files — dispatches to specific parsers by content detection.

use jc_rs_core::error::ParseError;
use jc_rs_core::registry::{ParserEntry, find_parser};
use jc_rs_core::traits::Parser;
use jc_rs_core::types::{ParseOutput, ParserInfo, Platform, Tag};
use regex::Regex;
use std::sync::LazyLock;

pub struct ProcDispatchParser;

static INFO: ParserInfo = ParserInfo {
    name: "proc",
    argument: "--proc",
    version: "1.0.0",
    description: "Automatically identifies and parses `/proc` files",
    author: "jc-rs contributors",
    author_email: "",
    compatible: &[Platform::Linux],
    tags: &[Tag::File, Tag::Slurpable],
    magic_commands: &[],
    streaming: false,
    hidden: false,
    deprecated: false,
};

static PROC_DISPATCH_PARSER: ProcDispatchParser = ProcDispatchParser;

inventory::submit! { ParserEntry::new(&PROC_DISPATCH_PARSER) }

/// jc's signature table, in jc's order -- the comments there mark pairs where
/// one pattern would swallow the other, so the order is part of the contract.
///
/// Python's `$` also matches just before a trailing newline while Rust's does
/// not, so patterns that jc anchors with `$` carry an explicit `\n?` here.
static SIGNATURES: &[(&str, &str)] = &[
    (
        r"^Node \d+, zone\s+\w+\s+(?:\d+\s+){11}\n",
        "proc_buddyinfo",
    ),
    (r"^BOOT_IMAGE=", "proc_cmdline"),
    (
        r"^\w+\s+[\-WUR]{3} \([ECBpba ]+\)\s+\d+:\d+\n",
        "proc_consoles",
    ),
    (
        r"(?s)^processor\t+: \d+.*bogomips\t+: \d+.\d\d\n",
        "proc_cpuinfo",
    ),
    (r"^name\s+:.*\ndriver\s+:.*\nmodule\s+:.*\n", "proc_crypto"),
    (r"^Character devices:\n\s+\d+ .*\n", "proc_devices"),
    (
        r"^\s*\d+\s+\d\s\w+\s(?:\d+\s){10,16}\d+\n",
        "proc_diskstats",
    ),
    (r"^(?:(?:nodev\t|\t)\w+\n){3}", "proc_filesystems"),
    (r"^\s+(?:CPU\d+ +)+\n\s*\d+:\s+\d+", "proc_interrupts"),
    (
        r"^00000000-[0-9a-f]{8} : .*\n[0-9a-f]{8}-[0-9a-f]{8} : ",
        "proc_iomem",
    ),
    (
        r"^0000-[0-9a-f]{4} : .*\n\s*0000-[0-9a-f]{4} : ",
        "proc_ioports",
    ),
    (
        r"^\d+.\d\d \d+.\d\d \d+.\d\d \d+/\d+ \d+\n?$",
        "proc_loadavg",
    ),
    (
        r"^\d+: (?:POSIX|FLOCK|OFDLCK)\s+(?:ADVISORY|MANDATORY)\s+(?:READ|WRITE) ",
        "proc_locks",
    ),
    (
        r"^MemTotal:.*\nMemFree:.*\nMemAvailable:.*\n",
        "proc_meminfo",
    ),
    (r"^\w+ \d+ \d+ (?:-|\w+,).*0x[0-9a-f]{16}\n", "proc_modules"),
    (r"^reg\d+: base=0x[0-9a-f]+ \(", "proc_mtrr"),
    (
        r"^Page block order:\s+\d+\nPages per block:\s+\d+\n\n",
        "proc_pagetypeinfo",
    ),
    (
        r"^major minor  #blocks  name\n\n\s*\d+\s+\d+\s+\d+ \w+\n",
        "proc_partitions",
    ),
    (r"^slabinfo - version: \d+.\d+\n", "proc_slabinfo"),
    (r"^\s+(CPU\d+\s+)+\n\s+HI:\s+\d", "proc_softirqs"),
    (r"(?s)^cpu\s+(?: \d+){7,10}.*intr ", "proc_stat"),
    (
        "^Filename\t\t\t\tType\t\tSize\t\tUsed\t\tPriority\n",
        "proc_swaps",
    ),
    (r"^\d+.\d\d \d+.\d\d\n?$", "proc_uptime"),
    (r"^.+\sversion\s[^\n]+\n?$", "proc_version"),
    (
        r"^0x[0-9a-f]{16}-0x[0-9a-f]{16}\s+\d+ \w+\+\w+/\w+ ",
        "proc_vmallocinfo",
    ),
    // zoneinfo before vmstat: vmstat's pattern also matches a zoneinfo file.
    (r"^Node \d+, zone\s+\w+\n", "proc_zoneinfo"),
    (r"(?s)nr_free_pages \d+\n.* \d\n?$", "proc_vmstat"),
    (
        r"^rtc_time\t: .*\nrtc_date\t: .*\nalrm_time\t: .*\n",
        "proc_driver_rtc",
    ),
    (
        r"^IP address\s+HW type\s+Flags\s+HW address\s+Mask\s+Device\n",
        "proc_net_arp",
    ),
    (r"^Inter-\|\s+Receive\s+\|\s+Transmit\n", "proc_net_dev"),
    (
        r"^[0-9a-f]{32} \d\d \d\d \d\d \d\d\s+\w+",
        "proc_net_if_inet6",
    ),
    (
        "^Idx\tDevice\\s+:\\s+Count\\s+Querier\tGroup\\s+Users\\s+Timer\tReporter\n",
        "proc_net_igmp",
    ),
    (
        r"^\d+\s+\w+\s+[0-9a-f]{32}\s+\d+\s+[0-9A-F]{8}\s+\d+",
        "proc_net_igmp6",
    ),
    (r"^sk\s+Eth Pid\s+Groups\s+Rmem\s+Wmem", "proc_net_netlink"),
    (
        r"^TcpExt: SyncookiesSent SyncookiesRecv SyncookiesFailed",
        "proc_net_netstat",
    ),
    (
        r"^sk       RefCnt Type Proto  Iface R Rmem   User   Inode\n",
        "proc_net_packet",
    ),
    (
        r"^protocol  size sockets  memory press maxhdr  slab module     cl co di ac io in de sh ss gs se re sp bi br ha uh gp em\n",
        "proc_net_protocols",
    ),
    (
        "^Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\\s+\n",
        "proc_net_route",
    ),
    (
        r"^\s+sl\s+local_address\s+(?:rem_address|remote_address)\s+st\s+tx_queue\s+rx_queue\s+tr\s+tm->when\s+retrnsmt\s+uid\s+timeout\s+inode",
        "proc_net_tcp",
    ),
    (
        r"^Num       RefCount Protocol Flags    Type St Inode Path\n",
        "proc_net_unix",
    ),
    // ipv6_route before dev_mcast: dev_mcast's pattern matches ipv6 route rows.
    (
        r"^[0-9a-f]{32} \d\d [0-9a-f]{32} \d\d [0-9a-f]{32} (?:[0-9a-f]{8} ){4}\s+\w+",
        "proc_net_ipv6_route",
    ),
    (
        r"^\d+\s+\w+\s+\d+\s+\d+\s+[0-9a-f]{12}",
        "proc_net_dev_mcast",
    ),
    (
        "^pos:\t\\d+\nflags:\t\\d+\nmnt_id:\t\\d+\n",
        "proc_pid_fdinfo",
    ),
    (r"^rchar: \d+\nwchar: \d+\nsyscr: \d+\n", "proc_pid_io"),
    (r"^\d+ \d+ \d+:\d+ /.+\n", "proc_pid_mountinfo"),
    (r"^[a-f0-9]{12} default [^\n]+\n", "proc_pid_numa_maps"),
    (
        r"(?s)^\d+ \(.+\) \S \d+ \d+ \d+ \d+ -?\d+ (?:\d+ ){43}\d+\n?$",
        "proc_pid_stat",
    ),
    (r"^\d+ \d+ \d+\s\d+\s\d+\s\d+\s\d+\n?$", "proc_pid_statm"),
    (
        r"^Name:\t.+\n(?:Umask:\t\d+\n)?State:\t.+\nTgid:\t\d+\n",
        "proc_pid_status",
    ),
    // smaps before maps: every smaps file opens with a maps line.
    (
        r"^[0-9a-f]{12}-[0-9a-f]{12} [rwxsp\-]{4} [0-9a-f]{8} [0-9a-f]{2}:[0-9a-f]{2} \d+ [^\n]+\nSize:\s+\d+ \S\S\n",
        "proc_pid_smaps",
    ),
    (
        r"^[0-9a-f]{12}-[0-9a-f]{12} [rwxsp\-]{4} [0-9a-f]{8} [0-9a-f]{2}:[0-9a-f]{2} \d+ ",
        "proc_pid_maps",
    ),
];

/// Compiled once; the table is 50 patterns and `--proc` is on the hot path for
/// every `jc-rs /proc/...` invocation.
static COMPILED: LazyLock<Vec<(Regex, &'static str)>> = LazyLock::new(|| {
    SIGNATURES
        .iter()
        .filter_map(|(pattern, name)| Regex::new(pattern).ok().map(|re| (re, *name)))
        .collect()
});

impl Parser for ProcDispatchParser {
    fn info(&self) -> &'static ParserInfo {
        &INFO
    }

    fn parse(&self, input: &str, quiet: bool) -> Result<ParseOutput, ParseError> {
        if input.trim().is_empty() {
            return Err(ParseError::InvalidInput(
                "Proc file could not be identified.".to_string(),
            ));
        }

        for (pattern, parser_name) in COMPILED.iter() {
            if !pattern.is_match(input) {
                continue;
            }
            return match find_parser(parser_name) {
                Some(parser) => parser.parse(input, quiet),
                None => Err(ParseError::InvalidInput(
                    "Proc file type not yet implemented.".to_string(),
                )),
            };
        }

        Err(ParseError::InvalidInput(
            "Proc file could not be identified.".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proc_dispatch_meminfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/meminfo");
        let parser = ProcDispatchParser;
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_proc_dispatch_cpuinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/cpuinfo");
        let parser = ProcDispatchParser;
        let result = parser.parse(input, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_proc_dispatch_pid_fdinfo() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_fdinfo_epoll");
        assert!(ProcDispatchParser.parse(input, false).is_ok());
    }

    #[test]
    fn test_proc_dispatch_pid_stat() {
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_stat_hack");
        assert!(ProcDispatchParser.parse(input, false).is_ok());
    }

    #[test]
    fn test_proc_dispatch_smaps_wins_over_maps() {
        // Both patterns match an smaps file; the order in SIGNATURES is what
        // decides, so assert on the field only smaps produces.
        let input = include_str!("../../../../tests/fixtures/linux-proc/pid_smaps");
        let out = ProcDispatchParser.parse(input, false).unwrap();
        let value = serde_json::to_value(out).unwrap();
        assert!(value[0].get("Size").is_some(), "parsed as maps, not smaps");
    }

    #[test]
    fn test_proc_dispatch_empty() {
        let parser = ProcDispatchParser;
        let result = parser.parse("", false);
        assert!(result.is_err());
    }
}
