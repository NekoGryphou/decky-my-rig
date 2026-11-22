use super::{
    PublicError, normalize_host, normalize_mac, normalize_name, normalize_port, normalize_token,
    normalize_uuid,
};
use std::{fmt::Debug, net::Ipv4Addr};

fn assert_rejected<T: Debug>(normalize: impl Fn(&str) -> Result<T, PublicError>, values: &[&str]) {
    for value in values {
        assert!(normalize(value).is_err(), "{value:?} was accepted");
    }
}

#[test]
fn names_are_trimmed_visible_text_of_1_to_80_characters() {
    assert_eq!(normalize_name("  Gaming PC  "), Ok("Gaming PC".to_owned()));
    assert_eq!(normalize_name(&"é".repeat(80)), Ok("é".repeat(80)));
    assert_rejected(
        normalize_name,
        &[
            "",
            "  ",
            &"a".repeat(81),
            "PC\n",
            "\0PC",
            "PC\x7f",
            "PC\u{200B}",
        ],
    );
}

#[test]
fn ports_are_1_to_65535_in_at_most_five_ascii_digits() {
    for (source, expected) in [("1", 1), ("65535", 65535), (" 47991 ", 47991), ("00009", 9)] {
        assert_eq!(normalize_port(source), Ok(expected));
    }
    assert_rejected(
        normalize_port,
        &["0", "65536", "1.2", "+9", "-1", "９", "000009", "9\n", ""],
    );
}

#[test]
fn macs_normalize_to_uppercase_colon_pairs() {
    for source in ["AABBCCDDEEFF", "aa-bb-cc-dd-ee-ff", " AA:BB:CC:DD:EE:FF "] {
        assert_eq!(normalize_mac(source), Ok("AA:BB:CC:DD:EE:FF".to_owned()));
    }
    assert_eq!(
        normalize_mac("020000000001"),
        Ok("02:00:00:00:00:01".to_owned())
    );
}

#[test]
fn malformed_zero_and_multicast_macs_are_rejected() {
    assert_rejected(
        normalize_mac,
        &[
            "00:00:00:00:00:00",
            "FF:FF:FF:FF:FF:FF",
            "01:00:5E:00:00:01",
            "AA-BB:CC:DD:EE:FF",
            "A:B:C:D:E:F",
            "AABBCCDDEEF",
            "AABBCCDDEEFF00",
            "AABBCCDDEEGG",
            "aabb.ccdd.eeff",
        ],
    );
    assert_eq!(
        normalize_mac("01:00:5E:00:00:01").unwrap_err().0,
        "Enter a nonzero unicast MAC address."
    );
}

#[test]
fn hosts_are_canonical_unicast_ipv4() {
    for (source, expected) in [
        (" 192.168.1.42 ", Ipv4Addr::new(192, 168, 1, 42)),
        ("127.0.0.1", Ipv4Addr::LOCALHOST),
    ] {
        assert_eq!(normalize_host(source), Ok(expected));
    }
}

#[test]
fn host_names_and_non_canonical_addresses_are_rejected() {
    let long_label = "a".repeat(64);
    let long_name = ["a".repeat(63).as_str(); 4].join(".");
    assert_rejected(
        normalize_host,
        &[
            "PC.Example.",
            "pc-2",
            "office-pc",
            "999.1.2.3",
            "192.168.01.2",
            "127.1",
            "123",
            "1.2.3.4.5",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "http://pc",
            "pc:47991",
            "::1",
            "[::1]",
            "pc/path",
            "user@pc",
            "pc name",
            "pc\n",
            "pc;echo",
            "$(pc)",
            "pc_1",
            "é",
            "-pc",
            "pc..",
            &long_label,
            &long_name,
            "",
        ],
    );
    assert_eq!(
        normalize_host("224.0.0.1").unwrap_err().0,
        "Enter the PC's unicast IPv4 address."
    );
}

#[test]
fn tokens_are_strict_lowercase_hex_and_errors_do_not_echo_them() {
    let example = "ab".repeat(32);
    assert_eq!(
        normalize_token(&format!(" {example} ")),
        Ok(example.clone())
    );
    assert_rejected(
        normalize_token,
        &[
            &example.to_uppercase(),
            &"ab".repeat(31),
            &"ab".repeat(33),
            &"g".repeat(64),
        ],
    );
    let error = normalize_token("secret-value").unwrap_err();
    assert!(!error.to_string().contains("secret-value"));
}

#[test]
fn uuids_are_canonical() {
    let identifier = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
    assert_eq!(
        normalize_uuid(&format!(" {identifier} ")).map(|id| id.to_string()),
        Ok(identifier.to_owned())
    );
    assert_rejected(
        normalize_uuid,
        &[
            &identifier.to_uppercase(),
            &identifier.replace('-', ""),
            &format!("{{{identifier}}}"),
            &format!("urn:uuid:{identifier}"),
            "bad",
        ],
    );
}
