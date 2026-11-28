use super::native::{current_owners, owner_of};
use crate::Windows;
use myrig_client::{
    store::PcChanges,
    testing::{StoreFixture, authentication, input},
};
use std::{fs, os::windows::ffi::OsStrExt, path::Path, process::Command, sync::Arc};
use windows::{
    Win32::{
        Foundation::{HLOCAL, LocalFree},
        Security::{
            Authorization::{
                ConvertSecurityDescriptorToStringSecurityDescriptorW,
                ConvertStringSecurityDescriptorToSecurityDescriptorW, GetNamedSecurityInfoW,
                SDDL_REVISION_1, SE_FILE_OBJECT,
            },
            DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, SetFileSecurityW,
        },
    },
    core::{PCWSTR, PWSTR},
};

fn fixture() -> StoreFixture {
    StoreFixture::new(Arc::new(Windows))
}

/// The DACL of `path` as SDDL, as Windows stores it.
fn dacl(path: &Path) -> String {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: `wide` is NUL-terminated; the descriptor is freed below.
    let status = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(wide.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            &mut descriptor,
        )
    };
    assert!(status.is_ok(), "{status:?}");
    // ! SAFETY: GetNamedSecurityInfoW returned this descriptor; it is freed by `sddl_of`.
    unsafe { sddl_of(descriptor) }
}

/// `sddl` the way Windows writes it back: an account with an alias (LA for the
/// built-in Administrator that CI runners use) is shown as the alias.
fn canonical(sddl: &str) -> String {
    let text: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: `text` is NUL-terminated; the descriptor is freed by `sddl_of`.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(text.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .unwrap();
    // ! SAFETY: the conversion returned this LocalAlloc'd descriptor.
    unsafe { sddl_of(descriptor) }
}

/// The DACL of `descriptor` as SDDL; frees the descriptor.
///
/// # Safety
/// `descriptor` must be a valid descriptor allocated with LocalAlloc.
unsafe fn sddl_of(descriptor: PSECURITY_DESCRIPTOR) -> String {
    let mut text = PWSTR::null();
    // ! SAFETY: the caller passes a valid descriptor; `text` is freed below.
    unsafe {
        ConvertSecurityDescriptorToStringSecurityDescriptorW(
            descriptor,
            SDDL_REVISION_1,
            DACL_SECURITY_INFORMATION,
            &mut text,
            None,
        )
    }
    .unwrap();
    // ! SAFETY: the converter returned a NUL-terminated string.
    let sddl = unsafe { text.to_string() }.unwrap();
    // ! SAFETY: both were allocated by the calls above and are freed once.
    unsafe {
        LocalFree(Some(HLOCAL(text.0.cast())));
        LocalFree(Some(HLOCAL(descriptor.0)));
    }
    sddl
}

#[test]
fn only_this_user_can_open_the_directory_and_document() {
    let fixture = fixture();
    let user = &current_owners().unwrap()[0];
    // * SetSecurityInfo marks the protected ACL as auto-inherited (AI).
    let private = canonical(&format!("D:PAI(A;OICI;FA;;;{user})"));
    assert_eq!(dacl(&fixture.directory), private);
    let pc = fixture.store.add(input("PC"), authentication()).unwrap();
    assert_eq!(
        dacl(&fixture.file()),
        canonical(&format!("D:PAI(A;;FA;;;{user})"))
    );
    assert_eq!(
        fixture.open(&fixture.directory).unwrap().configs(),
        vec![pc]
    );
}

#[test]
fn edits_are_saved_and_read_back() {
    let fixture = fixture();
    let pc = fixture.store.add(input("PC"), authentication()).unwrap();
    let renamed = fixture
        .store
        .update(
            pc.id,
            PcChanges {
                name: Some("Desk".to_owned()),
                ..PcChanges::default()
            },
        )
        .unwrap();
    let other = fixture.open(&fixture.directory).unwrap();
    assert_eq!(other.configs(), vec![renamed]);
    fixture.store.delete(pc.id).unwrap();
    assert!(
        fixture
            .open(&fixture.directory)
            .unwrap()
            .configs()
            .is_empty()
    );
}

#[test]
fn a_junction_as_the_folder_is_refused_but_one_above_it_is_followed() {
    let fixture = fixture();
    let alias = fixture.directory.parent().unwrap().join("alias");
    let made = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&alias)
        .arg(&fixture.directory)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    assert!(fixture.open(&alias).is_err());
    // - A profile on another drive, reached through a junction, still works.
    let store = fixture.open(&alias.join("nested")).unwrap();
    store.add(input("PC"), authentication()).unwrap();
    assert!(fixture.directory.join("nested").join("pcs.json").is_file());
}

#[test]
fn hard_linked_document_is_refused() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    fs::hard_link(
        fixture.file(),
        fixture.directory.parent().unwrap().join("copy"),
    )
    .unwrap();
    assert!(fixture.open(&fixture.directory).is_err());
}

/// Replaces `path`'s DACL with `sddl`, the way a copy or another tool might.
fn set_dacl(path: &Path, sddl: &str) {
    let text: Vec<u16> = sddl.encode_utf16().chain(Some(0)).collect();
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: both strings are NUL-terminated; the descriptor is freed below.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(text.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
        .unwrap();
        SetFileSecurityW(PCWSTR(wide.as_ptr()), DACL_SECURITY_INFORMATION, descriptor)
            .ok()
            .unwrap();
        LocalFree(Some(HLOCAL(descriptor.0)));
    }
}

#[test]
fn a_wider_document_acl_is_refused_and_left_for_the_user_to_fix() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    let user = &current_owners().unwrap()[0];
    // - Users (BU) can read it, as if the file had been copied in from somewhere
    //   else.
    let wider = format!("D:(A;;FR;;;BU)(A;;FA;;;{user})");
    set_dacl(&fixture.file(), &wider);
    let before = dacl(&fixture.file());
    let error = fixture
        .store
        .add(input("Two"), authentication())
        .unwrap_err();
    assert_eq!(error.0, "PC configuration is not private to this user.");
    // - A read never writes an ACL.
    assert_eq!(dacl(&fixture.file()), before);
}

#[test]
fn reading_never_changes_the_document_acl() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    let user = &current_owners().unwrap()[0];
    // - Private to this user, but not the exact ACL a save writes.
    set_dacl(&fixture.file(), &format!("D:P(A;;FR;;;{user})"));
    let before = dacl(&fixture.file());
    let other = fixture.open(&fixture.directory).unwrap();
    assert_eq!(other.configs().len(), 1);
    assert_eq!(dacl(&fixture.file()), before);
}

#[test]
fn a_save_gives_the_document_a_private_acl_again() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    let user = &current_owners().unwrap()[0];
    set_dacl(&fixture.file(), &format!("D:P(A;;FR;;;{user})"));
    fixture.store.add(input("Two"), authentication()).unwrap();
    assert_eq!(
        dacl(&fixture.file()),
        canonical(&format!("D:PAI(A;;FA;;;{user})"))
    );
}

#[test]
fn a_wider_folder_acl_is_refused_when_the_folder_is_opened_again() {
    let fixture = fixture();
    let user = &current_owners().unwrap()[0];
    set_dacl(
        &fixture.directory,
        &format!("D:(A;OICI;FR;;;BU)(A;OICI;FA;;;{user})"),
    );
    let before = dacl(&fixture.directory);
    let error = fixture
        .store
        .add(input("PC"), authentication())
        .unwrap_err();
    assert_eq!(error.0, "PC configuration is not private to this user.");
    // - Checking never writes the ACL.
    assert_eq!(dacl(&fixture.directory), before);
}

#[test]
fn the_document_is_owned_by_this_user() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    let file = std::fs::File::open(fixture.file()).unwrap();
    assert_eq!(owner_of(&file).unwrap(), current_owners().unwrap()[0]);
}

#[test]
fn a_leftover_temporary_file_is_removed_on_the_next_save() {
    fixture().assert_saves_remove_leftovers();
}
