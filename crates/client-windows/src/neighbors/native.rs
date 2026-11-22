use super::utils::ethernet_mac;
use std::{io, net::Ipv4Addr};
use windows::Win32::{
    Foundation::ERROR_SUCCESS,
    NetworkManagement::IpHelper::{FreeMibTable, GetIpNetTable2, MIB_IPNET_ROW2, MIB_IPNET_TABLE2},
    Networking::WinSock::{AF_INET, NlnsDelay, NlnsPermanent, NlnsProbe, NlnsReachable, NlnsStale},
};

/// The hardware address of each resolved IPv4 neighbor entry for `host`.
pub(crate) fn neighbor_macs(host: Ipv4Addr) -> io::Result<Vec<String>> {
    let mut table: *mut MIB_IPNET_TABLE2 = std::ptr::null_mut();
    // ! SAFETY: `table` is a valid out-pointer; the table is freed below.
    let result = unsafe { GetIpNetTable2(AF_INET, &mut table) };
    if result != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(result.0 as i32));
    }
    // ! SAFETY: GetIpNetTable2 succeeded, so `table` holds `NumEntries` rows.
    let rows = unsafe {
        std::slice::from_raw_parts(
            std::ptr::addr_of!((*table).Table).cast::<MIB_IPNET_ROW2>(),
            (*table).NumEntries as usize,
        )
    };
    let resolved = [
        NlnsReachable,
        NlnsStale,
        NlnsDelay,
        NlnsProbe,
        NlnsPermanent,
    ];
    let macs = rows
        .iter()
        .filter(|row| resolved.contains(&row.State))
        // ! SAFETY: rows of an AF_INET table hold IPv4 addresses.
        .filter(|row| unsafe { row.Address.Ipv4.sin_addr.S_un.S_addr } == u32::from(host).to_be())
        .filter_map(|row| {
            let length = (row.PhysicalAddressLength as usize).min(row.PhysicalAddress.len());
            ethernet_mac(&row.PhysicalAddress[..length])
        })
        .collect();
    // ! SAFETY: `table` came from GetIpNetTable2 and is freed exactly once.
    unsafe { FreeMibTable(table.cast()) };
    Ok(macs)
}
