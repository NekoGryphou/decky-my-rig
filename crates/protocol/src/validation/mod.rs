//! Rules every side applies to names, identifiers, codes and hex values.
mod consts;
mod utils;

pub use utils::{
    canonical_uuid, decode_hex, decode_hex_bytes, lower_hex, valid_name, valid_pairing_code,
};

#[cfg(test)]
mod tests;
