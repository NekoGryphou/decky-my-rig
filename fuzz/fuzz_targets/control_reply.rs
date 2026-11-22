//! Service replies as the host CLI and notifier parse them.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_protocol::local_control::client::{
    ListReply, OpenPairingReply, PortReply, RemoveReply, RenameReply, StatusReply, TestModeReply,
    WaitReply, parse,
};

fuzz_target!(|data: &[u8]| {
    let _ = parse::<StatusReply>(data);
    let _ = parse::<ListReply>(data);
    let _ = parse::<RemoveReply>(data);
    let _ = parse::<RenameReply>(data);
    let _ = parse::<PortReply>(data);
    let _ = parse::<TestModeReply>(data);
    let _ = parse::<OpenPairingReply>(data);
    let _ = parse::<WaitReply>(data);
});
