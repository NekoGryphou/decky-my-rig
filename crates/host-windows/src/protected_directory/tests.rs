use super::create_protected_directories;
use std::{io, path::Path, process::Command};

/// The folder's DACL in SDDL, which isn't localized like account names are.
fn dacl(path: &Path) -> String {
    // ? Windows PowerShell 5.1 can't load its modules from the PSModulePath that PowerShell 7
    // ? passes down.
    let output = Command::new("powershell.exe")
        .env_remove("PSModulePath")
        .args(["-NoProfile", "-NonInteractive", "-Command"])
        .arg(format!(
            "(Get-Acl -LiteralPath '{}').Sddl",
            path.display().to_string().replace('\'', "''")
        ))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let sddl = String::from_utf8(output.stdout).unwrap();
    let start = sddl.find("D:").unwrap();
    sddl[start..].trim().to_owned()
}

#[test]
fn new_folders_are_born_protected_for_system_and_administrators_only() {
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("MyRig");
    let child = folder.join("Security");
    create_protected_directories(&[&folder, &child]).unwrap();
    let protected = (dacl(&folder), dacl(&child));
    // - In the order Get-Acl reports them.
    let expected = "D:P(A;;RC;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";
    assert_eq!(protected, (expected.to_owned(), expected.to_owned()));
}

#[test]
fn an_existing_folder_is_refused_unchanged_and_the_new_ones_removed() {
    let parent = tempfile::tempdir().unwrap();
    let before = dacl(parent.path());
    let created = parent.path().join("MyRig");
    let error = create_protected_directories(&[&created, parent.path()]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(dacl(parent.path()), before);
    assert!(!created.exists());
}

// ! The folder owner comes from the creating token: Administrators when elevated, as the installer
// ! always is, and the user otherwise. Run this elevated with `cargo test -- --ignored`.
#[test]
#[ignore = "needs an elevated run, as the installer is"]
fn a_created_folder_passes_the_data_folder_check_the_service_runs() {
    // ! The service refuses a data folder that fails this check, so a folder the installer creates
    // ! must pass it straight away.
    let parent = tempfile::tempdir().unwrap();
    let folder = parent.path().join("MyRig");
    create_protected_directories(&[&folder]).unwrap();
    crate::install_directory::InstallDirectory::validate_data_folder(&folder)
        .expect("a folder from create-data passes the check");
}
