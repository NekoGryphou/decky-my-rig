use super::{detect_mac, select_mac};
use crate::platform::fixtures::TestPlatform;
use std::net::Ipv4Addr;

const TARGET: Ipv4Addr = Ipv4Addr::new(192, 168, 1, 40);
const FAILED: &str = "MAC detection failed. Enter the MAC address manually.";

fn select(macs: &[&str]) -> Result<String, &'static str> {
    let macs: Vec<String> = macs.iter().map(|mac| (*mac).to_owned()).collect();
    select_mac(&macs).map_err(|error| error.0)
}

#[test]
fn the_neighbor_of_the_host_is_detected_and_canonicalized() {
    let platform = TestPlatform {
        neighbors: vec![
            (
                Ipv4Addr::new(192, 168, 1, 4),
                "02:aa:bb:cc:dd:01".to_owned(),
            ),
            (TARGET, "02:aa:bb:cc:dd:02".to_owned()),
        ],
        ..TestPlatform::default()
    };
    assert_eq!(
        detect_mac(&platform, TARGET),
        Ok("02:AA:BB:CC:DD:02".to_owned())
    );
    assert_eq!(
        detect_mac(&TestPlatform::default(), TARGET).unwrap_err().0,
        FAILED
    );
}

#[test]
fn invalid_multicast_and_zero_addresses_are_not_returned() {
    for mac in ["invalid", "01:00:5e:00:00:01", "00:00:00:00:00:00"] {
        assert_eq!(select(&[mac]), Err(FAILED), "{mac}");
    }
}

#[test]
fn conflicting_entries_are_ambiguous_but_one_neighbor_twice_is_not() {
    let same = ["02:aa:bb:cc:dd:02", "02-AA-BB-CC-DD-02"];
    assert_eq!(select(&same), Ok("02:AA:BB:CC:DD:02".to_owned()));
    assert_eq!(
        select(&["02:aa:bb:cc:dd:02", "02:aa:bb:cc:dd:03"]),
        Err(FAILED)
    );
}
