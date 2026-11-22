use crate::Linux;
use myrig_client::{
    store::PcChanges,
    testing::{StoreFixture, authentication, input},
};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::Path,
    sync::Arc,
};

fn fixture() -> StoreFixture {
    StoreFixture::new(Arc::new(Linux))
}

fn mode(path: &Path) -> u32 {
    fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn the_directory_and_document_are_private() {
    let fixture = fixture();
    assert_eq!(mode(&fixture.directory), 0o700);
    let pc = fixture.store.add(input("PC"), authentication()).unwrap();
    assert_eq!(mode(&fixture.file()), 0o600);
    assert_eq!(fixture.entries(), ["pcs.json"]);
    assert_eq!(
        fixture.open(&fixture.directory).unwrap().configs(),
        vec![pc]
    );
}

#[test]
fn failed_write_leaves_file_memory_and_directory_unchanged() {
    let fixture = fixture();
    let pc = fixture.store.add(input("PC"), authentication()).unwrap();
    let before = fs::read(fixture.file()).unwrap();
    fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o500)).unwrap();
    let failure = fixture.store.update(
        pc.id,
        PcChanges {
            name: Some("must not publish".to_owned()),
            ..PcChanges::default()
        },
    );
    fs::set_permissions(&fixture.directory, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(failure.unwrap_err().0, "PC configuration was not saved.");
    assert_eq!(fixture.store.configs(), vec![pc]);
    assert_eq!(fs::read(fixture.file()).unwrap(), before);
    assert_eq!(fixture.entries(), ["pcs.json"]);
}

#[test]
fn symlinks_are_not_followed_and_parents_untouched() {
    let fixture = fixture();
    let parent = fixture.directory.parent().unwrap();
    let parent_mode = mode(parent);
    let target = parent.join("elsewhere");
    fs::write(&target, "unrelated").unwrap();
    symlink(&target, fixture.file()).unwrap();
    assert!(fixture.open(&fixture.directory).is_err());
    assert_eq!(fs::read_to_string(&target).unwrap(), "unrelated");
    fs::remove_file(fixture.file()).unwrap();
    let alias = parent.join("alias");
    symlink(&fixture.directory, &alias).unwrap();
    assert!(fixture.open(&alias).is_err());
    assert_eq!(mode(parent), parent_mode);
}

#[test]
fn a_symlinked_parent_folder_is_followed() {
    // - Fedora Atomic and Bazzite make /home a link to var/home.
    let fixture = fixture();
    let parent = fixture.directory.parent().unwrap();
    let real_home = parent.join("var-home");
    fs::create_dir(&real_home).unwrap();
    let home = parent.join("home");
    symlink(&real_home, &home).unwrap();
    let store = fixture.open(&home.join("settings")).unwrap();
    store.add(input("PC"), authentication()).unwrap();
    assert!(real_home.join("settings/pcs.json").is_file());
}

#[test]
fn hard_linked_file_is_refused() {
    let fixture = fixture();
    fixture.store.add(input("PC"), authentication()).unwrap();
    fs::hard_link(
        fixture.file(),
        fixture.directory.parent().unwrap().join("copy"),
    )
    .unwrap();
    assert!(fixture.open(&fixture.directory).is_err());
}

#[test]
fn a_leftover_temporary_file_is_removed_on_the_next_save() {
    fixture().assert_saves_remove_leftovers();
}
