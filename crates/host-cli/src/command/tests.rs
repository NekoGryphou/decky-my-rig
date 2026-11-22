use super::{Command, parse_command};

#[test]
fn parses_exact_management_commands() {
    assert_eq!(parse_command(&["status".into()]), Some(Command::Status));
    assert_eq!(parse_command(&["list".into()]), Some(Command::List));
    assert_eq!(
        parse_command(&["pair".into(), "Steam Deck".into()]),
        Some(Command::Pair("Steam Deck".into()))
    );
    assert_eq!(
        parse_command(&["set-port".into(), "47992".into()]),
        Some(Command::SetPort(47992))
    );
    assert_eq!(
        parse_command(&["test-mode".into(), "on".into()]),
        Some(Command::SetTestMode(true))
    );
    assert_eq!(
        parse_command(&["test-mode".into(), "off".into()]),
        Some(Command::SetTestMode(false))
    );
    assert_eq!(
        parse_command(&[
            "remove".into(),
            "03c85317-9d93-4fb3-a04b-91a0e16668c6".into()
        ]),
        Some(Command::Remove(
            "03c85317-9d93-4fb3-a04b-91a0e16668c6".into()
        ))
    );
    assert_eq!(
        parse_command(&[
            "rename".into(),
            "03c85317-9d93-4fb3-a04b-91a0e16668c6".into(),
            "Living room device".into()
        ]),
        Some(Command::Rename(
            "03c85317-9d93-4fb3-a04b-91a0e16668c6".into(),
            "Living room device".into()
        ))
    );
}

#[test]
fn rejects_invalid_arguments() {
    for args in [
        vec![],
        vec!["status".into(), "extra".into()],
        vec!["set-port".into(), "1023".into()],
        vec!["set-port".into(), "abc".into()],
        vec!["list".into(), "extra".into()],
        vec!["unknown".into()],
        vec!["set-port".into(), "65536".into()],
        vec!["test-mode".into(), "yes".into()],
        vec!["pair".into(), "".into()],
        vec!["remove".into(), "wrong".into()],
        vec!["rename".into(), "wrong".into(), "Device".into()],
        vec![
            "rename".into(),
            "03c85317-9d93-4fb3-a04b-91a0e16668c6".into(),
            "".into(),
        ],
    ] {
        assert!(parse_command(&args).is_none(), "accepted {args:?}");
    }
}

#[test]
fn pairing_name_must_be_valid_before_contacting_the_service() {
    for name in ["Device\nName".to_owned(), "x".repeat(81)] {
        assert!(parse_command(&["pair".into(), name]).is_none());
    }
}
