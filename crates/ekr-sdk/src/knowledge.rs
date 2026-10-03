//! Typed knowledge inbox operations over the existing child-process transport.
use crate::contracts::{
    EkrGraphObservationRecord, EkrObserveObservationImport, EkrObserveObservationImportReceipt,
    EkrObserveRetainedObservationRead,
};
use crate::read::{ReadError, Reader};
use crate::transport::{Request, Transport};

/// Builds an import using the original adapter's source-key encoding and UUID derivation.
/// The payload is retained byte for byte. Capture time uses the generated contract's RFC 3339
/// string; the runtime checks it before retaining anything.
pub fn observation_import(
    source: String,
    source_native_id: Option<String>,
    captured_at: String,
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
