use super::{
    messages::{NO_EXE, NOT_UTF8, USAGE},
    types::Command,
    utils::quoted_launch_path,
};
use std::ffi::OsString;

impl Command {
    pub fn parse(args: &[OsString]) -> Result<Self, &'static str> {
        match args {
            [] => Ok(Self::Run),
            [flag] if flag == "--enable-autostart" => Ok(Self::Enable),
            [flag] if flag == "--disable-autostart" => Ok(Self::Disable),
            _ => Err(USAGE),
        }
    }
}

/// The quoted path a login entry starts this notifier with.
pub fn launch_path() -> Result<String, &'static str> {
    let path = std::env::current_exe().map_err(|_| NO_EXE)?;
    quoted_launch_path(path.to_str().ok_or(NOT_UTF8)?)
}
