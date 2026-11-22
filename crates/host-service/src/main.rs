//! Entry point for the SCM only. There's no console server or fake power mode.
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    if myrig_host_windows::host_service::run().is_ok() {
        std::process::ExitCode::SUCCESS
    } else {
        eprintln!("My Rig host service failed; inspect SCM service status.");
        std::process::ExitCode::FAILURE
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("My Rig host runs as a Windows service.");
    std::process::ExitCode::FAILURE
}
