use super::{types::ServiceExit, utils::check_security_folder};

#[test]
fn a_security_folder_that_standard_users_can_write_is_refused() {
    // * A fresh folder under the user's temp folder belongs to the user and inherits their write
    // * access, which is what an unsafe identity folder looks like.
    let folder = tempfile::tempdir().unwrap();
    assert!(matches!(
        check_security_folder(folder.path()),
        Err(ServiceExit::Identity)
    ));
}

#[test]
fn a_missing_security_folder_is_refused() {
    let parent = tempfile::tempdir().unwrap();
    assert!(matches!(
        check_security_folder(&parent.path().join("MyRigSecurity")),
        Err(ServiceExit::Identity)
    ));
}
