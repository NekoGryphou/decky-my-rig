use std::{io, net::Ipv4Addr};
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};

/// Netmask for an on-link prefix length. Longer prefixes are clamped.
pub(super) fn netmask(prefix: u8) -> Ipv4Addr {
    Ipv4Addr::from(
        u32::MAX
            .checked_shl(32 - u32::from(prefix.min(32)))
            .unwrap_or(0),
    )
}

/// Calls `query` with a buffer and its size in bytes until it fits. The adapter
/// list can grow between calls, so a too-small answer is retried with the size
/// it asked for; endless growth is an error, not a loop.
pub(super) fn fill_adapter_buffer(
    mut query: impl FnMut(&mut [u64], &mut u32) -> u32,
) -> io::Result<Vec<u64>> {
    // ? u64 words keep the adapter records aligned.
    let mut buffer: Vec<u64> = vec![0; 2048];
    for _ in 0..MAX_ADAPTER_QUERIES {
        let mut size = (buffer.len() * 8) as u32;
        let result = query(&mut buffer, &mut size);
        if result == ERROR_BUFFER_OVERFLOW.0 {
            buffer = vec![0; (size as usize).div_ceil(8)];
        } else if result == ERROR_SUCCESS.0 {
            return Ok(buffer);
        } else {
            return Err(io::Error::from_raw_os_error(result as i32));
        }
    }
    Err(io::Error::from_raw_os_error(ERROR_BUFFER_OVERFLOW.0 as i32))
}

/// A list that outgrows its buffer this many times in a row isn't settling.
pub(super) const MAX_ADAPTER_QUERIES: usize = 5;
