//! Host-supplied authority anchor and the exact deterministic P1 validation profile.
use crate::{BootstrapContext, ValidatorName};
use ekr_core::{AgentId, Canonical, Encoder};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// A registered execution identity; capabilities are retained metadata in P1.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Agent {
    /// Stable identity, equal to its registry key.
    pub id: AgentId,
    /// Human-readable name, without identity authority.
    pub name: String,
    /// Retained capabilities; P1 does not infer policy semantics from them.
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub capabilities: BTreeSet<String>,
}

/// The complete fixed profile under which P1 seals and replays decisions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ValidationProfileV1 {
    /// Exactly `ekr.p1-validation-profile/1`.
    pub format: String,
    /// Exactly `ekr.p1-deterministic/1`.
    pub ruleset: String,
    /// All seven checks, in deterministic order.
    pub checks: Vec<ValidatorName>,
    /// The actual independently authenticated validating agent.
    pub validator: AgentId,
    /// Exactly `distinct-authenticated-actor/1`.
    pub proposer_separation: String,
    /// Exactly `retained-admissible-evidence/1`.
    pub provenance: String,
    /// Exactly `ekr.p1-apply/1`.
    pub application: String,
}
impl ValidationProfileV1 {
    /// Constructs the supported profile for a host-selected validator.
    #[must_use]
    pub fn deterministic(validator: AgentId) -> Self {
        Self {
            format: "ekr.p1-validation-profile/1".into(),
            ruleset: "ekr.p1-deterministic/1".into(),
            checks: vec![
                ValidatorName::Structural,
                ValidatorName::Reference,
                ValidatorName::Type,
                ValidatorName::Cardinality,
                ValidatorName::OntologyConstraint,
                ValidatorName::Provenance,
                ValidatorName::Authorization,
            ],
            validator,
            proposer_separation: "distinct-authenticated-actor/1".into(),
            provenance: "retained-admissible-evidence/1".into(),
            application: "ekr.p1-apply/1".into(),
        }
    }
}

/// Trusted host input, retained completely and compared again on every reopening.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AuthorityStateV1 {
    /// Exactly `ekr.authority-state/1`.
    pub format: String,
    /// Complete identity registry, including agents unused by the initial seed.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub agents: BTreeMap<AgentId, Agent>,
    /// The exact admitted profile.
    pub validation_profile: ValidationProfileV1,
}
impl AuthorityStateV1 {
    pub(crate) fn check(&self, context: BootstrapContext) -> Result<(), ekr_store::StoreError> {
        if context.operator == context.validator {
            return Err(ekr_store::StoreError::InvalidSeed(
                "proposer-is-validator".into(),
            ));
        }
        if self.format != "ekr.authority-state/1"
            || self.validation_profile != ValidationProfileV1::deterministic(context.validator)
            || !self.agents.contains_key(&context.operator)
            || !self.agents.contains_key(&context.validator)
            || self.agents.iter().any(|(id, agent)| *id != agent.id)
        {
            return Err(ekr_store::StoreError::InvalidSeed(
                "seed-authority-profile".into(),
            ));
        }
        Ok(())
    }
}
impl Canonical for Agent {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.name.encode(out);
        self.capabilities.encode(out);
    }
}
impl Canonical for ValidationProfileV1 {
    fn encode(&self, out: &mut Encoder) {
        self.format.encode(out);
        self.ruleset.encode(out);
        self.checks.encode(out);
        self.validator.encode(out);
        self.proposer_separation.encode(out);
        self.provenance.encode(out);
        self.application.encode(out);
    }
}
impl Canonical for AuthorityStateV1 {
    fn encode(&self, out: &mut Encoder) {
        self.format.encode(out);
        self.agents.encode(out);
        self.validation_profile.encode(out);
    }
}
impl Canonical for ValidatorName {
    fn encode(&self, out: &mut Encoder) {
        out.variant(match self {
            Self::Structural => 0,
            Self::Reference => 1,
            Self::Type => 2,
            Self::Cardinality => 3,
            Self::OntologyConstraint => 4,
            Self::Provenance => 5,
            Self::Authorization => 6,
        });
    }
}
