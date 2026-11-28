//! The host configuration loads only what it would write back unchanged.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_host_core::paired_devices::RegistryConfig;

fuzz_target!(|data: &[u8]| {
    let Ok(config) = RegistryConfig::from_json(data) else {
        return;
    };
    let written = config
        .to_json()
        .expect("serializing strings, numbers and booleans to JSON cannot fail");
    // ! A failure here means the config does not round-trip, so it panics.
    let again = RegistryConfig::from_json(&written)
        .expect("the saved form of an accepted config loads again (checked by this target)");
    assert_eq!(
        again
            .to_json()
            .expect("serializing strings, numbers and booleans to JSON cannot fail"),
        written
    );
});
