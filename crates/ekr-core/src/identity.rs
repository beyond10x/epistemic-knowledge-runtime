//! Stable identity: the id newtypes of the runtime, minted as UUIDv7.
//!
//! Design § 6.4 and § 10: a human-readable name is a property, never an identity. Every persistent
//! object carries an id that survives renames, aliases, merges and schema migration, so the id is
//! minted once and never derived from anything a person can edit.
//!
//! Each type below wraps a `u128` — a UUID's bits, not a counter — and is minted with
//! [`uuid::Uuid::now_v7`], whose leading 48 bits are a millisecond timestamp, so ids sort roughly
//! in creation order without carrying meaning.
//!
//! The types are distinct on purpose: an `EdgeId` where a `NodeId` belongs is a compile error, not
//! a runtime one. Each names the ESS declaration it comes from.

use std::fmt;
use std::str::FromStr;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::canonical::{Canonical, Encoder};

/// A string that is not an identifier.
///
/// Returned by every id type's [`FromStr`], and the reason serde refuses a malformed record. It
/// carries the text it refused and nothing else: a caller that needs to know *why* the text is not
/// a UUID has a problem no error variant fixes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a UUID identifier")]
pub struct IdParseError(String);

impl IdParseError {
    /// The text that was refused.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// A string that is not a revision number.
///
/// Its own type rather than `std::num::ParseIntError`, because the refusals are this crate's and
/// not the integer parser's: `+7` and `007` are numbers `u64` reads and a `RevisionNumber` does
/// not.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a revision number: expected decimal digits with no sign and no leading zero")]
pub struct RevisionNumberParseError(String);

impl RevisionNumberParseError {
    /// The text that was refused.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Whether `text` is exactly the 36-character lowercase hyphenated form every id writes:
/// `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`, hex in lowercase, hyphens at 8, 13, 18 and 23.
///
/// One value has one text, or everything that compares text — a cache key, a snapshot diff, a
/// file name, a JSON document held for byte equality — sees changes that are not changes.
fn is_canonical_uuid_text(text: &str) -> bool {
    const HYPHENS: [usize; 4] = [8, 13, 18, 23];
    const LEN: usize = 36;

    text.len() == LEN
        && text.bytes().enumerate().all(|(at, byte)| {
            if HYPHENS.contains(&at) {
                byte == b'-'
            } else {
                byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)
            }
        })
}

/// Declares one id newtype over `u128`, minted as UUIDv7.
///
/// Fifteen types share this shape; writing it fifteen times is fifteen chances to write it
/// differently. `$doc` is the type's own rustdoc and names the ESS declaration it carries.
macro_rules! id_newtype {
    ($(#[doc = $doc:expr])+ $name:ident) => {
        $(#[doc = $doc])+
        #[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);

        impl $name {
            /// Mints a new id as UUIDv7.
            ///
            /// Every call returns a distinct value; two objects with the same name have different
            /// ids, and one object keeps its id through every rename.
            #[must_use]
            pub fn mint() -> Self {
                Self(Uuid::now_v7().as_u128())
            }

            /// Rebuilds the id from a UUID that was minted before — a stored one, or one an
            /// import carries.
            #[must_use]
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid.as_u128())
            }

            /// The id as a UUID.
            #[must_use]
            pub const fn to_uuid(self) -> Uuid {
                Uuid::from_u128(self.0)
            }

            /// The id's bits.
            #[must_use]
            pub const fn as_u128(self) -> u128 {
                self.0
            }
        }

        impl fmt::Display for $name {
            /// The hyphenated lowercase UUID form — what serde writes and [`FromStr`] reads.
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.to_uuid().as_hyphenated())
            }
        }

        impl FromStr for $name {
            type Err = IdParseError;

            /// Reads the form [`Display`](fmt::Display) writes, and no other spelling of it.
            ///
            /// `uuid::Uuid::parse_str` also accepts the simple, braced and `urn:uuid:` wrappings
            /// and uppercase hex in each, which would give one id eight text forms and let a
            /// stored record read back as a string different from the one it was written as.
            /// [`ContentHash`](crate::ContentHash) refuses the same way and for the same reason.
            fn from_str(text: &str) -> Result<Self, Self::Err> {
                if !is_canonical_uuid_text(text) {
                    return Err(IdParseError(text.to_owned()));
                }
                Uuid::parse_str(text)
                    .map(Self::from_uuid)
                    .map_err(|_| IdParseError(text.to_owned()))
            }
        }

        impl Serialize for $name {
            /// As the text form; an id crosses a boundary as a string, never as 128 bits of
            /// number a JSON reader would round.
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                serializer.collect_str(&self.to_uuid().as_hyphenated())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            /// From the text form only. Anything else — a number, null, a string that is not a
            /// UUID — is refused rather than defaulted.
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let text = String::deserialize(deserializer)?;
                text.parse().map_err(D::Error::custom)
            }
        }

        impl Canonical for $name {
            /// The id's sixteen bytes, tagged — not its text, which would make the encoding
            /// depend on a formatting choice.
            fn encode(&self, out: &mut Encoder) {
                out.id(self.0);
            }
        }
    };
}

id_newtype! {
    /// A kernel-allocated immutable occurrence, independent of its content address.
    EventId
}

id_newtype! {
    /// An agent: `ekr.kernel.AgentId` of `systems/ekr/domains/kernel.yaml`.
    ///
    /// Carried by `ekr.kernel.Agent.id` and by every transaction's `proposed_by`.
    AgentId
}

id_newtype! {
    /// A transaction: `ekr.kernel.TransactionId` of `systems/ekr/domains/kernel.yaml`.
    ///
    /// Carried by `ekr.kernel.GraphTransaction.id`; a commit names it.
    TransactionId
}

id_newtype! {
    /// A committed revision: `ekr.kernel.RevisionId` of `systems/ekr/domains/kernel.yaml`.
    ///
    /// Carried by `ekr.kernel.Revision.id`. The revision's position in the lineage is its
    /// [`RevisionNumber`], which is a different thing.
    RevisionId
}

id_newtype! {
    /// A validation issue: `ekr.kernel.IssueId` of `systems/ekr/domains/kernel.yaml`.
    ///
    /// Carried by `ekr.kernel.ValidationIssue.id`, which names the transaction it was raised
    /// against.
    IssueId
}

id_newtype! {
    /// A type in an ontology: `ekr.ontology.TypeId` of `systems/ekr/domains/ontology.yaml`.
    TypeId
}

id_newtype! {
    /// A property definition: `ekr.ontology.PropertyId` of `systems/ekr/domains/ontology.yaml`.
    PropertyId
}

id_newtype! {
    /// A schema version: `ekr.ontology.SchemaVersionId` of `systems/ekr/domains/ontology.yaml`.
    SchemaVersionId
}

id_newtype! {
    /// A graph root: `ekr.graph.GraphRootId` of `systems/ekr/domains/graph.yaml`.
    GraphRootId
}

id_newtype! {
    /// A node: `ekr.graph.NodeId` of `systems/ekr/domains/graph.yaml`.
    ///
    /// The id a rename does not move — design § 6.4, and the acceptance of
    /// `story:kernel-identity-and-hashing`.
    NodeId
}

id_newtype! {
    /// An edge: `ekr.graph.EdgeId` of `systems/ekr/domains/graph.yaml`.
    EdgeId
}

id_newtype! {
    /// A bitemporal assertion: `ekr.graph.AssertionId` of `systems/ekr/domains/graph.yaml`.
    AssertionId
}

id_newtype! {
    /// The support an assertion rests on: `ekr.graph.SupportId` of
    /// `systems/ekr/domains/graph.yaml`.
    SupportId
}

id_newtype! {
    /// A piece of evidence: `ekr.graph.EvidenceId` of `systems/ekr/domains/graph.yaml`.
    EvidenceId
}

id_newtype! {
    /// An observation: `ekr.graph.ObservationId` of `systems/ekr/domains/graph.yaml`.
    ObservationId
}

/// The position of a revision in the lineage: `ekr.kernel.RevisionNumber` of
/// `systems/ekr/domains/kernel.yaml`, declared there as `newtype of: Integer`.
///
/// Not an id and not minted: design § 34 gives `Root.revision` as a `u64`, and the kernel's own
/// errors ask for it by value — `RevisionNotFound { requested }`, `TransactionStale
/// { validated_against, current }`. A UUID could not answer "no committed revision carries that
/// number", so this one counts instead, from zero at the seed, one per commit.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RevisionNumber(u64);

impl RevisionNumber {
    /// The seed revision, before any commit.
    pub const SEED: Self = Self(0);

    /// The number a stored revision carries.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        Self(number)
    }

    /// The number itself.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// The next revision in the lineage, or `None` past `u64::MAX`.
    #[must_use]
    pub const fn next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(next) => Some(Self(next)),
            None => None,
        }
    }
}

impl fmt::Display for RevisionNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for RevisionNumber {
    type Err = RevisionNumberParseError;

    /// Reads the decimal form [`Display`](fmt::Display) writes, and no other spelling of it.
    ///
    /// `u64::from_str` also accepts a leading `+` and any number of leading zeros, which would
    /// give one revision number infinitely many texts — the same defect the id types had, in a
    /// different alphabet. A number is not exempt from the one-text rule.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refuse = || RevisionNumberParseError(text.to_owned());
        if !text.bytes().all(|byte| byte.is_ascii_digit()) || text.is_empty() {
            return Err(refuse());
        }
        if text.len() > 1 && text.starts_with('0') {
            return Err(refuse());
        }
        text.parse().map(Self).map_err(|_| refuse())
    }
}
