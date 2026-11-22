use crate::Windows;
use myrig_client::platform::Platform;

#[test]
fn settings_live_in_local_app_data_and_the_hostname_is_known() {
    let directory = Windows.settings_directory().unwrap();
    assert!(
        directory.ends_with("decky-my-rig"),
        "{}",
        directory.display()
    );
    assert!(directory.is_absolute());
    assert!(!Windows.hostname().unwrap().is_empty());
}
