use super::{
    consts::MAX_HOSTNAME_BYTES,
    utils::{config_dir_from, read_hostname},
};

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

#[test]
fn the_hostname_file_is_read_with_a_size_limit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hostname");
    std::fs::write(&path, "steamdeck\n").unwrap();
    assert_eq!(read_hostname(&path).as_deref(), Some("steamdeck\n"));
    std::fs::write(&path, vec![b'a'; MAX_HOSTNAME_BYTES + 1]).unwrap();
    assert_eq!(read_hostname(&path), None);
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert_eq!(read_hostname(&path), None);
    assert_eq!(read_hostname(&directory.path().join("missing")), None);
}

#[test]
fn echo_is_off_only_while_the_guard_lives_on_a_terminal() {
    // ! A pseudo-terminal stands in for the console: the guard must clear ECHO on it, and restore
    // ! the flag it found once dropped.
    let (mut master, mut slave) = (0, 0);
    // ! SAFETY: openpty writes two descriptors into the out-pointers when it succeeds.
    let opened = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(opened, 0);
    let echo_on = |fd: i32| {
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        // ! SAFETY: tcgetattr initializes the structure on success, checked here.
        assert_eq!(unsafe { libc::tcgetattr(fd, settings.as_mut_ptr()) }, 0);
        // - SAFETY: initialized just above.
        unsafe { settings.assume_init() }.c_lflag & libc::ECHO != 0
    };
    assert!(echo_on(slave));
    let guard = super::native::Echo::hide_on(slave).expect("a terminal can hide echo");
    assert!(!echo_on(slave));
    drop(guard);
    assert!(echo_on(slave));
    // ! SAFETY: both descriptors came from openpty and are closed once.
    unsafe {
        libc::close(master);
        libc::close(slave);
    }
}
