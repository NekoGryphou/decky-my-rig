use super::utils::{listener_error_message, notifier_failure_message, shutdown_failure_message};
use std::io;
use windows::{
    Win32::System::EventLog::{
        DeregisterEventSource, EVENTLOG_ERROR_TYPE, RegisterEventSourceW, ReportEventW,
    },
    core::{PCWSTR, w},
};

pub fn report_listener_error(port: u16, error: &io::Error) {
    report_error(&listener_error_message(port, error));
}

pub fn report_shutdown_failure(code: u32) {
    report_error(&shutdown_failure_message(code));
}

/// The notifier has no console, so this is where its failures can be read.
pub fn report_notifier_failure(action: &str, error: &str) {
    report_error(&notifier_failure_message(action, error));
}

fn report_error(text: &str) {
    let message: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
    // ! SAFETY: Both strings are terminated and live through ReportEventW; the returned event
    // ! source handle is released before this function exits.
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
