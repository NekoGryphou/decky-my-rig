//! VERSION holds the one release version, and every build reads it or has to
//! match it.
use std::{fs, path::Path};

fn read(relative: &str) -> String {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    fs::read_to_string(root.join(relative)).unwrap_or_else(|error| panic!("{relative}: {error}"))
}

#[test]
fn every_artifact_takes_its_version_from_version() {
    let version = read("VERSION").trim().to_owned();
    let parts: Vec<&str> = version.split('.').collect();
    assert_eq!(parts.len(), 3, "{version}");
    assert!(
        parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
    );

    let plugin: serde_json::Value = serde_json::from_str(&read("package.json")).unwrap();
    assert_eq!(plugin["version"], version.as_str());

    let workspace = read("Cargo.toml");
    let package = workspace.split("[workspace.package]").nth(1).unwrap();
    let declared = package
        .lines()
        .find_map(|line| line.strip_prefix("version = "))
        .unwrap();
    assert_eq!(declared.trim_matches('"'), version);
    for member in fs::read_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("..")).unwrap() {
        let manifest = member.unwrap().path().join("Cargo.toml");
        let text = fs::read_to_string(&manifest).unwrap();
        assert!(
            text.contains("\nversion.workspace = true\n"),
            "{}",
            manifest.display()
        );
    }

    assert!(read("apps/windows-app/Directory.Build.props").contains("VERSION"));
    let installer = read("packaging/windows/my_rig.iss");
    assert!(!installer.lines().any(|line| {
        line.strip_prefix("AppVersion=")
            .is_some_and(|value| value.starts_with(|c: char| c.is_ascii_digit()))
    }));
    assert!(installer.contains("myrig-app-v{#AppVersion}-{#Arch}-windows"));
}
