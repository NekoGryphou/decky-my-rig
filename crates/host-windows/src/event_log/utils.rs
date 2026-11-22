use std::io;

pub(super) fn listener_error_message(port: u16, error: &io::Error) -> String {
    format!(
        "MyRigHost TCP listener on port {port} failed: kind={:?}, raw_os_error={:?}",
        error.kind(),
        error.raw_os_error()
    )
}
