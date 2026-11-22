use super::utils::key_path;
use super::{register, unregister};
use windows_registry::CURRENT_USER;

#[test]
fn key_path_names_the_app_id_under_user_classes() {
    assert_eq!(
        key_path("MyRig.Notifier"),
        Ok(r"Software\Classes\AppUserModelId\MyRig.Notifier".to_owned())
    );
    for bad in ["", r"Device\Notifier", "Device\0"] {
        assert!(key_path(bad).is_err(), "accepted {bad:?}");
    }
}

#[test]
fn register_then_unregister_round_trips_in_the_user_registry() {
    let app_id = "MyRig.NotificationIdentityTest";
    let path = key_path(app_id).unwrap();
    register(app_id, "My Rig").unwrap();
    let name = CURRENT_USER.open(&path).unwrap().get_string("DisplayName");
    assert_eq!(name.unwrap(), "My Rig");
    unregister(app_id).unwrap();
    assert!(CURRENT_USER.open(&path).is_err());
    unregister(app_id).unwrap();
}
