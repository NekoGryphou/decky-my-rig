use super::CliError;
use myrig_protocol::local_control::client::ReplyError;

#[test]
fn each_error_has_its_own_exit_code() {
    for (error, code, message) in [
        (CliError::Unavailable, 3, "My Rig service is not available."),
        (
            CliError::Unauthorized,
            4,
            "Not authorized. Ask an administrator to add you to My Rig Managers, then sign out and back in.",
        ),
        (
            CliError::Host("storage_failed"),
            1,
            "Host error: storage_failed",
        ),
        (CliError::Host("unavailable"), 1, "Host error: unavailable"),
        (CliError::NotPaired, 1, "Host error: not_paired"),
    ] {
        assert_eq!((error.exit_code(), error.message()), (code, message.into()));
    }
}

#[test]
fn an_unreadable_reply_means_the_service_is_unavailable() {
    assert_eq!(CliError::from(ReplyError::Invalid), CliError::Unavailable);
    assert_eq!(
        CliError::from(ReplyError::Unauthorized),
        CliError::Unauthorized
    );
}
