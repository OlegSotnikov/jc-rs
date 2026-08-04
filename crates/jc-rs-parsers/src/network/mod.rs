//! Network command parsers.

pub mod arp;
pub mod curl_head;
pub mod dig;
pub mod ethtool;
pub mod host;
pub mod http_headers;
pub mod ifconfig;
pub mod iftop;
pub mod ip_address;
pub mod ip_route;
pub mod iw_scan;
pub mod iwconfig;
pub mod netstat;
pub mod nmcli;
pub mod ping;
pub mod ping_s;
pub mod route;
pub mod route_print;
pub mod rsync;
pub mod rsync_s;
pub mod ss;
pub mod tracepath;
pub mod traceroute;
pub mod traceroute_s;

#[cfg(test)]
mod tests;
