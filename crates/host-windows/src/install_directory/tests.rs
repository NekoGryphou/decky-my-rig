use super::{InstallDirectory, native::decode, types::PathRole, utils::check_security};
use crate::shared::native::{LocalAllocation, wide};
use std::path::Path;
use windows::{
    Win32::Security::{
        Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1},
        PSECURITY_DESCRIPTOR,
    },
    core::PCWSTR,
};

fn accepted(sddl: &str, role: PathRole) -> bool {
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: the SDDL text is NUL-terminated and lives until the call returns.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(wide(sddl).as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .unwrap();
    let _allocation = LocalAllocation(descriptor.0);
    decode(descriptor)
        .and_then(|security| check_security(&security, role))
        .is_ok()
}

#[test]
fn writable_or_unowned_destinations_fail_closed() {
    for sddl in [
        "O:BAG:BAD:(A;;FA;;;BA)(A;;FW;;;WD)",
        "O:BAG:BAD:(A;;FA;;;BA)(A;;WD;;;AU)",
        "O:BAG:BAD:(A;;FA;;;BA)(A;;WO;;;BU)",
        "O:BAG:BAD:NO_ACCESS_CONTROL",
        "O:BUG:BAD:(A;;FA;;;BA)",
    ] {
        assert!(!accepted(sddl, PathRole::Destination), "{sddl}");
        assert!(!accepted(sddl, PathRole::File), "{sddl}");
    }
}

#[test]
fn deleting_a_child_and_creating_a_directory_are_admitted_separately() {
    assert!(!accepted(
        "O:BAG:BAD:(A;;FA;;;BA)(A;;0x40;;;WD)",
        PathRole::Ancestor
    ));
    assert!(!accepted(
        "O:BAG:BAD:(A;;FA;;;BA)(A;;SD;;;AU)",
        PathRole::Ancestor
    ));
    let create_only = "O:BAG:BAD:(A;;FA;;;BA)(A;;0x4;;;BU)";
    assert!(accepted(create_only, PathRole::Ancestor));
    assert!(!accepted(create_only, PathRole::CreationParent));
}

#[test]
fn creator_owner_inheritance_does_not_reject_administrator_created_files() {
    let safe = "O:BAG:BAD:(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)(A;OICI;FRFX;;;BU)(A;OICIIO;FA;;;CO)";
    assert!(accepted(safe, PathRole::Destination));
    assert!(accepted(safe, PathRole::CreationParent));
    let inherited_world_write = "O:BAG:BAD:(A;;FA;;;BA)(A;OICIIO;FW;;;WD)";
    assert!(accepted(inherited_world_write, PathRole::Ancestor));
    assert!(!accepted(inherited_world_write, PathRole::CreationParent));
}

#[test]
fn local_protected_path_check_does_not_create_the_directory() {
    let parent = std::env::var_os("ProgramFiles").unwrap();
    let path = Path::new(&parent).join(format!("MyRig-path-check-{}", std::process::id()));
    assert!(!path.exists());
    assert_eq!(InstallDirectory::validate(&path), Ok(()));
    assert!(!path.exists());
}

#[test]
fn unsafe_namespaces_and_real_user_writable_directories_are_refused() {
    for path in [
        r"relative\MyRig",
        r"\\server\share\MyRig",
        r"\\?\C:\MyRig",
        r"C:\Program Files\..\MyRig",
        r"C:\Program Files\MyRig:stream",
        r"C:\Program Files\MyRig.",
        r"C:\Program Files\NUL",
    ] {
        assert!(
            InstallDirectory::validate(Path::new(path)).is_err(),
            "{path}"
        );
    }
    assert!(InstallDirectory::validate(&std::env::temp_dir()).is_err());
}

#[test]
fn native_metadata_refuses_a_junction_without_touching_its_target() {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };
    let base = std::env::temp_dir().join(format!(
        "MyRig-directory-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&base).unwrap();
    let checked = std::panic::catch_unwind(|| {
        let target = base.join("target");
        let link = base.join("link");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("owned.txt"), "preserve").unwrap();
        let status = Command::new("cmd.exe")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(&target)
            .output()
            .unwrap();
        assert!(status.status.success());
        let rejected = super::native::metadata(&link).is_err();
        fs::remove_dir(&link).unwrap();
        assert_eq!(
            fs::read_to_string(target.join("owned.txt")).unwrap(),
            "preserve"
        );
        rejected
    });
    fs::remove_dir_all(&base).unwrap();
    assert!(checked.unwrap());
}

#[test]
fn new_payload_files_must_not_inherit_unprivileged_write_access() {
    let inherited_world_write = "O:BAG:BAD:(A;;FA;;;BA)(A;OICIIO;FW;;;WD)";
    assert!(!accepted(inherited_world_write, PathRole::Destination));
}

#[test]
fn drive_admission_requires_a_fixed_volume_with_a_registered_drive_letter() {
    use super::utils::drive_is_registered;
    let registered: Vec<u16> = "C:\\\0D:\\Mount\\\0\0".encode_utf16().collect();
    assert!(drive_is_registered("c:\\", &registered));
    assert!(!drive_is_registered("Z:\\", &registered));
    assert!(!drive_is_registered("S:\\", &[]));
}
