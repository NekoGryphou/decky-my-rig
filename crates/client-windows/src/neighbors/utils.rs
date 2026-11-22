/// `AA:BB:CC:DD:EE:FF` for a six-byte Ethernet address, none otherwise.
pub(super) fn ethernet_mac(address: &[u8]) -> Option<String> {
    (address.len() == 6).then(|| {
        address
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(":")
    })
}
