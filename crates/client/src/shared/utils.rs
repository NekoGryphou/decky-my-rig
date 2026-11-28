//! Input normalization. Errors never echo the values passed in.
use super::{
    errors::PublicError,
    messages::{
        CREDENTIALS_INVALID, HOST_INVALID, HOST_NOT_UNICAST, IDENTIFIER_INVALID, MAC_INVALID,
        MAC_NOT_UNICAST, NAME_INVALID, PORT_INVALID,
    },
};
use myrig_protocol::validation::{canonical_uuid, decode_hex, valid_name};
use std::{
    io::{self, Read},
    net::Ipv4Addr,
};
use uuid::Uuid;

/// Trims surrounding whitespace. Control characters are rejected before
/// trimming, so a trailing newline can't quietly disappear.
fn text<'a>(value: &'a str, invalid: &'static str) -> Result<&'a str, PublicError> {
    if value.chars().any(char::is_control) {
        return Err(PublicError(invalid));
    }
    Ok(value.trim())
}

pub fn normalize_name(value: &str) -> Result<String, PublicError> {
    let name = text(value, NAME_INVALID)?;
    if !valid_name(name) {
        return Err(PublicError(NAME_INVALID));
    }
    Ok(name.to_owned())
}

pub fn normalize_port(value: &str) -> Result<u16, PublicError> {
    let digits = text(value, PORT_INVALID)?;
    if !(1..=5).contains(&digits.len()) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(PublicError(PORT_INVALID));
    }
    match digits.parse::<u16>() {
        Ok(port) if port != 0 => Ok(port),
        _ => Err(PublicError(PORT_INVALID)),
    }
}

/// Accepts twelve hex digits, or six pairs separated all by colons or all by
/// dashes. Returns uppercase pairs joined by colons.
pub fn normalize_mac(value: &str) -> Result<String, PublicError> {
    let mac = text(value, MAC_INVALID)?;
    let digits: String = match mac.len() {
        12 => mac.to_owned(),
        17 => {
            let separator = mac.as_bytes()[2];
            let separated = (separator == b':' || separator == b'-')
                && mac
                    .bytes()
                    .enumerate()
                    .all(|(index, b)| (index % 3 == 2) == (b == separator));
            if !separated {
                return Err(PublicError(MAC_INVALID));
            }
            mac.chars()
                .filter(|&c| c != char::from(separator))
                .collect()
        }
        _ => return Err(PublicError(MAC_INVALID)),
    };
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(PublicError(MAC_INVALID));
    }
    let octets = decode_hex::<6>(&digits.to_ascii_lowercase()).ok_or(PublicError(MAC_INVALID))?;
    if octets == [0; 6] || octets[0] & 1 == 1 {
        return Err(PublicError(MAC_NOT_UNICAST));
    }
    Ok(octets
        .iter()
        .map(|octet| format!("{octet:02X}"))
        .collect::<Vec<_>>()
        .join(":"))
}

/// Only plain dotted-decimal, so a typo never turns into a DNS lookup.
pub fn normalize_host(value: &str) -> Result<Ipv4Addr, PublicError> {
    let host = text(value, HOST_INVALID)?;
    let address = host
        .parse::<Ipv4Addr>()
        .ok()
        .filter(|address| address.to_string() == host)
        .ok_or(PublicError(HOST_INVALID))?;
    if address.is_unspecified() || address.is_multicast() || address.is_broadcast() {
        return Err(PublicError(HOST_NOT_UNICAST));
    }
    Ok(address)
}

pub fn normalize_token(value: &str) -> Result<String, PublicError> {
    let token = text(value, CREDENTIALS_INVALID)?;
    decode_hex::<32>(token).ok_or(PublicError(CREDENTIALS_INVALID))?;
    Ok(token.to_owned())
}

pub fn normalize_uuid(value: &str) -> Result<Uuid, PublicError> {
    canonical_uuid(text(value, IDENTIFIER_INVALID)?).ok_or(PublicError(IDENTIFIER_INVALID))
}

/// Reads at most `limit` bytes; one more byte means the file is too big and
/// fails with `InvalidData` instead of being read whole.
pub fn read_bounded(file: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut raw = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut raw)?;
    if raw.len() > limit {
        return Err(io::ErrorKind::InvalidData.into());
    }
    Ok(raw)
}
