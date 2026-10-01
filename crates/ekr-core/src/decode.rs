//! Strict decoding primitives shared by live typed input carriers.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    marker::PhantomData,
};

use serde::de::{self, MapAccess, SeqAccess, Visitor};
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

/// Decode a set without silently discarding a duplicate member.
///
/// # Errors
/// Refuses malformed elements and repeated decoded identities.
pub fn unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    struct Unique<T>(PhantomData<T>);
    impl<'de, T: Deserialize<'de> + Ord> Visitor<'de> for Unique<T> {
        type Value = BTreeSet<T>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a sequence of unique decoded members")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut input: A) -> Result<Self::Value, A::Error> {
            let mut result = BTreeSet::new();
            while let Some(value) = input.next_element()? {
                if !result.insert(value) {
                    return Err(de::Error::custom("duplicate decoded set member"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_seq(Unique(PhantomData))
}

/// Why [`observe_yaml`] refused a text before any value of it was decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum YamlRefusal {
    /// More bytes than the reader reads.
    TooLarge {
        /// The text's length in bytes.
        bytes: usize,
        /// The most bytes read.
        limit: usize,
    },
    /// Containers nested deeper than the reader reads.
    TooDeep {
        /// The deepest nesting read, the document's own container counting one.
        limit: usize,
    },
    /// A YAML alias (`*name`), which repeats its anchor's value; at this event of its document.
    Alias {
        /// The event's index.
        event: usize,
    },
    /// Not well-formed YAML; the loader's own message.
    Malformed(String),
}

/// The bounded observation every YAML document reader of the workspace that refuses aliases runs
/// before it decodes a value: at most `max_bytes` bytes, containers nested at most `max_depth`
/// deep, no alias, and every document well formed. The loader stops at the first container past
/// `max_depth`, so a deeper nesting costs what the limit allows, not what the input holds.
///
/// # Errors
/// The first [`YamlRefusal`], in that order within each document.
pub fn observe_yaml(text: &str, max_bytes: usize, max_depth: usize) -> Result<(), YamlRefusal> {
    use serde_yaml_ng::observation::{Documents, Event};

    let malformed = |error: &dyn fmt::Display| YamlRefusal::Malformed(error.to_string());
    if text.len() > max_bytes {
        return Err(YamlRefusal::TooLarge {
            bytes: text.len(),
            limit: max_bytes,
        });
    }
    let mut documents =
        Documents::from_str_within_depth(text, max_depth).map_err(|e| malformed(&e))?;
    while let Some(document) = documents.next_document() {
        let mut depth = 0usize;
        for at in 0..document.event_count() {
            match document.event(at).map_err(|e| malformed(&e))? {
                Some(Event::Alias { .. }) => return Err(YamlRefusal::Alias { event: at }),
                Some(Event::MappingStart(_) | Event::SequenceStart(_)) => {
                    depth += 1;
                    if depth > max_depth {
                        return Err(YamlRefusal::TooDeep { limit: max_depth });
                    }
                }
                Some(Event::MappingEnd | Event::SequenceEnd) => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        document.check().map_err(|e| malformed(&e))?;
    }
    Ok(())
}

/// A list of strings, decoded self-describingly: a number, a boolean, a null, a tagged value or
/// a null list is refused, not read as its text or as an empty list. `List<String>` means strings.
///
/// # Errors
/// Refuses anything but a sequence of strings.
pub fn strings<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<String>, D::Error> {
    struct Text(String);
    impl<'de> Deserialize<'de> for Text {
        fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
            struct Visit;
            impl Visitor<'_> for Visit {
                type Value = Text;
                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("a string")
                }
                fn visit_str<E: de::Error>(self, text: &str) -> Result<Text, E> {
                    Ok(Text(text.to_owned()))
                }
            }
            decoder.deserialize_any(Visit)
        }
    }
    struct Visit;
    impl<'de> Visitor<'de> for Visit {
        type Value = Vec<String>;
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a list of strings")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut items: A) -> Result<Vec<String>, A::Error> {
            let mut aliases = Vec::new();
            while let Some(Text(alias)) = items.next_element()? {
                aliases.push(alias);
            }
            Ok(aliases)
        }
    }
    decoder.deserialize_any(Visit)
}

/// What the refusal names, as a reader quotes it after its code.
impl fmt::Display for YamlRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge { bytes, limit } => write!(f, "{bytes} bytes, at most {limit}"),
            Self::TooDeep { limit } => write!(f, "nested more than {limit} deep"),
            Self::Alias { event } => write!(f, "event {event}"),
            Self::Malformed(message) => f.write_str(message),
        }
    }
}
