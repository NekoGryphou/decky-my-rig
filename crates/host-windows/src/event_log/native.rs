use super::utils::listener_error_message;
use std::io;
use windows::{
    Win32::System::EventLog::{
        DeregisterEventSource, EVENTLOG_ERROR_TYPE, RegisterEventSourceW, ReportEventW,
    },
    core::{PCWSTR, w},
};

pub fn report_listener_error(port: u16, error: &io::Error) {
    let message: Vec<u16> = listener_error_message(port, error)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    // ! SAFETY: Both strings are terminated and live through ReportEventW; the
    //   returned event source handle is released before this function exits.
    unsafe {
        if let Ok(source) = RegisterEventSourceW(PCWSTR::null(), w!("MyRigHost")) {
            let _ = ReportEventW(
                source,
                EVENTLOG_ERROR_TYPE,
                0,
                1,
                None,
                0,
                Some(&[PCWSTR(message.as_ptr())]),
                None,
            );
            let _ = DeregisterEventSource(source);
        }
    }
}
