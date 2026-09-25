//! A point in time: the one scalar every domain of the runtime dates its records with.
//!
//! [`Timestamp`] is a newtype over `i64`, counting **milliseconds since the Unix epoch, UTC**.
//! `architecture-decision-record:0004-timestamp-in-ekr-core` settles that, and settles that it
//! lives here rather than in `ekr-graph` where the roadmap had put it: four of the five ESS
//! domains declare a `Timestamp` field — `kernel.yaml`, `graph.yaml`, `ontology.yaml`,
//! `store.yaml` — and `ekr-ontology` does not depend on `ekr-graph`, so a `Timestamp` any higher
//! would leave one domain scalar with two unrelated Rust types.
//!
//! * **Milliseconds, not nanoseconds.** `i64` nanoseconds spans 1678–2262. Valid time (design
//!   § 13–14) is the time a fact was true in the world, which for an organisational memory is
//!   routinely outside that window; `i64` milliseconds spans roughly ±292 million years.
//! * **Milliseconds, not seconds.** Transaction time orders records written inside one process.
//!   Commit order itself is carried by [`RevisionNumber`](crate::RevisionNumber) rather than by a
//!   clock, so a millisecond collision is not a correctness problem — a second-resolution one
//!   would still lose the ordering a reader expects.
//! * **Signed.** Valid time before 1970 is ordinary: a claim about when a company was founded is
//!   one.
//!
//! # What this type is not
//!
//! It is an integer with an origin and a unit, and nothing else. There is no RFC 3339 formatting,
//! because that needs a calendar; there is no clock, because nothing in this crate reads the
//! system time — design § 19–20 makes the transaction the thing that stamps a record, and a
//! `Timestamp` always arrives from its caller. It makes no leap-second or monotonicity claim.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::canonical::{Canonical, Encoder};

/// A string that is not a timestamp.
///
/// Its own type rather than `std::num::ParseIntError`, for the reason
/// [`RevisionNumberParseError`](crate::RevisionNumberParseError) has one: the refusals are this
/// crate's and not the integer parser's. `+7`, `007` and `-0` are numbers `i64` reads and a
/// `Timestamp` does not, because one value with several texts is a value that a cache key, a
/// snapshot diff or a stored document sees change when nothing changed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a timestamp: expected decimal milliseconds, optionally negative, with no leading zero")]
pub struct TimestampParseError(String);

impl TimestampParseError {
    /// The text that was refused.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// A point in time, as milliseconds since the Unix epoch in UTC: the `Timestamp` scalar of every
/// ESS domain that declares one.
///
/// Ordered, so a valid-time or transaction-time comparison is the integer comparison it should be.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(transparent)]
pub struct Timestamp(i64);

impl Timestamp {
    /// The Unix epoch itself, 1970-01-01T00:00:00Z.
    ///
    /// Not a default and not a null: an unknown time is [`None`], never this value.
    pub const EPOCH: Self = Self(0);

    /// The instant `millis` milliseconds after the Unix epoch, or before it when negative.
    #[must_use]
    pub const fn from_millis(millis: i64) -> Self {
        Self(millis)
    }

    /// The milliseconds since the Unix epoch.
    #[must_use]
    pub const fn millis(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    /// The decimal milliseconds — the one text form, which [`FromStr`] reads back.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for Timestamp {
    type Err = TimestampParseError;

    /// Reads the decimal form [`Display`](fmt::Display) writes, and no other spelling of it.
    ///
    /// `i64::from_str` also accepts a leading `+` and any number of leading zeros, and both it and
    /// this type read `-0` as zero — which would give one instant several texts. The same
    /// one-text rule the id types and [`RevisionNumber`](crate::RevisionNumber) keep, in a third
    /// alphabet.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let refuse = || TimestampParseError(text.to_owned());
        let digits = text.strip_prefix('-').unwrap_or(text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(refuse());
        }
        if digits.len() > 1 && digits.starts_with('0') {
            return Err(refuse());
        }
        // `-0` is zero written a second way, and zero already has a text.
        if digits == "0" && text.starts_with('-') {
            return Err(refuse());
        }
        text.parse().map(Self).map_err(|_| refuse())
    }
}

impl Canonical for Timestamp {
    /// The bytes of the `i64` it wraps, with no discriminant: a newtype is structural, which is
    /// rule 5 of [`canonical`](crate::canonical).
    fn encode(&self, out: &mut Encoder) {
        out.signed(i128::from(self.0));
    }
}
