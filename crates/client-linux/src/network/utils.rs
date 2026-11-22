pub(super) fn can_broadcast(flags: u32) -> bool {
    let required = (libc::IFF_UP | libc::IFF_BROADCAST) as u32;
    let refused = (libc::IFF_LOOPBACK | libc::IFF_POINTOPOINT) as u32;
    flags & required == required && flags & refused == 0
}
