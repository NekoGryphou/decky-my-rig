use std::io;

pub(super) fn listener_error_message(port: u16, error: &io::Error) -> String {
    format!(
        "MyRigHost TCP listener on port {port} failed: kind={:?}, raw_os_error={:?}",
        error.kind(),
        error.raw_os_error()
    )
}

pub(super) fn shutdown_failure_message(code: u32) -> String {
    format!("MyRigHost could not schedule a shutdown: code=0x{code:08X}")
}

pub(super) fn notifier_failure_message(action: &str, error: &str) -> String {
    format!("MyRig notifier could not {action}: {error}")
}
