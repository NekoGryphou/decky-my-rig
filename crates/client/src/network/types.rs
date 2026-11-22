use std::net::Ipv4Addr;

/// An IPv4 address of an interface that's up and can broadcast, with its prefix
/// length.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interface {
    pub address: Ipv4Addr,
    pub prefix: u8,
}
