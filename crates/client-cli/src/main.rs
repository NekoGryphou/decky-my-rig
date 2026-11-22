//! `myrig`: the client, for this device's platform.
use std::{process::ExitCode, sync::Arc};

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    let platform = Arc::new(myrig_client_linux::Linux);
    #[cfg(windows)]
    let platform = Arc::new(myrig_client_windows::Windows);
    myrig_client::cli::main(platform)
}
