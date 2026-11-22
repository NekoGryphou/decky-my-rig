use super::create_protected_directories;
use std::{io, path::Path, process::Command};

/// The folder's DACL in SDDL, which isn't localized like account names are.
fn dacl(path: &Path) -> String {
    // ? Windows PowerShell 5.1 can't load its modules from the PSModulePath that
    //   PowerShell 7 passes down (like on CI).
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
    let expected = "D:P(A;;FA;;;OW)(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";
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
