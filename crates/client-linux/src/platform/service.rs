use super::{consts::HOSTNAME_PATH, messages::NO_HOME, types::Linux, utils::config_dir_from};
use crate::{neighbors::neighbor_macs, network::broadcast_interfaces, settings::LinuxSettings};
use myrig_client::platform::{Platform, Settings};
use std::{
    io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

impl Platform for Linux {
    fn settings_directory(&self) -> Result<PathBuf, &'static str> {
        config_dir_from(
            std::env::var_os("XDG_CONFIG_HOME"),
            std::env::var_os("HOME"),
        )
        .ok_or(NO_HOME)
    }
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>> {
        Ok(Box::new(LinuxSettings::open(directory, create)?))
    }
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
        broadcast_interfaces()
    }
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>> {
        neighbor_macs(host)
    }
    fn hostname(&self) -> Option<String> {
        std::fs::read_to_string(HOSTNAME_PATH).ok()
    }
}
