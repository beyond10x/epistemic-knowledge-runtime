//! Content addressing: the SHA-256 of a value's canonical bytes, under a domain.
//!
//! Design § 57: raw payloads and immutable artifacts are content-addressed, so an artifact's
//! address is its content and two equal artifacts have one address between them. The digest is
//! taken over the [`Canonical`] encoding, never over a serialisation,
//! because a serialisation is a function of its format as well as of its value.
//!
//! # Two domains, never one
//!
//! There are two kinds of thing to address and they do not share an address space:
//!
//! * a **value** of this runtime, addressed by [`ContentHash::of`] over its canonical encoding;
//! * a **payload** from outside it, addressed by [`ContentHash::of_bytes`] over bytes nobody here
//!   chose — the observations and evidence the story's Notes name.
//!
//! Each digest is taken over its domain label followed by the bytes. Without the labels the two
//! spaces are one, and a payload whose bytes happen to be some value's canonical encoding takes
//! that value's address — the collision rule 1 of [`canonical`](crate::canonical) exists to
//! prevent, on the one entry point that writes no tag, fed by input the runtime does not choose.
//! Neither label is a prefix of the other, which is what makes the separation total rather than
//! probable: two byte strings that begin with labels differing at some position differ there,
//! whatever follows.
//!
//! The labels carry a version because changing one changes every address ever recorded. A change
//! to either is a migration, not an edit.

use std::fmt;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};

use crate::canonical::Canonical;

/// The number of hex characters a `ContentHash` reads and writes.
const HEX_LEN: usize = 64;

/// A SHA-256 digest over canonical bytes: `ekr.kernel.ContentHash` of
/// `systems/ekr/domains/kernel.yaml`.
///
/// Held as the thirty-two bytes and written as sixty-four lowercase hex characters. The ESS
/// declares the type `newtype of: String` because that is its wire shape; the bytes are the
/// value.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// The domain label of an address over raw bytes the runtime did not choose.
    ///
    /// Neither this nor [`ContentHash::VALUE_DOMAIN`] is a prefix of the other; see the module
    /// documentation for why that is the whole of the separation.
    pub const PAYLOAD_DOMAIN: &'static [u8] = b"ekr.payload.v1";

    /// The domain label of an address over a value's canonical encoding.
    pub const VALUE_DOMAIN: &'static [u8] = b"ekr.value.v1";

    /// The digest of a domain label followed by bytes. The one place either domain is hashed.
    fn under_domain(domain: &[u8], bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(domain);
        hasher.update(bytes);
        Self(hasher.finalize().into())
    }

    /// The address of a raw payload — bytes from outside the runtime, under
    /// [`ContentHash::PAYLOAD_DOMAIN`].
    ///
    /// Not the bare SHA-256 of `bytes`: an attacker who chooses the payload would otherwise
    /// choose its address, and could hand over the canonical encoding of a value to be given that
    /// value's address.
    #[must_use]
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self::under_domain(Self::PAYLOAD_DOMAIN, bytes)
    }

    /// The address of a value — its canonical encoding under [`ContentHash::VALUE_DOMAIN`].
    ///
    /// `of(&v)` is never `of_bytes(&v.canonical_bytes())`; the two live in different domains, and
    /// that is the point.
    #[must_use]
    pub fn of<T: Canonical + ?Sized>(value: &T) -> Self {
        Self::under_domain(Self::VALUE_DOMAIN, &value.canonical_bytes())
    }

    /// A digest that was computed before — read back from storage, say.
    #[must_use]
    pub const fn from_bytes(digest: [u8; 32]) -> Self {
        Self(digest)
    }

    /// The thirty-two bytes of the digest.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// The sixty-four lowercase hex characters the digest is written as.
    #[must_use]
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Display for ContentHash {
    /// Lowercase hex, and only lowercase: a content address has one text form or it is not one.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}

/// A string that is not a content hash.
///
/// One error for every way the text can be wrong; a caller that must tell "too short" from "not
/// hex" is reading a hash it should have refused either way.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a content hash: expected 64 lowercase hex characters")]
pub struct ContentHashParseError(String);

impl ContentHashParseError {
    /// The text that was refused.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

impl FromStr for ContentHash {
    type Err = ContentHashParseError;

    /// Reads the lowercase hex form only. Uppercase is refused rather than accepted and
    /// normalised: two spellings of one address are two addresses to everything that compares
    /// text.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refuse = || ContentHashParseError(text.to_owned());
        if text.len() != HEX_LEN
            || !text
                .bytes()
                .all(|b| b.is_ascii_digit() || b"abcdef".contains(&b))
        {
            return Err(refuse());
        }
        let mut digest = [0u8; 32];
        hex::decode_to_slice(text, &mut digest).map_err(|_| refuse())?;
        Ok(Self(digest))
    }
}

impl Serialize for ContentHash {
    /// As the lowercase hex string the ESS declares.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ContentHash {
    /// From that same string and nothing else.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(D::Error::custom)
    }
}
