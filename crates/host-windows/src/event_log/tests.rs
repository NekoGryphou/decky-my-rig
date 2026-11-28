use super::utils::{listener_error_message, notifier_failure_message, shutdown_failure_message};
use std::io;

#[test]
fn bind_failure_keeps_os_kind_and_raw_code() {
    let error = io::Error::from_raw_os_error(10048);
    let message = listener_error_message(47991, &error);
    assert!(message.contains("10048"));
    assert!(message.contains(&format!("{:?}", error.kind())));
    assert!(message.contains("47991"));
}

#[test]
fn shutdown_failure_keeps_the_error_code() {
    assert!(shutdown_failure_message(0x8007_0522).contains("0x80070522"));
}

#[test]
fn notifier_failure_names_the_action_and_the_cause() {
    let message = notifier_failure_message("register its notifications", "access denied");
    assert!(message.contains("register its notifications"));
    assert!(message.contains("access denied"));
}
