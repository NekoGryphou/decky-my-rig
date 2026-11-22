use super::utils::netmask;
use std::{io, net::Ipv4Addr};
use windows::Win32::{
    Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS},
    NetworkManagement::{
        IpHelper::{
            GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_MULTICAST,
            GetAdaptersAddresses, IF_TYPE_PPP, IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_TUNNEL,
            IP_ADAPTER_ADDRESSES_LH,
        },
        Ndis::IfOperStatusUp,
    },
    Networking::WinSock::{AF_INET, SOCKADDR_IN},
};

/// Address and netmask of every IPv4 address on an adapter that's up and isn't
/// loopback, PPP or a tunnel.
pub(crate) fn broadcast_interfaces() -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
    // ? u64 words keep the adapter records aligned.
    let mut buffer: Vec<u64> = vec![0; 2048];
    loop {
        let mut size = (buffer.len() * 8) as u32;
        // ! SAFETY: `buffer` holds `size` writable bytes, 8-byte aligned.
        let result = unsafe {
            GetAdaptersAddresses(
                u32::from(AF_INET.0),
                GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_DNS_SERVER,
                None,
                Some(buffer.as_mut_ptr().cast()),
                &mut size,
            )
        };
        if result == ERROR_BUFFER_OVERFLOW.0 {
            buffer = vec![0; (size as usize).div_ceil(8)];
            continue;
        }
        if result != ERROR_SUCCESS.0 {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
        break;
    }
    let mut found = Vec::new();
    let mut adapter = buffer.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    while !adapter.is_null() {
        // ! SAFETY: the list lives in `buffer`, which GetAdaptersAddresses filled.
        let current = unsafe { &*adapter };
        let eligible = current.OperStatus == IfOperStatusUp
            && ![IF_TYPE_SOFTWARE_LOOPBACK, IF_TYPE_PPP, IF_TYPE_TUNNEL].contains(&current.IfType);
        let mut unicast = current.FirstUnicastAddress;
        while eligible && !unicast.is_null() {
            // ! SAFETY: unicast entries point into `buffer`.
            let address = unsafe { &*unicast };
            let socket = address.Address.lpSockaddr;
            // ! SAFETY: a non-null address of family AF_INET is a SOCKADDR_IN.
            if !socket.is_null() && unsafe { (*socket).sa_family } == AF_INET {
                // ! SAFETY: the family check above says this is a SOCKADDR_IN.
                let ipv4 = unsafe { &*socket.cast::<SOCKADDR_IN>() };
                // ! SAFETY: every bit pattern of the address union is a valid u32.
                let raw = unsafe { ipv4.sin_addr.S_un.S_addr };
                found.push((
                    Ipv4Addr::from(u32::from_be(raw)),
                    netmask(address.OnLinkPrefixLength),
                ));
            }
            unicast = address.Next;
        }
        adapter = current.Next;
    }
    Ok(found)
}
