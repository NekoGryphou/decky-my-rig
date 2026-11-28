//! The release version is written from the tag by tools/set_version.sh, and
//! every manifest that records it has to agree.
use std::{fs, path::Path};

fn read(relative: &str) -> String {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    fs::read_to_string(root.join(relative)).unwrap_or_else(|error| panic!("{relative}: {error}"))
}

#[test]
fn every_manifest_records_the_same_version() {
    let workspace = read("Cargo.toml");
    let package = workspace.split("[workspace.package]").nth(1).unwrap();
    let declared = package
        .lines()
        .find_map(|line| line.strip_prefix("version = "))
        .unwrap()
        .trim_matches('"')
        .to_owned();
    let parts: Vec<&str> = declared.split('.').collect();
    assert_eq!(parts.len(), 3, "{declared}");
    assert!(
        parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    );

    let plugin: serde_json::Value = serde_json::from_str(&read("package.json")).unwrap();
    assert_eq!(plugin["version"], declared.as_str());
    assert!(read("apps/windows-app/Directory.Build.props").contains(&format!("<Version>{declared}</Version>")));

    for member in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("..")).unwrap() {
        let manifest = member.unwrap().path().join("Cargo.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        assert!(
            text.contains("\nversion.workspace = true\n"),
            "{}",
            manifest.display()
        );
    }

    let installer = read("packaging/windows/my_rig.iss");
    assert!(!installer.lines().any(|line| {
        line.strip_prefix("AppVersion=")
            .is_some_and(|value| value.starts_with(|c: char| c.is_ascii_digit()))
    }));
    assert!(installer.contains("myrig-app-v{#AppVersion}-{#Arch}-windows"));
}
