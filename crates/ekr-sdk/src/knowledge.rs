//! Typed knowledge inbox operations over the existing child-process transport.
use crate::contracts::{
    EkrGraphObservationRecord, EkrObserveObservationImport, EkrObserveObservationImportReceipt,
    EkrObserveRetainedObservationRead,
};
use crate::read::{ReadError, Reader};
use crate::transport::{Request, Transport};

/// Builds an import using the original adapter's source-key encoding and UUID derivation.
/// The payload is retained byte for byte. Capture time uses the generated RFC 3339 timestamp.
pub fn observation_import(
    source: String,
    source_native_id: Option<String>,
    captured_at: crate::contracts::EssTimestamp,
    kind: crate::contracts::EkrGraphObservationKind,
    payload: &[u8],
) -> EkrObserveObservationImport {
    use crate::contracts::{
        EkrGraphObservationId, EkrKernelContentHash, EkrObserveObservationIdempotencyKey,
        EssPresence,
    };
    let key = ekr_core::ObservationIdempotencyKey {
        source,
        source_native_id,
        content_hash: ekr_core::ContentHash::of_bytes(payload),
    };
    let id = key.observation_id();
    let source_native_id = key
        .source_native_id
        .map_or(EssPresence::Absent, EssPresence::Present);
    let hash = Box::new(EkrKernelContentHash(key.content_hash.to_hex()));
    EkrObserveObservationImport {
        observation: Box::new(EkrGraphObservationRecord {
            observation_id: Box::new(EkrGraphObservationId(id.to_string())),
            source: key.source.clone(),
            source_native_id: source_native_id.clone(),
            content_hash: hash.clone(),
            captured_at,
            kind: Box::new(kind),
        }),
        key: Box::new(EkrObserveObservationIdempotencyKey {
            source: key.source,
            source_native_id,
            content_hash: hash,
        }),
        payload: ekr_core::bytes::encode(payload),
    }
}

/// A client for supplied knowledge. Every operation uses the configured CLI/session transport.
pub struct Knowledge<T: Transport> {
    transport: T,
}

impl<T: Transport> Knowledge<T> {
    /// Previews the supported authority upgrade using public policy material. The runtime must
    /// already have an independently provisioned binding to this exact policy digest.
    /// # Errors
    /// Serialization, transport, unprovisioned trust, runtime refusal or invalid response.
    pub fn preview_upgrade(
        &mut self,
        policy: &crate::contracts::EkrKernelReviewerTrustPolicy,
    ) -> Result<crate::contracts::EkrKernelUpgradePreview, ReadError> {
        let text = serde_json::to_string(policy).map_err(|source| ReadError::Document {
            verb: "upgrade preview".into(),
            source,
        })?;
        Reader::new(&mut self.transport)
            .read_request(Request::new(["upgrade", "preview", "-"]).with_stdin(text))
    }
    /// Applies an exact externally signed review through ordinary CLI/session transport.
    /// This client never signs a decision, selects a trusted reviewer or upgrades implicitly.
    /// # Errors
    /// Serialization, transport, invalid proof or stale basis, runtime refusal or invalid response.
    pub fn apply_upgrade(
        &mut self,
        input: &crate::contracts::EkrKernelAuthorityUpgradeApplication,
    ) -> Result<crate::contracts::EkrKernelAuthorityTransitionRecord, ReadError> {
        let text = serde_json::to_string(input).map_err(|source| ReadError::Document {
            verb: "upgrade apply".into(),
            source,
        })?;
        Reader::new(&mut self.transport)
            .read_request(Request::new(["upgrade", "apply", "-"]).with_stdin(text))
    }
    /// Submit an externally signed answer; an exact retry returns the original receipt.
    /// # Errors
    /// Invalid JSON, transport failure, invalid signature, changed review basis or runtime refusal.
    pub fn answer_attention(
        &mut self,
        input: &crate::contracts::EkrKernelAttentionAnswerApplication,
    ) -> Result<crate::contracts::EkrKernelAnswerReceipt, ReadError> {
        let text = serde_json::to_string(input).map_err(|source| ReadError::Document {
            verb: "attention answer".into(),
            source,
        })?;
        Reader::new(&mut self.transport)
            .read_request(Request::new(["attention", "answer", "-"]).with_stdin(text))
    }
    /// Read immutable authenticated answer history, optionally restricted to one dispute.
    /// # Errors
    /// Invalid dispute identity, transport failure, unverifiable history or invalid response.
    pub fn answer_history(
        &mut self,
        dispute: Option<&crate::contracts::EkrKernelDisputeId>,
    ) -> Result<Vec<crate::contracts::EkrKernelHumanAnswerRecord>, ReadError> {
        let mut args = vec!["attention".to_owned(), "history".to_owned()];
        if let Some(id) = dispute {
            args.extend(["--dispute".into(), id.0.clone()]);
        }
        Reader::new(&mut self.transport).read_request(Request::new(args))
    }
    /// Lists unresolved questions with the kernel's exact evidence-bound review basis.
    /// # Errors
    /// Transport failure, invalid retained state or an invalid response document.
    pub fn attention(
        &mut self,
    ) -> Result<Vec<crate::contracts::EkrKernelAttentionItem>, ReadError> {
        Reader::new(&mut self.transport).read_request(Request::new(["attention", "list"]))
    }
    /// Reads a current question by the typed subject returned by [`Self::attention`].
    /// # Errors
    /// Malformed, unknown or settled subject, transport failure or invalid response.
    pub fn attention_item(
        &mut self,
        subject: &crate::contracts::EkrKernelAttentionSubject,
    ) -> Result<crate::contracts::EkrKernelAttentionItem, ReadError> {
        use crate::contracts::{EkrKernelAttentionKind as K, EssPresence as P};
        let (kind, id) = match (
            &*subject.kind,
            &subject.dispute_id,
            &subject.blocker_id,
            &subject.proposal_id,
        ) {
            (K::V0, P::Absent, P::Present(id), P::Absent) => ("blocked-integration", id.0.clone()),
            (K::V1, P::Present(id), P::Absent, P::Absent) => ("dispute", id.0.clone()),
            (K::V2, P::Absent, P::Absent, P::Present(id)) => ("schema-proposal", id.0.clone()),
            _ => {
                return Err(ReadError::Document {
                    verb: "attention show".into(),
                    source: <serde_json::Error as serde::de::Error>::custom(
                        "attention subject kind and identity disagree",
                    ),
                })
            }
        };
        Reader::new(&mut self.transport).read_request(Request::new([
            "attention".into(),
            "show".into(),
            kind.into(),
            id,
        ]))
    }
    /// Retains a typed local interpretation and its exact bytes without committing its facts.
    /// # Errors
    /// Serialization, transport, runtime refusal or invalid response.
    pub fn import_interpretation(
        &mut self,
        input: &crate::contracts::EkrIntegrateInterpretationImport,
    ) -> Result<crate::contracts::EkrIntegrateIncubationImportReceipt, ReadError> {
        let text = serde_json::to_string(input).map_err(|source| ReadError::Document {
            verb: "incubate import".into(),
            source,
        })?;
        Reader::new(&mut self.transport)
            .read_request(Request::new(["incubate", "import", "-"]).with_stdin(text))
    }
    /// Lists immutable interpretation coordinates and exact byte digests.
    /// # Errors
    /// Transport, runtime refusal or invalid response.
    pub fn interpretations(
        &mut self,
    ) -> Result<Vec<crate::contracts::EkrIntegrateInterpretationVersion>, ReadError> {
        Reader::new(&mut self.transport).read_request(Request::new(["incubate", "list"]))
    }
    /// Reads local facts, declarations, blockers and receipts for an exact immutable version.
    /// # Errors
    /// Unknown version, changed digest, transport failure, runtime refusal or invalid response.
    pub fn interpretation(
        &mut self,
        version: &crate::contracts::EkrIntegrateInterpretationVersion,
    ) -> Result<crate::contracts::EkrIntegrateInterpretationRead, ReadError> {
        Reader::new(&mut self.transport).read_request(Request::new([
            "incubate".into(),
            "show".into(),
            version.interpretation_id.0.clone(),
            version.version.to_string(),
            version.document_digest.0.clone(),
        ]))
    }
    /// Uses a child-process session or another admitted transport.
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
    /// Returns the transport without opening or changing a store.
    pub fn into_inner(self) -> T {
        self.transport
    }
    /// Imports a typed observation. An identical retry returns the retained identity.
    /// # Errors
    /// Serialization, transport, runtime refusal or invalid response.
    pub fn import_observation(
        &mut self,
        input: &EkrObserveObservationImport,
    ) -> Result<EkrObserveObservationImportReceipt, ReadError> {
        let text = serde_json::to_string(input).map_err(|source| ReadError::Document {
            verb: "observe import".into(),
            source,
        })?;
        Reader::new(&mut self.transport)
            .read_request(Request::new(["observe", "import", "-"]).with_stdin(text))
    }
    /// Lists independently retained source records in identity order.
    /// # Errors
    /// Transport, runtime refusal or invalid response.
    pub fn observations(&mut self) -> Result<Vec<EkrGraphObservationRecord>, ReadError> {
        Reader::new(&mut self.transport).read_request(Request::new(["observe", "list"]))
    }
    /// Reads an observation's source metadata and exact base64 payload.
    /// # Errors
    /// Unknown identity, transport failure, runtime refusal or invalid response.
    pub fn observation(
        &mut self,
        id: ekr_core::ObservationId,
    ) -> Result<EkrObserveRetainedObservationRead, ReadError> {
        Reader::new(&mut self.transport).read_request(Request::new([
            "observe".into(),
            "show".into(),
            id.to_string(),
        ]))
    }
}
