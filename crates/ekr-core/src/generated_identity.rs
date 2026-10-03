//! Generated identities for the knowledge contracts, with checked UUID ingress.
//!
//! The generated serde types deliberately retain wire strings. Use [`Identity::parse_identity`]
//! at a runtime boundary: deserializing a generated document alone does not validate its IDs.

/// Checked operations shared by generated UUID identities without duplicating their models.
pub trait Identity: Sized {
    /// Mint a UUIDv7 in its one canonical wire spelling.
    fn mint() -> Self;
    /// Read only lowercase hyphenated UUID text.
    fn parse_identity(text: &str) -> Result<Self, crate::IdParseError>;
    /// The stored wire spelling. Callers must validate deserialized input before trusting it.
    fn identity_text(&self) -> &str;
}

macro_rules! identities {
    ($($name:ident => $generated:ident),+ $(,)?) => {
        $(
            #[doc = concat!("Generated ESS identity `", stringify!($name), "`.")]
            pub use ekr_contract_data::$generated as $name;
            impl Identity for $name {
                fn mint() -> Self { Self(uuid::Uuid::now_v7().to_string()) }
                fn parse_identity(text: &str) -> Result<Self, crate::IdParseError> {
                    // Reuse the strict legacy UUID parser, including its one-spelling rule.
                    let _: crate::NodeId = text.parse()?;
                    Ok(Self(text.to_owned()))
                }
                fn identity_text(&self) -> &str { &self.0 }
            }
        )+
        /// The exact generated UUID inventory, checked against every ESS domain beside legacy IDs.
        pub const NAMES: &[&str] = &[$(stringify!($name)),+];

        #[cfg(test)]
        mod tests {
            use super::*;
            #[test]
            fn every_generated_identity_mints_and_checks_canonical_uuid_text() {
                $(
                    let first = $name::mint();
                    let second = $name::mint();
                    assert_ne!(first.identity_text(), second.identity_text());
                    assert_eq!(uuid::Uuid::parse_str(first.identity_text()).unwrap().get_version_num(), 7);
                    assert_eq!($name::parse_identity(first.identity_text()).unwrap().identity_text(), first.identity_text());
                    assert!($name::parse_identity("not-an-id").is_err());
                    assert!($name::parse_identity("00000000-0000-0000-0000-00000000000A").is_err());
                    let json = serde_json::to_string(&first).unwrap();
                    let read: $name = serde_json::from_str(&json).unwrap();
                    assert_eq!(read.identity_text(), first.identity_text());
                )+
            }
        }
    };
}

identities! {
    InterpretationId => EkrIntegrateInterpretationId,
    IntegrationBlockerId => EkrIntegrateIntegrationBlockerId,
    ProcessingReceiptId => EkrIntegrateProcessingReceiptId,
    SchemaProposalId => EkrIntegrateSchemaProposalId,
    ProposalReviewId => EkrIntegrateProposalReviewId,
    ApplicationReceiptId => EkrIntegrateApplicationReceiptId,
    SchemaApplicationId => EkrIntegrateSchemaApplicationId,
    ApplicationStepId => EkrIntegrateApplicationStepId,
    MappingRecordId => EkrIntegrateMappingRecordId,
    GapGroupId => EkrIntegrateGapGroupId,
    DisputeId => EkrKernelDisputeId,
    HumanAnswerId => EkrKernelHumanAnswerId,
    AuthorityTransitionId => EkrKernelAuthorityTransitionId,
}
