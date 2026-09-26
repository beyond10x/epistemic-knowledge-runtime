//! Byte strings in retained JSON: one standard padded base64 string per byte string.
//!
//! The original record formats wrote a `Vec<u8>` the way serde does by default, as a JSON array
//! of decimal numbers: three to four bytes of JSON for every byte held, and the same again each
//! time a record carrying one is nested in another. The current formats write RFC 4648 § 4
//! base64 instead, about four bytes for every three.
//!
//! The decoder is strict, so a byte string has one spelling: the standard alphabet, `=` padding
//! to a multiple of four and zero bits after the last byte. Anything else is refused rather than
//! normalised, because a retained record is addressed by its bytes and two spellings of one
//! payload would be two records.
//!
//! ```
//! use ekr_core::bytes::{decode, encode};
//!
//! assert_eq!(encode(b"ekr"), "ZWty");
//! assert_eq!(decode("ZWty").unwrap(), b"ekr");
//! assert!(decode("ZWt=").is_err());
//! ```

use serde::{Deserialize, Deserializer, Serialize, Serializer};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// A string that is not the one base64 spelling of any byte string.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("not canonical standard padded base64: {0}")]
pub struct Base64Error(&'static str);

/// The standard padded base64 of `bytes`.
#[must_use]
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0_u32, |acc, (at, byte)| {
            acc | (u32::from(*byte) << (16 - 8 * at))
        });
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((triple >> (18 - 6 * position)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn sextet(symbol: u8) -> Result<u32, Base64Error> {
    Ok(u32::from(match symbol {
        b'A'..=b'Z' => symbol - b'A',
        b'a'..=b'z' => symbol - b'a' + 26,
        b'0'..=b'9' => symbol - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return Err(Base64Error("a symbol outside the standard alphabet")),
    }))
}

/// The bytes `text` spells, if it is their canonical standard padded base64.
///
/// # Errors
///
/// [`Base64Error`] for a length that is not a multiple of four, a symbol outside the alphabet,
/// padding anywhere but at the end or more than two of it, and nonzero bits after the last byte.
pub fn decode(text: &str) -> Result<Vec<u8>, Base64Error> {
    let symbols = text.as_bytes();
    let (quads, rest) = symbols.as_chunks::<4>();
    if !rest.is_empty() {
        return Err(Base64Error("a length that is not a multiple of four"));
    }
    let mut out = Vec::with_capacity(quads.len() * 3);
    let last = quads.len();
    for (index, quad) in quads.iter().enumerate() {
        let padding = quad
            .iter()
            .rev()
            .take_while(|symbol| **symbol == b'=')
            .count();
        if padding > 2 || (padding > 0 && index + 1 != last) {
            return Err(Base64Error("padding that does not end the string"));
        }
        let mut triple = 0_u32;
        for (position, symbol) in quad[..4 - padding].iter().enumerate() {
            triple |= sextet(*symbol)? << (18 - 6 * position);
        }
        let kept = 3 - padding;
        if triple & ((1 << (8 * (3 - kept))) - 1) != 0 {
            return Err(Base64Error("nonzero bits after the last byte"));
        }
        out.extend_from_slice(&triple.to_be_bytes()[1..=kept]);
    }
    Ok(out)
}

/// Serde `with` module writing a byte string as its base64 string and reading only that.
pub mod base64 {
    use super::{decode, encode, Deserialize, Deserializer, Serializer};
    use serde::de::Error as _;

    /// Writes `bytes` as one base64 string.
    ///
    /// # Errors
    ///
    /// Whatever the serializer refuses.
    pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&encode(bytes))
    }

    /// Reads one canonical base64 string.
    ///
    /// # Errors
    ///
    /// A value that is not a string, or a string that is not canonical base64.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        let text = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        decode(&text).map_err(D::Error::custom)
    }
}

/// A byte string to write in one of the two spellings a versioned format chose.
#[derive(Clone, Copy, Debug)]
pub struct Spell<'a> {
    /// The bytes.
    pub bytes: &'a [u8],
    /// `true` for the original number array, `false` for base64.
    pub numbers: bool,
}
impl Serialize for Spell<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.numbers {
            self.bytes.serialize(serializer)
        } else {
            base64::serialize(self.bytes, serializer)
        }
    }
}

/// A byte string read in whichever of the two spellings it had, and which one that was.
///
/// A reader holds the spelling against the format of the record it read, so that each format
/// admits exactly one spelling of each of its byte strings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spelled {
    /// The bytes.
    pub bytes: Vec<u8>,
    /// `true` when they were a number array, `false` when a base64 string.
    pub numbers: bool,
}
impl<'de> Deserialize<'de> for Spelled {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Either;
        impl<'de> serde::de::Visitor<'de> for Either {
            type Value = Spelled;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("a base64 string or an array of bytes")
            }
            fn visit_str<E: serde::de::Error>(self, text: &str) -> Result<Spelled, E> {
                Ok(Spelled {
                    bytes: decode(text).map_err(E::custom)?,
                    numbers: false,
                })
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<Spelled, A::Error> {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0).min(1 << 20));
                while let Some(byte) = seq.next_element::<u8>()? {
                    bytes.push(byte);
                }
                Ok(Spelled {
                    bytes,
                    numbers: true,
                })
            }
        }
        deserializer.deserialize_any(Either)
    }
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn round_trips_every_length_and_refuses_every_other_spelling() {
        let bytes: Vec<u8> = (0..=255).collect();
        for length in 0..bytes.len() {
            let text = encode(&bytes[..length]);
            assert_eq!(decode(&text).unwrap(), &bytes[..length]);
        }
        for refused in [
            "A", "AA", "AAA", "A===", "AA=A", "AB==", "AAB=", "AA==AA==", "AA-_", "AA =",
        ] {
            assert!(decode(refused).is_err(), "{refused}");
        }
        assert_eq!(encode(b"\xff\xfe"), "//4=");
    }
}
