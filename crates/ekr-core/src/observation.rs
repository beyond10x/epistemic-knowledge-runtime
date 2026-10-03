//! The existing observation key, shared by the adapter, kernel and transport-only SDK.
use crate::{Canonical, ContentHash, Encoder, ObservationId};
use uuid::Builder;

/// Source identity, optional source-native identity and exact content address.
///
/// `ekr.observe.ObservationIdempotencyKey`, design § 56. This is the original adapter
/// implementation, moved without changing its canonical encoding or UUID derivation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationIdempotencyKey {
    /// The source, by the name the runtime knows it as.
    pub source: String,
    /// The identifier the source itself uses for the record, where it has one.
    pub source_native_id: Option<String>,
    /// The content address of the record's bytes.
    pub content_hash: ContentHash,
}

impl Canonical for ObservationIdempotencyKey {
    fn encode(&self, out: &mut Encoder) {
        self.source.encode(out);
        out.option(self.source_native_id.as_ref());
        self.content_hash.encode(out);
    }
}

impl ObservationIdempotencyKey {
    /// The first sixteen bytes of the key's value address, marked as a custom UUIDv8.
    /// Equal source keys retain exactly the identifiers produced by the original adapter.
    #[must_use]
    pub fn observation_id(&self) -> ObservationId {
        let address = ContentHash::of(self);
        let mut leading = [0_u8; 16];
        leading.copy_from_slice(&address.as_bytes()[..16]);
        ObservationId::from_uuid(Builder::from_custom_bytes(leading).into_uuid())
    }
}
