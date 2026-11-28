use super::consts::{EM_AARCH64, EM_X86_64, PE_AARCH64, PE_X86_64};
use super::stamp::{check_stamp, input_hash, write_stamp};
use super::utils::{
    crate_dependencies, deterministic_zip, hex_sha256, is_pe, is_static_elf, reject_credentials,
    release_files, valid_version, write_bin,
};
use serde_json::json;
use std::{collections::BTreeMap, path::Path, process::Command};

fn root() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

#[test]
fn notice_scope_excludes_unrelated_and_test_only_crates() {
    let metadata = json!({
        "packages": [
            {"id": "client", "manifest_path": "/repo/crates/client/Cargo.toml"},
            {"id": "crypto", "manifest_path": "/registry/crypto/Cargo.toml"},
            {"id": "host-only", "manifest_path": "/registry/host-only/Cargo.toml"},
            {"id": "test-only", "manifest_path": "/registry/test-only/Cargo.toml"},
        ],
        "resolve": {"nodes": [
            {"id": "client", "deps": [
                {"pkg": "crypto", "dep_kinds": [{"kind": null}]},
                {"pkg": "test-only", "dep_kinds": [{"kind": "dev"}]},
            ]},
            {"id": "test-only", "deps": []},
            {"id": "crypto", "deps": []},
            {"id": "host-only", "deps": []},
        ]},
    });
    let mut ids: Vec<&str> =
        crate_dependencies(&metadata, Path::new("/repo/crates/client/Cargo.toml"))
            .unwrap()
            .into_iter()
            .map(|package| package["id"].as_str().unwrap())
            .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["client", "crypto"]);
    assert!(crate_dependencies(&metadata, Path::new("/elsewhere/Cargo.toml")).is_err());
}

#[test]
fn only_static_x86_64_executables_are_accepted() {
    let musl = root().join("out/cargo/x86_64-unknown-linux-musl/release/myrig");
    if let Ok(data) = std::fs::read(musl) {
        assert!(is_static_elf(&data, EM_X86_64));
        assert!(!is_static_elf(&data, EM_AARCH64));
    }
    // - This test binary is linked against the system loader.
    let own = std::fs::read(std::env::current_exe().unwrap()).unwrap();
    assert!(!is_static_elf(&own, EM_X86_64));
    assert!(!is_static_elf(b"#!/bin/sh\n", EM_X86_64));
    assert!(!is_static_elf(&[], EM_X86_64));
}

#[test]
fn credential_material_is_refused() {
    for bad in [
        "-----BEGIN PRIVATE KEY-----",
        "run sshpass -p x",
        "see https://user:secret@example.com/x",
        "HTTP://User:Secret@host",
    ] {
        assert!(reject_credentials("file", bad.as_bytes()).is_err(), "{bad}");
    }
    for good in [
        "https://example.com/a:b@c",
        "mail user@example.com",
        "https://host:8080/path",
        "http://:x@host",
    ] {
        assert!(
            reject_credentials("file", good.as_bytes()).is_ok(),
            "{good}"
        );
    }
}

#[test]
fn versions_are_plain_tokens() {
    assert!(valid_version("0.1.0"));
    assert!(valid_version("1.2.3-rc.1+build"));
    assert!(!valid_version(""));
    assert!(!valid_version("1.0/../x"));
    assert!(!valid_version("1 0"));
}

#[test]
fn sha256_is_lowercase_hex() {
    assert_eq!(
        hex_sha256(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_archive_is_deterministic_and_keeps_modes() {
    let payload = BTreeMap::from([
        (
            "bin/myrig-x86_64-linux".to_owned(),
            b"\x7fELF binary".to_vec(),
        ),
        (
            "bin/THIRD-PARTY-NOTICES.txt".to_owned(),
            b"notices".to_vec(),
        ),
        ("main.py".to_owned(), b"print('hi')\n".repeat(50)),
    ]);
    let archive = deterministic_zip("DeckyMyRig/", &payload).unwrap();
    assert_eq!(archive, deterministic_zip("DeckyMyRig/", &payload).unwrap());
    let directory = tempfile_directory();
    let path = directory.join("plugin.zip");
    std::fs::write(&path, &archive).unwrap();
    // - Check the format with an independent reader.
    let listing = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import sys,zipfile\n\
             z=zipfile.ZipFile(sys.argv[1]); assert z.testzip() is None\n\
             for i in z.infolist(): print(i.filename, oct(i.external_attr>>16), i.date_time[:3], len(z.read(i)))",
        ])
        .arg(&path)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&directory).unwrap();
    assert!(
        listing.status.success(),
        "{}",
        String::from_utf8_lossy(&listing.stderr)
    );
    assert_eq!(
        String::from_utf8(listing.stdout).unwrap(),
        "DeckyMyRig/bin/THIRD-PARTY-NOTICES.txt 0o100644 (1980, 1, 1) 7\n\
         DeckyMyRig/bin/myrig-x86_64-linux 0o100755 (1980, 1, 1) 11\n\
         DeckyMyRig/main.py 0o100644 (1980, 1, 1) 600\n"
    );
}

#[test]
fn an_archive_beyond_the_plain_zip_limits_is_refused() {
    let payload: BTreeMap<String, Vec<u8>> = (0..=u32::from(u16::MAX))
        .map(|index| (format!("{index}"), Vec::new()))
        .collect();
    assert!(deterministic_zip("R/", &payload).is_err());
}

#[test]
fn the_plugin_ships_its_license_and_the_frontend_notices() {
    let files = release_files(root()).unwrap();
    for required in ["LICENSE", "dist/THIRD-PARTY-NOTICES.txt"] {
        assert!(files.iter().any(|file| file == required), "{required}");
    }
}

#[test]
fn the_released_python_imports_without_the_checkout() {
    let source = root();
    let files = release_files(source).unwrap();
    assert!(
        files
            .iter()
            .any(|file| file == "py_modules/my_rig/agent/agent.py")
    );
    let payload = tempfile_directory();
    for relative in files.iter().filter(|file| file.ends_with(".py")) {
        let target = payload.join(relative);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::copy(source.join(relative), target).unwrap();
    }
    let result = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import runpy,sys,types; sys.modules['decky']=types.ModuleType('decky'); runpy.run_path(sys.argv[1])",
        ])
        .arg(payload.join("main.py"))
        .current_dir(&payload)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&payload).unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn agents_land_in_the_store_bin_folder_without_the_prefix() {
    let out = tempfile_directory();
    std::fs::create_dir_all(&out).unwrap();
    let payload = BTreeMap::from([
        ("bin/myrig-x86_64-linux".to_owned(), b"agent".to_vec()),
        (
            "bin/THIRD-PARTY-NOTICES.txt".to_owned(),
            b"notices".to_vec(),
        ),
    ]);
    write_bin(&out, &payload).unwrap();
    assert_eq!(
        std::fs::read(out.join("myrig-x86_64-linux")).unwrap(),
        b"agent"
    );
    assert_eq!(
        std::fs::read(out.join("THIRD-PARTY-NOTICES.txt")).unwrap(),
        b"notices"
    );
    let outside = BTreeMap::from([("main.py".to_owned(), Vec::new())]);
    assert!(write_bin(&out, &outside).is_err());
    std::fs::remove_dir_all(&out).unwrap();
}

#[test]
fn bin_files_are_written_into_a_missing_output_directory() {
    let scratch = tempfile_directory();
    let out = scratch.join("not/yet/created");
    let payload = BTreeMap::from([("bin/myrig-x86_64-linux".to_owned(), b"agent".to_vec())]);
    write_bin(&out, &payload).unwrap();
    assert_eq!(
        std::fs::read(out.join("myrig-x86_64-linux")).unwrap(),
        b"agent"
    );
    std::fs::remove_dir_all(&scratch).unwrap();
}

/// A workspace folder holding the files an agent's stamp covers, with one crate
/// and a file outside the agent's inputs.
fn workspace() -> (std::path::PathBuf, std::path::PathBuf) {
    let root = tempfile_directory();
    let crate_dir = root.join("crates/app");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::create_dir_all(crate_dir.join("tests")).unwrap();
    for (path, text) in [
        ("Cargo.toml", "[workspace]\n"),
        ("Cargo.lock", "# lock\n"),
        ("crates/app/Cargo.toml", "[package]\n"),
        ("crates/app/src/lib.rs", "pub fn a() {}\n"),
        ("crates/app/tests/it.rs", "#[test] fn t() {}\n"),
    ] {
        std::fs::write(root.join(path), text).unwrap();
    }
    (root, crate_dir)
}

fn set_mtime(path: &Path, seconds: u64) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(seconds))
        .unwrap();
}

#[test]
fn an_edit_that_keeps_the_mtime_makes_the_agent_stale() {
    let (root, crate_dir) = workspace();
    let source = crate_dir.join("src/lib.rs");
    let crates = [crate_dir.clone()];
    let binary = root.join("myrig");
    set_mtime(&source, 100);
    write_stamp(&binary, &input_hash(&root, &crates).unwrap()).unwrap();
    std::fs::write(&source, "pub fn b() {}\n").unwrap();
    set_mtime(&source, 100);
    assert!(check_stamp(&binary, &input_hash(&root, &crates).unwrap()).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_newer_mtime_with_the_same_content_is_not_stale() {
    let (root, crate_dir) = workspace();
    let source = crate_dir.join("src/lib.rs");
    let crates = [crate_dir.clone()];
    let binary = root.join("myrig");
    write_stamp(&binary, &input_hash(&root, &crates).unwrap()).unwrap();
    set_mtime(&source, 9_999_999);
    assert!(check_stamp(&binary, &input_hash(&root, &crates).unwrap()).is_ok());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_stamp_or_a_new_input_file_is_stale() {
    let (root, crate_dir) = workspace();
    let crates = [crate_dir.clone()];
    let binary = root.join("myrig");
    let hash = input_hash(&root, &crates).unwrap();
    assert!(check_stamp(&binary, &hash).is_err());
    write_stamp(&binary, &hash).unwrap();
    assert!(check_stamp(&binary, &hash).is_ok());
    std::fs::write(crate_dir.join("src/extra.rs"), "").unwrap();
    assert!(check_stamp(&binary, &input_hash(&root, &crates).unwrap()).is_err());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn files_outside_the_agent_inputs_do_not_change_the_hash() {
    let (root, crate_dir) = workspace();
    let crates = [crate_dir.clone()];
    let before = input_hash(&root, &crates).unwrap();
    std::fs::write(crate_dir.join("tests/it.rs"), "#[test] fn u() {}\n").unwrap();
    assert_eq!(before, input_hash(&root, &crates).unwrap());
    std::fs::remove_dir_all(&root).unwrap();
}

fn tempfile_directory() -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let next = NEXT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let directory = root().join(format!("out/xtask-test-{}-{next}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

/// The smallest header `is_pe` reads: "MZ", the PE offset, "PE\0\0" and the
/// machine type.
fn pe_header(machine: u16) -> Vec<u8> {
    let mut data = vec![0u8; 0x48];
    data[..2].copy_from_slice(b"MZ");
    data[0x3c..0x40].copy_from_slice(&0x40u32.to_le_bytes());
    data[0x40..0x44].copy_from_slice(b"PE\0\0");
    data[0x44..0x46].copy_from_slice(&machine.to_le_bytes());
    data
}

#[test]
fn windows_agents_must_be_pe_files_for_their_processor() {
    assert!(is_pe(&pe_header(PE_X86_64), PE_X86_64));
    assert!(is_pe(&pe_header(PE_AARCH64), PE_AARCH64));
    assert!(!is_pe(&pe_header(PE_X86_64), PE_AARCH64));
    assert!(!is_pe(b"\x7fELF", PE_X86_64));
    assert!(!is_pe(&pe_header(PE_X86_64)[..0x44], PE_X86_64));
}

#[cfg(unix)]
#[test]
fn the_hash_does_not_depend_on_how_the_same_folders_are_spelled() {
    // ! A symlink gives the same folders two spellings. Their hashes must match, or
    //   a Windows runner (verbatim paths) and a Linux one would disagree about one tree.
    let (root, crate_dir) = workspace();
    let link = root.with_file_name(format!(
        "{}-link",
        root.file_name().unwrap().to_string_lossy()
    ));
    std::os::unix::fs::symlink(&root, &link).unwrap();
    {
        // ? The root is canonical, as the stamp makes it; the crate folder comes from cargo's metadata.
        let canonical_root = root.canonicalize().unwrap();
        let plain =
            super::stamp::input_hash(&canonical_root, std::slice::from_ref(&crate_dir)).unwrap();
        let spelled =
            super::stamp::input_hash(&canonical_root, &[link.join("crates/app")]).unwrap();
        assert_eq!(plain, spelled);
        std::fs::remove_file(&link).unwrap();
    }
    std::fs::remove_dir_all(&root).unwrap();
}
