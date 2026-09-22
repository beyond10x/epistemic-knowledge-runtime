//! Strict decoding primitives shared by live typed input carriers.

use std::{collections::BTreeMap, fmt, marker::PhantomData};

use serde::de::{self, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

/// Decode a map without allowing a later spelling of a key to replace its value.
///
/// Uniqueness is checked after decoding the actual key type and **before** decoding
/// its value. In particular, two textual spellings of one identity are duplicates.
/// This does not impose resource limits; the ingress decoder owns those limits.
///
/// # Errors
/// Refuses invalid keys/values and any duplicate decoded key.
pub fn unique_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    struct Unique<K, V>(PhantomData<(K, V)>);
    impl<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>> Visitor<'de> for Unique<K, V> {
        type Value = BTreeMap<K, V>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a mapping with unique decoded keys")
        }
        fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
            let mut entries = BTreeMap::new();
            while let Some(key) = input.next_key()? {
                if entries.contains_key(&key) {
                    return Err(de::Error::custom("duplicate decoded map key"));
                }
                entries.insert(key, input.next_value()?);
            }
            Ok(entries)
        }
    }
    deserializer.deserialize_map(Unique(PhantomData))
}
