use std::{io, net::Ipv4Addr, ptr};

/// (flags, address, netmask) of every IPv4 interface address.
pub(super) fn ipv4_addresses() -> io::Result<Vec<(u32, Ipv4Addr, Ipv4Addr)>> {
    let mut list: *mut libc::ifaddrs = ptr::null_mut();
    // ! SAFETY: getifaddrs writes a list head that is released below.
    if unsafe { libc::getifaddrs(&mut list) } != 0 {
        return Err(io::Error::last_os_error());
    }
    let mut found = Vec::new();
    let mut entry = list;
    while !entry.is_null() {
        // ! SAFETY: entries come from getifaddrs and live until freeifaddrs.
        let current = unsafe { &*entry };
        if let (Some(address), Some(netmask)) = (ipv4(current.ifa_addr), ipv4(current.ifa_netmask))
        {
            found.push((current.ifa_flags, address, netmask));
        }
        entry = current.ifa_next;
    }
    // ! SAFETY: `list` came from a successful getifaddrs and is freed once.
    unsafe { libc::freeifaddrs(list) };
    Ok(found)
}

fn ipv4(address: *const libc::sockaddr) -> Option<Ipv4Addr> {
    if address.is_null() {
        return None;
    }
    // ! SAFETY: a non-null sockaddr from getifaddrs; it is read as sockaddr_in only after its
    // ! family says it is one.
    unsafe {
        if i32::from((*address).sa_family) != libc::AF_INET {
            return None;
        }
        let address = &*address.cast::<libc::sockaddr_in>();
        Some(Ipv4Addr::from(u32::from_be(address.sin_addr.s_addr)))
    }
}
