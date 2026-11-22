use serde::{Deserialize, Deserializer};

/// A boolean that has to be there: `null` is refused instead of being treated
/// as missing.
pub(super) fn present_bool<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<bool>, D::Error> {
    bool::deserialize(deserializer).map(Some)
}
