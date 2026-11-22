use super::utils::config_dir_from;

#[test]
fn the_store_follows_xdg_config_home() {
    assert_eq!(
        config_dir_from(Some("/x/config".into()), Some("/home/u".into())),
        Some("/x/config/decky-my-rig".into())
    );
    assert_eq!(
        config_dir_from(Some("relative".into()), Some("/home/u".into())),
        Some("/home/u/.config/decky-my-rig".into())
    );
    assert_eq!(
        config_dir_from(None, Some("/home/u".into())),
        Some("/home/u/.config/decky-my-rig".into())
    );
    assert_eq!(config_dir_from(None, None), None);
}
