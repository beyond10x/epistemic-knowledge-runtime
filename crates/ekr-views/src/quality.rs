//! `ReportStoreQuality` and its format `ekr.store-quality/1` ([`report_quality`]): how well one
//! revision's assertions are evidenced, its properties constrained and its nodes of one type
//! named apart (`views.yaml`, `ekr.views.StoreQualityV1`).
//!
//! The report is a function of one revision. It reads the revision as [`crate::load`] does, and
//! the seed's evidence ids — [`Runtime::replay`] of revision 0, read only for a later revision —
//! which is what tells evidence an `AddEvidence` brought after the seed from the seed's own.
//! [`quality`] is the pure half: the same inputs give the same bytes.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EvidenceId, RevisionNumber};
use ekr_graph::AssertionLifecycle;
use ekr_kernel::Runtime;
use serde::Serialize;

use crate::query::{encode, hash, Answer};
use crate::{LoadedRevision, ProjectError};

/// The format literal every quality report carries in `meta.format`.
pub const QUALITY_FORMAT: &str = "ekr.store-quality/1";

/// What one report returned, counted over the document: `ekr.views.StoreQualityReported`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreQualityReported {
    /// `meta.revision`.
    pub revision: u64,
    /// `assertions.active`.
    pub active_assertions: u64,
    /// `assertions.with_evidence`.
    pub with_evidence: u64,
    /// `assertions.with_item_evidence`.
    pub with_item_evidence: u64,
    /// `properties.declared`.
    pub properties: u64,
    /// `properties.constrained`.
    pub constrained_properties: u64,
    /// Entries of `shared_names`.
    pub shared_names: u64,
    /// `sharing_nodes`.
    pub sharing_nodes: u64,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub quality_hash: String,
}

/// `ekr.views.QualityMeta`.
#[derive(Serialize)]
struct QualityMeta {
    format: &'static str,
    revision: u64,
}

/// `ekr.views.AssertionQuality`.
#[derive(Serialize)]
struct AssertionQuality {
    active: u64,
    with_evidence: u64,
    with_item_evidence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_evidence_share: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    with_item_evidence_share: Option<u64>,
}

/// `ekr.views.PropertyQuality`.
#[derive(Serialize)]
struct PropertyQuality {
    declared: u64,
    constrained: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    constrained_share: Option<u64>,
}

/// `ekr.views.SharedName`.
#[derive(Serialize)]
struct SharedName {
    #[serde(rename = "type")]
    type_id: String,
    name: String,
    nodes: Vec<String>,
}

/// `ekr.views.StoreQualityV1`.
#[derive(Serialize)]
struct StoreQualityV1 {
    meta: QualityMeta,
    assertions: AssertionQuality,
    properties: PropertyQuality,
    shared_names: Vec<SharedName>,
    sharing_nodes: u64,
}

/// `count` of `whole` in basis points, rounded down; `None` for a whole of 0.
fn share(count: u64, whole: u64) -> Option<u64> {
    (whole > 0)
        .then(|| u64::try_from(u128::from(count) * 10_000 / u128::from(whole)).unwrap_or(u64::MAX))
}

/// Reports the quality of revision `at` of `runtime`'s store, or of its head when `at` is
/// `None`.
///
/// # Errors
///
/// What [`crate::load`] refuses — [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`]
/// or the kernel's refusal of the store's history — and the kernel's refusal of the seed's replay.
pub fn report_quality(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
) -> Result<Answer<StoreQualityReported>, ProjectError> {
    let loaded = crate::load(runtime, at)?;
    let seed: BTreeSet<EvidenceId> = if loaded.graph.revision == RevisionNumber::SEED {
        loaded.graph.evidence.keys().copied().collect()
    } else {
        runtime
            .replay(RevisionNumber::SEED)
            .map_err(ProjectError::from)?
            .evidence
            .into_keys()
            .collect()
    };
    quality(&loaded, &seed)
}

/// The pure half of [`report_quality`]: the document of `loaded`, where `seed` is the evidence the
/// store's seed admitted.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] if the document does not encode.
pub fn quality(
    loaded: &LoadedRevision,
    seed: &BTreeSet<EvidenceId>,
) -> Result<Answer<StoreQualityReported>, ProjectError> {
    let graph = &loaded.graph;

    let held = |id: &EvidenceId| {
        graph
            .evidence
            .get(id)
            .is_some_and(|record| loaded.retained.contains(&record.content_hash))
    };
    let after_seed = |id: &EvidenceId| graph.evidence.contains_key(id) && !seed.contains(id);
    let (mut active, mut with_evidence, mut with_item_evidence) = (0_u64, 0_u64, 0_u64);
    for assertion in graph.assertions.values() {
        if !matches!(assertion.lifecycle, AssertionLifecycle::Active) {
            continue;
        }
        active += 1;
        // Evidence attached after the assertion was added counts as cited evidence does
        // (`story:evidence-attaches-to-a-held-assertion`).
        let cited: Vec<EvidenceId> = assertion
            .evidence
            .iter()
            .copied()
            .chain(
                graph
                    .attached(assertion.id)
                    .map(|attached| attached.evidence),
            )
            .map(|e| e.id())
            .collect();
        if cited.iter().any(held) {
            with_evidence += 1;
        }
        if cited.iter().any(after_seed) {
            with_item_evidence += 1;
        }
    }

    let ontology = graph.ontology.to_document();
    let declarations = ontology
        .node_types
        .iter()
        .flat_map(|declared| declared.properties.values())
        .chain(
            ontology
                .edge_types
                .iter()
                .flat_map(|declared| declared.properties.values()),
        );
    let (mut declared, mut constrained) = (0_u64, 0_u64);
    for property in declarations {
        declared += 1;
        if !property.constraints.is_empty() {
            constrained += 1;
        }
    }

    // (type, name) → the nodes of that type holding it, as canonical name or alias.
    let mut holders: BTreeMap<(String, &str), BTreeSet<String>> = BTreeMap::new();
    for node in graph.nodes.values() {
        let names = std::iter::once(node.canonical_name.as_str())
            .chain(node.aliases.iter().map(String::as_str))
            .filter(|name| !name.is_empty());
        for name in names {
            holders
                .entry((node.type_id.to_string(), name))
                .or_default()
                .insert(node.id.to_string());
        }
    }
    let mut sharing = BTreeSet::new();
    let shared_names: Vec<SharedName> = holders
        .into_iter()
        .filter(|(_, nodes)| nodes.len() > 1)
        .map(|((type_id, name), nodes)| {
            sharing.extend(nodes.iter().cloned());
            SharedName {
                type_id,
                name: name.to_owned(),
                nodes: nodes.into_iter().collect(),
            }
        })
        .collect();

    let document = StoreQualityV1 {
        meta: QualityMeta {
            format: QUALITY_FORMAT,
            revision: graph.revision.get(),
        },
        assertions: AssertionQuality {
            active,
            with_evidence,
            with_item_evidence,
            with_evidence_share: share(with_evidence, active),
            with_item_evidence_share: share(with_item_evidence, active),
        },
        properties: PropertyQuality {
            declared,
            constrained,
            constrained_share: share(constrained, declared),
        },
        sharing_nodes: sharing.len() as u64,
        shared_names,
    };
    let bytes = encode(&document)?;
    let summary = StoreQualityReported {
        revision: document.meta.revision,
        active_assertions: active,
        with_evidence,
        with_item_evidence,
        properties: declared,
        constrained_properties: constrained,
        shared_names: document.shared_names.len() as u64,
        sharing_nodes: document.sharing_nodes,
        quality_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}
