//! Repository tasks, run as `cargo xtask <task>`.
mod plugin;
mod version;

use std::process::ExitCode;

const USAGE: &str = "usage: cargo xtask plugin [--prebuilt-linux] [--windows x86_64,aarch64]
       cargo xtask agents <directory>
       cargo xtask stamp <agent binary>";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    if let ["agents", directory] = arguments.as_slice() {
        return finish(plugin::agents(std::path::Path::new(directory)));
    }
    if let ["stamp", binary] = arguments.as_slice() {
        return finish(plugin::stamp(std::path::Path::new(binary)));
    }
    let (prebuilt_linux, rest) = match arguments.as_slice() {
        ["plugin", "--prebuilt-linux", rest @ ..] => (true, rest),
        ["plugin", rest @ ..] => (false, rest),
        _ => (false, &["usage"][..]),
    };
    let result = match rest {
        [] => plugin::build(&[], prebuilt_linux),
        ["--windows", list] => {
            let windows: Vec<String> = list.split(',').map(str::to_owned).collect();
            if windows
                .iter()
                .all(|arch| ["x86_64", "aarch64"].contains(&arch.as_str()))
            {
                plugin::build(&windows, prebuilt_linux)
            } else {
                Err(USAGE.to_owned())
            }
        }
        _ => Err(USAGE.to_owned()),
    };
    finish(result)
}

fn finish(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}
