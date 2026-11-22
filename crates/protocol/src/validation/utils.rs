use super::consts::{DIGITS, INVISIBLE};
use uuid::Uuid;
use zeroize::Zeroizing;

pub fn canonical_uuid(value: &str) -> Option<Uuid> {
    let parsed = Uuid::parse_str(value).ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

pub fn lower_hex(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

/// Decodes lowercase hex of 1 to `maximum` bytes. The result is wiped on drop.
pub fn decode_hex_bytes(text: &str, maximum: usize) -> Option<Zeroizing<Vec<u8>>> {
    if text.is_empty()
        || text.len() > maximum * 2
        || !text.len().is_multiple_of(2)
        || !text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    let nibble = |b: u8| if b <= b'9' { b - b'0' } else { b - b'a' + 10 };
    let mut bytes = Zeroizing::new(Vec::with_capacity(text.len() / 2));
    for pair in text.as_bytes().as_chunks::<2>().0 {
        bytes.push((nibble(pair[0]) << 4) | nibble(pair[1]));
    }
    Some(bytes)
}

/// Decodes lowercase hex of exactly `N` bytes.
pub fn decode_hex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 {
        return None;
    }
    decode_hex_bytes(text, N)?.as_slice().try_into().ok()
}

/// A pairing code: exactly eight ASCII digits.
pub fn valid_pairing_code(code: &str) -> bool {
    code.len() == 8 && code.bytes().all(|b| b.is_ascii_digit())
}

/// Names come from the network (pairing requests) and are shown to people, so
/// only visible text is allowed and one name can't be made to look like
/// another.
pub fn valid_name(name: &str) -> bool {
    name.trim() == name
        && (1..=80).contains(&name.chars().count())
        && !name
            .chars()
            .any(|c| c.is_control() || INVISIBLE.iter().any(|range| range.contains(&c)))
}
