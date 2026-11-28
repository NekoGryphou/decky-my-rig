use super::{
    messages::NO_LOCAL_APP_DATA,
    native::{Echo, hostname, local_app_data},
    types::Windows,
};
use crate::{neighbors::neighbor_macs, network::broadcast_interfaces, settings::WindowsSettings};
use myrig_client::platform::{InputEcho, Platform, SETTINGS_FOLDER, Settings};
use std::{
    io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

impl Platform for Windows {
    fn hide_input(&self) -> Option<Box<dyn InputEcho>> {
        Echo::hide().map(|echo| Box::new(echo) as Box<dyn InputEcho>)
    }
    fn settings_directory(&self) -> Result<PathBuf, &'static str> {
        local_app_data()
            .map(|base| base.join(SETTINGS_FOLDER))
            .ok_or(NO_LOCAL_APP_DATA)
    }
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>> {
        Ok(Box::new(WindowsSettings::open(directory, create)?))
    }
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
        broadcast_interfaces()
    }
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>> {
        neighbor_macs(host)
    }
    fn hostname(&self) -> Option<String> {
        hostname()
    }
}
