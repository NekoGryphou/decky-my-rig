use super::{
    consts::{ARP_CACHE, MAX_CACHE_BYTES},
    utils::complete_ethernet_entries,
};
use std::{fs::File, io, io::Read, net::Ipv4Addr, path::Path};

pub(crate) fn neighbor_macs(host: Ipv4Addr) -> io::Result<Vec<String>> {
    neighbor_macs_in(Path::new(ARP_CACHE), host)
}

pub(super) fn neighbor_macs_in(cache: &Path, host: Ipv4Addr) -> io::Result<Vec<String>> {
    let mut text = String::new();
    File::open(cache)?
        .take(MAX_CACHE_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_CACHE_BYTES {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(complete_ethernet_entries(&text, host))
}
