//! Credential redaction before model input (`story:credential-redaction-before-model-input`).
//!
//! A consumer that hands observed text to a model passes it through [`redact_for_model`] first:
//! every credential of a [`CredentialShape`] is replaced by a marker naming its shape, and each
//! replacement comes back as a [`Redaction`] holding the shape and the [`RedactionSpan`] only,
//! never the credential. The text given is borrowed and left as it was; a text with no credential
//! comes back byte-identical. Which fields of which source reach a model is the consumer's policy.
//!
//! These are `ekr_core::redaction`'s items, re-exported, not a second detector: the same function
//! `ekr-observe` re-exports as `ekr_observe::redaction`. It runs in process and needs no session,
//! and a session does not apply it: a text read back through a session reaches the consumer as it
//! was committed, and is redacted only if the consumer passes it through [`redact_for_model`].

pub use ekr_core::redaction::{
    redact_for_model, CredentialShape, ModelInput, Redaction, RedactionSpan,
};
