//! Credential redaction before model input (`story:credential-redaction-before-model-input`).
//!
//! A consumer that hands observed text to a model passes it through [`redact`] first: every
//! credential of a [`CredentialShape`] is replaced by a marker naming its shape, and each
//! replacement comes back as a [`Redaction`] holding the shape and the [`RedactionSpan`] only,
//! never the credential. The text given is borrowed and left as it was; a text with no credential
//! comes back byte-identical. Which fields of which source reach a model is the consumer's policy.
//!
//! These are `ekr_core::redact`'s items, re-exported, not a second detector: the same function
//! the observation layer calls as `ekr_observe::redact`. It runs in process and needs no session.

pub use ekr_core::redact::{redact, CredentialShape, ModelInput, Redaction, RedactionSpan};
