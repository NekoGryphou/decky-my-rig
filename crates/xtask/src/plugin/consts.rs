/// The plugin's own files. Every module in `py_modules/my_rig` is added as
/// well.
pub(super) const TOP_LEVEL_FILES: [&str; 6] = [
    "LICENSE",
    "plugin.json",
    "package.json",
    "main.py",
    "dist/index.js",
    "dist/THIRD-PARTY-NOTICES.txt",
];
pub(super) const ARCHIVE_ROOT: &str = "DeckyMyRig/";
pub(super) const EM_X86_64: u16 = 62;
pub(super) const EM_AARCH64: u16 = 183;
pub(super) const PE_X86_64: u16 = 0x8664;
pub(super) const PE_AARCH64: u16 = 0xaa64;

/// How an agent gets built and what its file has to be.
pub(super) enum Build {
    /// Built here: a static ELF for this machine type.
    Linux(u16),
    /// Built on Windows by `tools/windows/build_windows.ps1`: a PE file for
    /// this machine type.
    Windows(u16),
}

/// One agent per platform the plugin ships for: the Rust target, the
/// `<arch>-<os>` release name (`bin/myrig-<name>`, as `agent_utils.py` picks
/// it) and how it's built.
pub(super) struct Agent {
    pub(super) target: &'static str,
    pub(super) arch: &'static str,
    pub(super) build: Build,
}

pub(super) const AGENTS: [Agent; 4] = [
    Agent {
        target: "x86_64-unknown-linux-musl",
        arch: "x86_64",
        build: Build::Linux(EM_X86_64),
    },
    Agent {
        target: "aarch64-unknown-linux-musl",
        arch: "aarch64",
        build: Build::Linux(EM_AARCH64),
    },
    Agent {
        target: "x86_64-pc-windows-msvc",
        arch: "x86_64",
        build: Build::Windows(PE_X86_64),
    },
    Agent {
        target: "aarch64-pc-windows-msvc",
        arch: "aarch64",
        build: Build::Windows(PE_AARCH64),
    },
];
pub(super) const FORBIDDEN: [&str; 4] = [
    "-----BEGIN PRIVATE KEY-----",
    "-----BEGIN OPENSSH PRIVATE KEY-----",
    "sshpass",
    "SSH_ASKPASS",
];
