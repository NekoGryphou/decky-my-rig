//! A local-control frame is exactly its length prefix and that many bytes,
//! within the limit.
// - A local-control frame is its length prefix followed by that many bytes,
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_protocol::local_control::framing::read_frame;

const LIMIT: usize = 4096;

fuzz_target!(|data: &[u8]| {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime without I/O or timer drivers needs no OS resources");
    let mut reader = data;
    let Ok(frame) = runtime.block_on(read_frame(&mut reader, LIMIT)) else {
        return;
    };
    assert!(!frame.is_empty() && frame.len() <= LIMIT);
    assert_eq!(
        u32::from_le_bytes(
            data[..4]
                .try_into()
                .expect("read_frame read the 4-byte prefix, so data is at least 4 bytes"),
        ) as usize,
        frame.len()
    );
    assert_eq!(frame, data[4..4 + frame.len()]);
});
