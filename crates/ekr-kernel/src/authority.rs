//! Host-supplied authority anchor and the two exact deterministic validation profiles.
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

/// The ruleset of validation profile v1, which refuses every schema change.
const P1_RULESET: &str = "ekr.p1-deterministic/1";
/// The application of validation profile v1.
const P1_APPLICATION: &str = "ekr.p1-apply/1";
/// The ruleset of validation profile v2, which admits schema changes (wave p5-01, decision 2).
const P2_RULESET: &str = "ekr.p2-deterministic/1";
/// The application of validation profile v2: a committed schema change replaces the ontology.
const P2_APPLICATION: &str = "ekr.p2-apply/1";

/// The complete fixed profile under which a store seals and replays decisions.
///
/// Two profiles exist, and each is one exact value for a given validator:
/// [`ValidationProfileV1::deterministic`] (v1: `ekr.p1-deterministic/1` + `ekr.p1-apply/1`) and
/// [`ValidationProfileV1::schema_evolving`] (v2: `ekr.p2-deterministic/1` + `ekr.p2-apply/1`).
/// A store keeps the profile its authority anchor named at seed: the anchor is part of the seed
/// envelope and compared again on every reopening, and no mechanism moves a store from one to
/// the other.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "schema",
    schemars(extend("oneOf" = [
        {
            "properties": {
                "ruleset": { "const": "ekr.p1-deterministic/1" },
                "application": { "const": "ekr.p1-apply/1" }
            }
        },
        {
            "properties": {
                "ruleset": { "const": "ekr.p2-deterministic/1" },
                "application": { "const": "ekr.p2-apply/1" }
            }
        }
    ]))
)]
#[serde(deny_unknown_fields)]
pub struct ValidationProfileV1 {
    /// Exactly `ekr.p1-validation-profile/1`, for both profiles.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr.p1-validation-profile/1")))]
    pub format: String,
    /// `ekr.p1-deterministic/1` (v1) or `ekr.p2-deterministic/1` (v2).
    #[cfg_attr(
        feature = "schema",
        schemars(extend("enum" = ["ekr.p1-deterministic/1", "ekr.p2-deterministic/1"]))
    )]
    pub ruleset: String,
    /// All seven checks, in deterministic order.
    pub checks: Vec<ValidatorName>,
    /// The actual independently authenticated validating agent.
    pub validator: AgentId,
    /// Exactly `distinct-authenticated-actor/1`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "distinct-authenticated-actor/1")))]
    pub proposer_separation: String,
    /// Exactly `retained-admissible-evidence/1`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "retained-admissible-evidence/1")))]
    pub provenance: String,
    /// `ekr.p1-apply/1` (v1) or `ekr.p2-apply/1` (v2), matching the ruleset.
    #[cfg_attr(
        feature = "schema",
        schemars(extend("enum" = ["ekr.p1-apply/1", "ekr.p2-apply/1"]))
    )]
    pub application: String,
}
impl ValidationProfileV1 {
    /// Validation profile v1 for a host-selected validator: the P1 profile, which refuses the
    /// three schema kinds.
    #[must_use]
    pub fn deterministic(validator: AgentId) -> Self {
        Self::with(validator, P1_RULESET, P1_APPLICATION)
    }

    /// Validation profile v2 for a host-selected validator: the same seven checks, with
    /// `DefineNodeType`, `DefineEdgeType` and `ModifyProperty` admitted as schema-only
    /// transactions and applied as a new schema version.
    #[must_use]
    pub fn schema_evolving(validator: AgentId) -> Self {
        Self::with(validator, P2_RULESET, P2_APPLICATION)
    }

    /// Whether this profile admits schema changes: true of v2 only.
    #[must_use]
    pub fn admits_schema_changes(&self) -> bool {
        self.ruleset == P2_RULESET && self.application == P2_APPLICATION
    }

    /// Whether this is exactly one of the two supported profiles for `validator`.
    fn is_supported_for(&self, validator: AgentId) -> bool {
        *self == Self::deterministic(validator) || *self == Self::schema_evolving(validator)
    }

    fn with(validator: AgentId, ruleset: &str, application: &str) -> Self {
        Self {
            format: "ekr.p1-validation-profile/1".into(),
            ruleset: ruleset.into(),
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
            application: application.into(),
        }
    }
}

/// Trusted host input, retained completely and compared again on every reopening.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct AuthorityStateV1 {
    /// Exactly `ekr.authority-state/1`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr.authority-state/1")))]
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
            || !self.validation_profile.is_supported_for(context.validator)
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
