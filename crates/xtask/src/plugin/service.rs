use super::{
    consts::{AGENTS, ARCHIVE_ROOT, Agent, Build},
    stamp::{check_stamp, input_hash, stamp_path, write_stamp},
    utils::{
        crate_dependencies, deterministic_zip, hex_sha256, is_pe, is_static_elf,
        reject_credentials, release_files, valid_version, write_bin,
    },
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

/// Builds `out/build/decky-plugin/myrig-plugin-v<version>.zip` and a manifest
/// of file hashes: the frontend, the Python adapter and a static
/// `bin/myrig-<arch>-<os>` for each built platform, plus the notices of every
/// crate in them. `windows` lists the Windows architectures whose agents
/// (already built on Windows) go in too. The Linux agents are built here unless
/// `prebuilt_linux` is set, because CI's Linux job already built them into
/// out/cargo.
pub fn build(windows: &[String], prebuilt_linux: bool) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let source = root.clone();
    run(Command::new("pnpm")
        .args(["run", "build"])
        .current_dir(&source))?;
    let package: Value = serde_json::from_slice(&read(&source.join("package.json"))?)
        .map_err(|error| format!("package.json: {error}"))?;
    let version = package["version"]
        .as_str()
        .filter(|version| valid_version(version))
        .ok_or("package.json has an invalid version")?;

    let mut payload = BTreeMap::new();
    for relative in release_files(&source)? {
        let path = source.join(&relative);
        if !path.is_file() {
            return Err(format!("required release file is missing: {relative}"));
        }
        payload.insert(relative, read(&path)?);
    }
    payload.extend(agent_payload(&root, windows, prebuilt_linux)?);
    for (relative, data) in &payload {
        reject_credentials(relative, data)?;
    }

    let out = root.join("out/build/decky-plugin");
    fs::create_dir_all(&out).map_err(|error| error.to_string())?;
    let name = format!("myrig-plugin-v{version}.zip");
    let archive = deterministic_zip(ARCHIVE_ROOT, &payload)?;
    let files: serde_json::Map<String, Value> = payload
        .iter()
        .map(|(relative, data)| (relative.clone(), json!(hex_sha256(data))))
        .collect();
    let manifest = json!({
        "archive": name,
        "archive_root": ARCHIVE_ROOT,
        "archive_sha256": hex_sha256(&archive),
        "file_count": payload.len(),
        "files": files,
        "version": version,
    });
    write(&out.join(&name), &archive)?;
    let manifest = serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    write(
        &out.join(format!("myrig-plugin-v{version}.manifest.json")),
        format!("{manifest}\n").as_bytes(),
    )?;
    println!("{}", out.join(name).display());
    Ok(())
}

/// Builds the Linux agents and their notices into `out`. Decky's store runs
/// this from `backend/entrypoint.sh` and ships `out` as the plugin's `bin/`.
pub fn agents(out: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    write_bin(out, &agent_payload(&root, &[], false)?)
}

/// Writes the stamp of an agent built by a script outside xtask, such as
/// `tools/build_linux.sh`. The plugin build refuses an agent without one.
pub fn stamp(binary: &Path) -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    fs::metadata(binary).map_err(|error| format!("{}: {error}", binary.display()))?;
    let hash = input_hash(&root, &own_crate_dirs(&root)?)?;
    write_stamp(binary, &hash)?;
    println!("{}", stamp_path(binary).display());
    Ok(())
}

/// The static agents and the notices of every crate compiled into them.
fn agent_payload(
    root: &Path,
    windows: &[String],
    prebuilt_linux: bool,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let manifest = root.join("crates/client-cli/Cargo.toml");
    let selected: Vec<&Agent> = AGENTS
        .iter()
        .filter(|agent| match agent.build {
            Build::Linux(_) => true,
            Build::Windows(_) => windows.iter().any(|arch| arch == agent.arch),
        })
        .collect();
    // - One notices file covers every agent: all their crates together, from one
    //   metadata run for all the selected targets.
    let metadata = cargo_metadata(root, &selected)?;
    let mut packages = BTreeMap::new();
    for package in crate_dependencies(&metadata, &manifest)? {
        if package["source"].is_null() {
            continue;
        }
        let key = (
            text(&package["name"]).to_owned(),
            text(&package["version"]).to_owned(),
        );
        packages.entry(key).or_insert_with(|| package.clone());
    }
    let inputs = input_hash(root, &own_crate_dirs(root)?)?;
    let target = root.join("out/cargo");
    let mut payload = BTreeMap::new();
    for agent in &selected {
        let (name, binary, path) = match agent.build {
            Build::Linux(machine) => {
                let path = target.join(agent.target).join("release/myrig");
                if !prebuilt_linux {
                    run(cargo_command(root)
                        .args([
                            "build",
                            "--locked",
                            "--offline",
                            "--release",
                            "--bin",
                            "myrig",
                        ])
                        .args(["--target", agent.target, "--manifest-path"])
                        .arg(&manifest))?;
                    write_stamp(&path, &inputs)?;
                }
                let binary = read(&path)?;
                if !is_static_elf(&binary, machine) {
                    return Err(format!(
                        "myrig for {} is not a static executable",
                        agent.target
                    ));
                }
                (format!("bin/myrig-{}-linux", agent.arch), binary, path)
            }
            Build::Windows(machine) => {
                let path = target.join(agent.target).join("release/myrig.exe");
                let binary = read(&path).map_err(|error| {
                    format!("{error}: build it on Windows with tools/windows/build_windows.ps1")
                })?;
                if !is_pe(&binary, machine) {
                    return Err(format!(
                        "{} is not a {} executable",
                        path.display(),
                        agent.target
                    ));
                }
                (
                    format!("bin/myrig-{}-windows.exe", agent.arch),
                    binary,
                    path,
                )
            }
        };
        check_stamp(&path, &inputs)?;
        payload.insert(name, binary);
    }
    let mut notices = String::from(
        "Decky My Rig agent (myrig) dependency notices\n\
         Includes build-time and optional dependency notices conservatively.\n",
    );
    for package in packages.values() {
        notices.push_str(&package_notice(package)?);
    }
    payload.insert(
        "bin/THIRD-PARTY-NOTICES.txt".to_owned(),
        notices.into_bytes(),
    );
    Ok(payload)
}

/// The cargo command that builds into the shared `out/cargo` folder. The musl
/// targets get their own subfolders there.
fn cargo_command(root: &Path) -> Command {
    let mut command = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    command.env("CARGO_TARGET_DIR", root.join("out/cargo"));
    command
}

/// `cargo metadata` for the client CLI, limited to the platforms of `agents`, or
/// every platform when `agents` is empty.
fn cargo_metadata(root: &Path, agents: &[&Agent]) -> Result<Value, String> {
    let mut command = cargo_command(root);
    command.args(["metadata", "--locked", "--offline", "--format-version", "1"]);
    for agent in agents {
        command.args(["--filter-platform", agent.target]);
    }
    let output = command
        .arg("--manifest-path")
        .arg(root.join("crates/client-cli/Cargo.toml"))
        .output()
        .map_err(|error| format!("cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|error| format!("cargo metadata: {error}"))
}

/// The folders of every workspace crate. Stamping needs only these, so it asks
/// for no dependency: resolving every platform's dependencies needs packages a
/// Windows-only checkout never downloads, and `--offline` then fails.
fn own_crate_dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let output = cargo_command(root)
        .args([
            "metadata",
            "--locked",
            "--offline",
            "--no-deps",
            "--format-version",
            "1",
        ])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .output()
        .map_err(|error| format!("cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata = serde_json::from_slice::<Value>(&output.stdout)
        .map_err(|error| format!("cargo metadata: {error}"))?;
    Ok(metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|package| {
            Path::new(text(&package["manifest_path"]))
                .parent()
                .map(Path::to_path_buf)
        })
        .collect())
}

fn package_notice(package: &Value) -> Result<String, String> {
    let (name, version) = (text(&package["name"]), text(&package["version"]));
    let license = text(&package["license"]);
    let directory = Path::new(text(&package["manifest_path"]))
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let mut licenses: Vec<PathBuf> = fs::read_dir(&directory)
        .map_err(|error| format!("{}: {error}", directory.display()))?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| {
            let file = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            let upper = file.to_ascii_uppercase();
            path.is_file()
                && ["LICENSE", "COPYING", "NOTICE"]
                    .iter()
                    .any(|p| upper.starts_with(p))
        })
        .collect();
    licenses.sort();
    if license.is_empty() || licenses.is_empty() {
        return Err(format!(
            "missing myrig dependency license evidence: {name} {version}"
        ));
    }
    let mut notice = format!("\n=== {name} {version} ===\nLicense expression: {license}\n");
    for path in licenses {
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        // ? Crates pick their own encoding, and a stray byte shouldn't block a release.
        let body = String::from_utf8_lossy(&read(&path)?).replace("\r\n", "\n");
        notice.push_str(&format!("\n--- {file} ---\n{body}\n"));
    }
    Ok(notice)
}

fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}

fn run(command: &mut Command) -> Result<(), String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .map_err(|error| format!("{program}: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{program} failed: {status}"))
}

fn read(path: &Path) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("{}: {error}", path.display()))
}

fn write(path: &Path, data: &[u8]) -> Result<(), String> {
    fs::write(path, data).map_err(|error| format!("{}: {error}", path.display()))
}
