use std::{
    io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
};

pub trait Platform: Send + Sync + 'static {
    /// Where the CLI saves PCs, or a message saying what to set.
    fn settings_directory(&self) -> Result<PathBuf, &'static str>;
    /// Opens the private settings directory, creating it if `create` is set.
    /// Refuses a directory this user doesn't own, or one reached through a
    /// link.
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>>;
    /// IPv4 address and netmask of every interface that's up, can broadcast,
    /// and isn't loopback or point-to-point.
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>>;
    /// Hardware addresses of the complete Ethernet entries for `host` in this
    /// device's neighbor table.
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>>;
    /// This device's hostname, if it has one.
    fn hostname(&self) -> Option<String>;
}

/// An open settings directory holding `pcs.json`. A lock lasts until the value
/// is dropped.
pub trait Settings {
    /// Locks out every other writer, in any process, until dropped.
    fn lock(&self) -> io::Result<()>;
    /// The saved document, or `None` if there isn't one yet. It's an error if
    /// the document is over `limit` bytes or isn't a private regular file.
    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>>;
    /// Replaces the document so readers see either the old or the new one,
    /// never a mix.
    fn replace(&self, contents: &[u8]) -> io::Result<()>;
}
