use std::{ffi::OsString, path::Path};

fn main() -> std::process::ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    if let [command, path] = args.as_slice()
        && command == "check-install-dir"
    {
        return match myrig_host_windows::install_directory::InstallDirectory::validate(Path::new(
            path,
        )) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    #[cfg(windows)]
    if let [command, folders @ ..] = args.as_slice()
        && command == "check-data"
        && !folders.is_empty()
    {
        for folder in folders {
            if let Err(error) =
                myrig_host_windows::install_directory::InstallDirectory::validate_data_folder(
                    Path::new(folder),
                )
            {
                eprintln!("{error}");
                return std::process::ExitCode::FAILURE;
            }
        }
        return std::process::ExitCode::SUCCESS;
    }
    #[cfg(windows)]
    if let [command, directories @ ..] = args.as_slice()
        && command == "create-data"
        && !directories.is_empty()
    {
        let paths: Vec<&Path> = directories.iter().map(Path::new).collect();
        if let Err(error) =
            myrig_host_windows::protected_directory::create_protected_directories(&paths)
        {
            eprintln!("Cannot create the My Rig data folders: {error}");
            return std::process::ExitCode::FAILURE;
        }
        return std::process::ExitCode::SUCCESS;
    }
    let [command, config, security, port] = args.as_slice() else {
        eprintln!("Usage: myrig-setup init <host.json> <host-security.json> <port>");
        return std::process::ExitCode::FAILURE;
    };
    if command != "init" {
        eprintln!("Unknown setup command.");
        return std::process::ExitCode::FAILURE;
    }
    let Some(port) = port.to_str().and_then(|value| value.parse::<u16>().ok()) else {
        eprintln!("Invalid host port.");
        return std::process::ExitCode::FAILURE;
    };
    match myrig_setup::initialize(Path::new(config), Path::new(security), port) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
