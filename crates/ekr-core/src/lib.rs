//! Kernel types for the Epistemic Knowledge Runtime.
//!
//! Implements the types half of the `ekr.kernel` domain, `systems/ekr/domains/kernel.yaml`:
//! stable identifiers, content hashes and the canonical encoding every layer above names. The
//! commands of the same domain live in `ekr-kernel`, which sits above the graph.
//!
//! Three primitives, in dependency order:
//!
//! * [`identity`] — the id newtypes, minted as UUIDv7. Design § 6.4 and § 10: a name is a
//!   property, never an identity, so an id survives every rename.
//! * [`canonical`] — the deterministic byte encoding a value has, whatever built it.
//! * [`hash`] — the SHA-256 of those bytes, which is how design § 57 addresses content.
//! * [`time`] — [`Timestamp`], the point-in-time scalar every domain above dates its records
//!   with (`architecture-decision-record:0004-timestamp-in-ekr-core`).
//!
//! This crate names no domain concept and no graph concept — `AGENTS.md` invariant 8 — and
//! depends on no other crate of the workspace.
//!
//! ```
//! use ekr_core::{ContentHash, NodeId};
//!
//! let id = NodeId::mint();
//! assert_eq!(id.to_string().parse::<NodeId>().unwrap(), id);
//! assert_eq!(ContentHash::of(&"the same bytes".to_owned()).to_string().len(), 64);
//! ```

pub mod canonical;
pub mod hash;
pub mod identity;
pub mod time;

pub use canonical::{Canonical, Encoder};
pub use hash::{ContentHash, ContentHashParseError};
pub use identity::{
    AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, IdParseError, IssueId, NodeId,
    ObservationId, PropertyId, RevisionId, RevisionNumber, RevisionNumberParseError,
    SchemaVersionId, SupportId, TransactionId, TypeId,
};
pub use time::{Timestamp, TimestampParseError};
