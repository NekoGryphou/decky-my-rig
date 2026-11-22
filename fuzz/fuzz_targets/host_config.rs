//! The host configuration loads only what it would write back unchanged.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_host_core::paired_devices::RegistryConfig;

fuzz_target!(|data: &[u8]| {
    let Ok(config) = RegistryConfig::from_json(data) else {
        return;
    };
    let written = config.to_json().unwrap();
    let again = RegistryConfig::from_json(&written).unwrap();
    assert_eq!(again.to_json().unwrap(), written);
});
