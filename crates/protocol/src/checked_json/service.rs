use super::types::CheckedJson;
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::Value;
use std::fmt;

// * One recursive visitor rejects duplicate keys, even in objects we don't know
//   about yet.
impl<'de> Deserialize<'de> for CheckedJson {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CheckedVisitor;
        impl<'de> Visitor<'de> for CheckedVisitor {
            type Value = CheckedJson;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("JSON with unique object keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(CheckedJson(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(CheckedJson(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(CheckedJson(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| CheckedJson(n.into()))
                    .ok_or_else(|| E::custom("Nonfinite number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(CheckedJson(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
                Ok(CheckedJson(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(CheckedJson(value)) = access.next_element()? {
                    values.push(value);
                }
                Ok(CheckedJson(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<Self::Value, A::Error> {
                let mut values = serde_json::Map::new();
                while let Some(key) = access.next_key::<String>()? {
                    if values.contains_key(&key) {
                        return Err(de::Error::custom("Duplicate object key"));
                    }
                    let CheckedJson(value) = access.next_value()?;
                    values.insert(key, value);
                }
                Ok(CheckedJson(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(CheckedVisitor)
    }
}
