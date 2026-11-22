use super::{service::neighbor_macs_in, utils::complete_ethernet_entries};
use std::{fs, net::Ipv4Addr};
use tempfile::TempDir;

const HEADER: &str = "IP address HW type Flags HW address Mask Device\n";
const TARGET: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 40);

fn entries(rows: &str) -> Vec<String> {
    complete_ethernet_entries(&format!("{HEADER}{rows}"), TARGET)
}

#[test]
fn only_complete_ethernet_rows_for_the_exact_host_are_returned() {
    let rows = "192.168.1.4 0x1 0x2 02:aa:bb:cc:dd:01 * wlan0\n192.168.1.40 0x1 0x2 02:aa:bb:cc:dd:02 * wlan0\n192.168.1.40 0x1 0x6 02:aa:bb:cc:dd:03 * eth0\n";
    assert_eq!(entries(rows), ["02:aa:bb:cc:dd:02", "02:aa:bb:cc:dd:03"]);
    for rows in [
        "",
        "192.168.1.40 0x1 0x0 02:aa:bb:cc:dd:02 * wlan0\n",
        "192.168.1.40 0x2 0x2 02:aa:bb:cc:dd:02 * wlan0\n",
        "192.168.1.400 0x1 0x2 02:aa:bb:cc:dd:02 * wlan0\n",
    ] {
        assert!(entries(rows).is_empty(), "{rows}");
    }
}

#[test]
fn unreadable_and_oversized_caches_fail_safely() {
    let directory = TempDir::new().unwrap();
    let cache = directory.path().join("arp");
    assert!(neighbor_macs_in(&cache, TARGET).is_err());
    fs::write(&cache, format!("{HEADER}{}", "x".repeat(1_048_577))).unwrap();
    assert!(neighbor_macs_in(&cache, TARGET).is_err());
    fs::write(
        &cache,
        format!("{HEADER}192.168.1.40 0x1 0x2 02:aa:bb:cc:dd:02 * wlan0\n"),
    )
    .unwrap();
    assert_eq!(
        neighbor_macs_in(&cache, TARGET).unwrap(),
        ["02:aa:bb:cc:dd:02"]
    );
}
