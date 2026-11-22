//! Every network JSON body goes through CheckedJson: anything it accepts is
//! plain JSON with the same value.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_protocol::checked_json::CheckedJson;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let Ok(CheckedJson(value)) = serde_json::from_slice::<CheckedJson>(data) else {
        return;
    };
    assert_eq!(serde_json::from_slice::<Value>(data).ok(), Some(value));
});
