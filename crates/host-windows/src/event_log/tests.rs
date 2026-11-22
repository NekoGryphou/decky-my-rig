use super::utils::listener_error_message;
use std::io;

#[test]
fn bind_failure_keeps_os_kind_and_raw_code() {
    let error = io::Error::from_raw_os_error(10048);
    let message = listener_error_message(47991, &error);
    assert!(message.contains("10048"));
    assert!(message.contains(&format!("{:?}", error.kind())));
    assert!(message.contains("47991"));
}
