use super::{Command, utils::quoted_launch_path};

#[test]
fn accepts_only_the_two_login_flags_or_no_flag() {
    assert_eq!(Command::parse(&[]), Ok(Command::Run));
    assert_eq!(
        Command::parse(&["--enable-autostart".into()]),
        Ok(Command::Enable)
    );
    assert_eq!(
        Command::parse(&["--disable-autostart".into()]),
        Ok(Command::Disable)
    );
    assert!(Command::parse(&["--enable-autostart".into(), "extra".into()]).is_err());
    assert!(Command::parse(&["--unknown".into()]).is_err());
}

#[test]
fn login_entry_quotes_a_path_with_spaces() {
    assert_eq!(
        quoted_launch_path(r"C:\Program Files\My Rig\myrig-notifier.exe"),
        Ok(r#""C:\Program Files\My Rig\myrig-notifier.exe""#.to_owned())
    );
    assert!(quoted_launch_path("bad\"path").is_err());
}
