use super::utils::{control_pipe_sddl, manager_users};

#[test]
fn bind_sddl_sets_service_owner_and_one_client_entry_per_manager() {
    let managers = ["S-1-5-32-1001".to_owned(), "S-1-5-21-1-2-3-1001".to_owned()];
    assert_eq!(
        control_pipe_sddl("S-1-5-80-123", &managers),
        "O:S-1-5-80-123D:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;FA;;;S-1-5-80-123)(A;;0x0012019b;;;S-1-5-32-1001)(A;;0x0012019b;;;S-1-5-21-1-2-3-1001)(A;;RC;;;OW)"
    );
}

#[test]
fn recorded_users_are_granted_only_while_they_are_group_members() {
    let local = "S-1-5-21-3375904584-3235973717-1963357494-1001";
    let entra = "S-1-12-1-1111111111-2222222222-3333333333-444444444";
    let removed = "S-1-5-21-1-2-3-1002";
    let recorded = [
        local,
        local,
        entra,
        removed,
        // - Not a user account, or not a SID, so it's never granted.
        "S-1-5-32-544",
        "S-1-1-0",
        "S-1-5-21-1-2-3",
        "S-1-5-21-1-2-3-4294967296",
        "S-1-5-21-1-2-3-1001)(A;;FA;;;WD",
        "",
    ]
    .map(str::to_owned);
    let members = [local, entra, "S-1-5-21-1-2-3-4294967296"].map(str::to_owned);
    assert_eq!(manager_users(&recorded, &members), [local, entra]);
}
