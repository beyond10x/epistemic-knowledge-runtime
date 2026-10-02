//! Fact quality by a judged sample (`views.yaml`, the fact-quality paragraph of the summary).
//!
//! * `DrawFactSample` and its format `ekr.fact-sample/1` ([`draw_sample`]): a reproducible sample
//!   of one revision's Active assertions, each with the bytes of the evidence it cites, for a
//!   judge. The sample is a function of the [`SampleRequest`] and the revision read; [`sample`]
//!   is the pure half over one [`VerifiedRead`].
//! * `ReportFactQuality` and its format `ekr.fact-quality/1` ([`report_fact_quality`]): the pass
//!   rate of a judged sample, [`FactJudgements`], with its Wilson score interval at a stated
//!   confidence. Arithmetic over its input only: it reads no store.
//!
//! The runtime judges nothing (AGENTS.md invariant 7): every verdict is the caller's.

use std::collections::BTreeSet;

use ekr_core::{AssertionId, RevisionNumber, TypeId};
use ekr_graph::{AssertionLifecycle, CanonicalDependency, Object, Predicate, Subject};
use ekr_kernel::{CommitError, Runtime, VerifiedRead};
use ekr_ontology::Ontology;
use serde::{Deserialize, Serialize};

use crate::document::{self, ProjectedAssertion};
use crate::query::{bounded, encode, hash, Answer};
use crate::{LimitExceeded, ProjectError};

/// The format literal every fact sample carries in `meta.format`.
pub const SAMPLE_FORMAT: &str = "ekr.fact-sample/1";

/// The format literal every fact-quality report carries in `meta.format`.
pub const FACT_QUALITY_FORMAT: &str = "ekr.fact-quality/1";

/// The format literal of the judged sample a report reads, [`FactJudgements`].
pub const JUDGEMENTS_FORMAT: &str = "ekr.fact-judgements/1";

// ---- the draw ----------------------------------------------------------------------------------

/// A bounded `DrawFactSample` request: the seed, how many facts to draw, and the type whose facts
/// alone are drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleRequest {
    seed: i64,
    size: u64,
    type_id: Option<TypeId>,
}

impl SampleRequest {
    /// The greatest admitted size.
    pub const MAX_SIZE: i64 = 1000;

    /// `seed` is any integer; `size` is 1 to [`Self::MAX_SIZE`]; `type_id`, when given, keeps the
    /// facts whose subject is of exactly that type.
    ///
    /// # Errors
    ///
    /// [`LimitExceeded`] naming `size`.
    pub fn new(seed: i64, size: i64, type_id: Option<TypeId>) -> Result<Self, LimitExceeded> {
        let size = bounded("size", size, 1, Some(Self::MAX_SIZE))?;
        Ok(Self {
            seed,
            size,
            type_id,
        })
    }

    /// The seed.
    #[must_use]
    pub const fn seed(&self) -> i64 {
        self.seed
    }

    /// How many facts to draw.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// The type filter.
    #[must_use]
    pub const fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }
}

/// What one draw returned, counted over the document: `ekr.views.FactSampleDrawn`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactSampleDrawn {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.seed`.
    pub seed: i64,
    /// `meta.size`.
    pub size: u64,
    /// `meta.population`.
    pub population: u64,
    /// `meta.drawn`.
    pub drawn: u64,
    /// The evidence entries of every item.
    pub evidence: u64,
    /// The first item's assertion id.
    pub first_assertion: Option<AssertionId>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub sample_hash: String,
}

/// `ekr.views.FactSampleMeta`.
#[derive(Serialize)]
struct FactSampleMeta {
    format: &'static str,
    revision: u64,
    seed: i64,
    size: u64,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    type_id: Option<String>,
    population: u64,
    drawn: u64,
}

/// `ekr.views.SampledEvidence`.
#[derive(Serialize)]
struct SampledEvidence {
    id: String,
    kind: &'static str,
    locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
    content_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    base64: Option<String>,
}

/// `ekr.views.SampledFact`.
#[derive(Serialize)]
struct SampledFact {
    subject_kind: &'static str,
    subject: String,
    subject_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    subject_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    predicate_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_name: Option<String>,
    assertion: ProjectedAssertion,
    evidence: Vec<SampledEvidence>,
}

/// `ekr.views.FactSampleV1`.
#[derive(Serialize)]
struct FactSampleV1 {
    meta: FactSampleMeta,
    items: Vec<SampledFact>,
}

/// The draw key of `assertion` under `seed`: the lowercase hex SHA-256 of the UTF-8 text
/// `ekr.fact-sample/1:<seed>:<assertion id>`. A sample is the facts with the lowest keys.
#[must_use]
pub fn draw_key(seed: i64, assertion: AssertionId) -> String {
    hash(format!("{SAMPLE_FORMAT}:{seed}:{assertion}").as_bytes())
}

/// Draws the sample `request` names from revision `at` of `runtime`'s store, or from its head
/// when `at` is `None`.
///
/// # Errors
///
/// [`ProjectError::NotSeeded`], [`ProjectError::RevisionNotFound`], or the kernel's refusal of
/// the store's history as [`ProjectError::Read`].
pub fn draw_sample(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    request: &SampleRequest,
) -> Result<Answer<FactSampleDrawn>, ProjectError> {
    let head = || -> Result<RevisionNumber, ProjectError> {
        Ok(runtime
            .head()?
            .ok_or(ProjectError::NotSeeded { requested: at })?
            .revision)
    };
    let read = runtime.read(at).map_err(|error| match error {
        CommitError::RevisionNotFound { against } => match head() {
            Ok(head) => ProjectError::RevisionNotFound {
                requested: against,
                head,
            },
            Err(error) => error,
        },
        CommitError::NotSeeded => ProjectError::NotSeeded { requested: at },
        other => other.into(),
    })?;
    sample(&read, request)
}

/// The type a subject is of: a node's type, an edge's type, or a Type subject's own id.
fn subject_type(read: &VerifiedRead, subject: &Subject) -> Option<TypeId> {
    match subject {
        Subject::Node(node) => read.graph.nodes.get(&node.id()).map(|node| node.type_id),
        Subject::Edge(edge) => read.graph.edges.get(&edge.id()).map(|edge| edge.type_id),
        Subject::Type(type_id) => Some(*type_id),
    }
}

/// The name the ontology gives the type `type_id`, a node type's or an edge type's.
fn type_name(ontology: &Ontology, type_id: TypeId) -> Option<String> {
    ontology
        .node_type(type_id)
        .map(|declared| declared.name.clone())
        .or_else(|| {
            ontology
                .edge_type(type_id)
                .map(|declared| declared.name.clone())
        })
}

/// The name the type `type_id` gives the property `property`, or, where it does not declare it,
/// the first node type by id that `type_id` conforms to and that does.
fn property_name(
    ontology: &Ontology,
    type_id: TypeId,
    property: ekr_core::PropertyId,
) -> Option<String> {
    let own = ontology
        .node_type(type_id)
        .and_then(|declared| declared.properties.get(&property))
        .or_else(|| {
            ontology
                .edge_type(type_id)
                .and_then(|declared| declared.properties.get(&property))
        })
        .map(|definition| definition.name.clone());
    own.or_else(|| {
        ontology
            .to_document()
            .node_types
            .iter()
            .filter(|declared| ontology.conforms_to(type_id, declared.id))
            .find_map(|declared| {
                declared
                    .properties
                    .get(&property)
                    .map(|definition| definition.name.clone())
            })
    })
}

/// Standard padded base64 (RFC 4648 § 4).
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0_u32, |acc, (at, byte)| {
            acc | (u32::from(*byte) << (16 - 8 * at))
        });
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((triple >> (18 - 6 * position)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The pure half of [`draw_sample`]: the sample `request` names from the revision `read` holds.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] if the document does not encode.
pub fn sample(
    read: &VerifiedRead,
    request: &SampleRequest,
) -> Result<Answer<FactSampleDrawn>, ProjectError> {
    let graph = &read.graph;
    let mut ranked: Vec<(String, String, &ekr_graph::Assertion)> = graph
        .assertions
        .values()
        .filter(|assertion| matches!(assertion.lifecycle, AssertionLifecycle::Active))
        .filter(|assertion| {
            request
                .type_id
                .is_none_or(|wanted| subject_type(read, &assertion.subject) == Some(wanted))
        })
        .map(|assertion| {
            (
                draw_key(request.seed, assertion.id),
                assertion.id.to_string(),
                assertion,
            )
        })
        .collect();
    let population = ranked.len() as u64;
    ranked.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    ranked.truncate(usize::try_from(request.size).unwrap_or(usize::MAX));

    let first_assertion = ranked.first().map(|(_, _, assertion)| assertion.id);
    let ontology = &graph.ontology;
    let mut evidence_count = 0_u64;
    let items: Vec<SampledFact> = ranked
        .into_iter()
        .map(|(_, _, assertion)| {
            let (subject_kind, subject, subject_name) = match &assertion.subject {
                Subject::Node(node) => (
                    "Node",
                    node.id().to_string(),
                    graph
                        .nodes
                        .get(&node.id())
                        .map(|node| node.canonical_name.clone()),
                ),
                Subject::Edge(edge) => ("Edge", edge.id().to_string(), None),
                Subject::Type(type_id) => {
                    ("Type", type_id.to_string(), type_name(ontology, *type_id))
                }
            };
            let of_type = subject_type(read, &assertion.subject);
            let predicate_name = match assertion.predicate {
                Predicate::Property(property) => {
                    of_type.and_then(|type_id| property_name(ontology, type_id, property))
                }
                Predicate::Relation(edge_type) => type_name(ontology, edge_type),
            };
            let object_name = match &assertion.object {
                Object::Node(node) => graph
                    .nodes
                    .get(&node.id())
                    .map(|node| node.canonical_name.clone()),
                Object::Value(_) | Object::Type(_) => None,
            };
            let mut evidence: Vec<SampledEvidence> = assertion
                .evidence
                .iter()
                .filter_map(|cited| graph.evidence.get(&CanonicalDependency::id(cited)))
                .map(|record| {
                    let bytes = read.content(&record.content_hash);
                    let (text, base64) = match bytes.map(std::str::from_utf8) {
                        Some(Ok(text)) => (Some(text.to_owned()), None),
                        Some(Err(_)) => (None, bytes.map(self::base64)),
                        None => (None, None),
                    };
                    SampledEvidence {
                        id: record.id.to_string(),
                        kind: document::evidence_kind(record.source.kind()),
                        locator: record.source.locator(),
                        section: record.source.section().map(str::to_owned),
                        content_hash: record.content_hash.to_string(),
                        text,
                        base64,
                    }
                })
                .collect();
            evidence.sort_by(|a, b| a.id.cmp(&b.id));
            evidence_count += evidence.len() as u64;
            SampledFact {
                subject_kind,
                subject,
                subject_type: of_type
                    .map(|type_id| type_id.to_string())
                    .unwrap_or_default(),
                subject_name,
                predicate_name,
                object_name,
                assertion: document::assertion(assertion),
                evidence,
            }
        })
        .collect();

    let document = FactSampleV1 {
        meta: FactSampleMeta {
            format: SAMPLE_FORMAT,
            revision: graph.revision.get(),
            seed: request.seed,
            size: request.size,
            type_id: request.type_id.map(|type_id| type_id.to_string()),
            population,
            drawn: items.len() as u64,
        },
        items,
    };
    let bytes = encode(&document)?;
    let summary = FactSampleDrawn {
        revision: document.meta.revision,
        seed: request.seed,
        size: request.size,
        population,
        drawn: document.meta.drawn,
        evidence: evidence_count,
        first_assertion,
        sample_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}

// ---- the report --------------------------------------------------------------------------------

/// A judge's verdict on one sampled fact: `ekr.views.FactVerdict`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Its evidence supports it.
    Pass,
    /// It does not.
    Fail,
}

/// One judged fact: `ekr.views.FactJudgement`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Judgement {
    /// The sampled assertion.
    pub assertion: AssertionId,
    /// The judge's verdict on it.
    pub verdict: Verdict,
}

/// Which draw a judged sample came from: `ekr.views.SampleOrigin`, as the sample's meta gave it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SampleOrigin {
    /// The revision drawn from.
    pub revision: u64,
    /// The seed.
    pub seed: i64,
    /// The size requested.
    pub size: u64,
    /// The type filter, if any.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<TypeId>,
}

/// The judged sample `ReportFactQuality` reads: the document `ekr.fact-judgements/1`
/// (`ekr.views.FactJudgementsV1`), whose `format` key [`Self::from_json`] requires and
/// [`Self::to_json`] writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "JudgementsWire", into = "JudgementsWire")]
pub struct FactJudgements {
    /// The sample's origin, echoed by the report.
    pub sample: Option<SampleOrigin>,
    /// One judgement per judged fact.
    pub judgements: Vec<Judgement>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum JudgementsFormat {
    #[serde(rename = "ekr.fact-judgements/1")]
    V1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JudgementsWire {
    format: JudgementsFormat,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sample: Option<SampleOrigin>,
    judgements: Vec<Judgement>,
}

impl From<JudgementsWire> for FactJudgements {
    fn from(wire: JudgementsWire) -> Self {
        let JudgementsWire {
            format: JudgementsFormat::V1,
            sample,
            judgements,
        } = wire;
        Self { sample, judgements }
    }
}

impl From<FactJudgements> for JudgementsWire {
    fn from(judged: FactJudgements) -> Self {
        Self {
            format: JudgementsFormat::V1,
            sample: judged.sample,
            judgements: judged.judgements,
        }
    }
}

/// A document that is not an `ekr.fact-judgements/1`.
#[derive(Debug, thiserror::Error)]
#[error("not an {JUDGEMENTS_FORMAT} document: {0}")]
pub struct JudgementsMalformed(String);

impl FactJudgements {
    /// Reads an `ekr.fact-judgements/1` JSON document.
    ///
    /// # Errors
    ///
    /// [`JudgementsMalformed`] for anything else: another format, a missing or unknown key, a
    /// verdict other than `Pass` or `Fail`, an assertion that is no id.
    pub fn from_json(bytes: &[u8]) -> Result<Self, JudgementsMalformed> {
        serde_json::from_slice(bytes).map_err(|error| JudgementsMalformed(error.to_string()))
    }

    /// Writes this judged sample as an `ekr.fact-judgements/1` JSON document.
    #[must_use]
    pub fn to_json(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }
}

/// Why a judged sample was not reported.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum FactQualityError {
    /// `ekr.views.LimitExceeded`, naming `confidence`.
    #[error(transparent)]
    LimitExceeded(#[from] LimitExceeded),
    /// `ekr.views.JudgedTwice`: the first assertion judged a second time.
    #[error("the judged sample judges assertion {assertion} more than once")]
    JudgedTwice {
        /// The assertion.
        assertion: AssertionId,
    },
}

/// What one report returned, counted over the document: `ekr.views.FactQualityReported`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactQualityReported {
    /// `meta.confidence`.
    pub confidence: u64,
    /// `judged`.
    pub judged: u64,
    /// `passed`.
    pub passed: u64,
    /// `failed`.
    pub failed: u64,
    /// 10000 times `rate`, rounded down.
    pub rate_bp: Option<u64>,
    /// 10000 times `lower`, rounded down.
    pub lower_bp: Option<u64>,
    /// 10000 times `upper`, rounded down.
    pub upper_bp: Option<u64>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub fact_quality_hash: String,
}

/// `ekr.views.FactQualityMeta`.
#[derive(Serialize)]
struct FactQualityMeta<'a> {
    format: &'static str,
    confidence: u64,
    z: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample: Option<&'a SampleOrigin>,
}

/// `ekr.views.FactQualityV1`.
#[derive(Serialize)]
struct FactQualityV1<'a> {
    meta: FactQualityMeta<'a>,
    judged: u64,
    passed: u64,
    failed: u64,
    rate: Option<f64>,
    lower: f64,
    upper: f64,
}

/// The confidence when a request states none, in basis points.
pub const DEFAULT_CONFIDENCE: i64 = 9_500;

/// Reports the pass rate of `judged` with its Wilson score interval at `confidence` basis points
/// (1 to 9999; [`DEFAULT_CONFIDENCE`] when `None`).
///
/// # Errors
///
/// [`FactQualityError::LimitExceeded`] for a confidence outside its bounds, checked first; then
/// [`FactQualityError::JudgedTwice`] for the first assertion judged a second time.
pub fn report_fact_quality(
    judged: &FactJudgements,
    confidence: Option<i64>,
) -> Result<Answer<FactQualityReported>, FactQualityError> {
    let confidence = bounded(
        "confidence",
        confidence.unwrap_or(DEFAULT_CONFIDENCE),
        1,
        Some(9_999),
    )?;
    let mut seen = BTreeSet::new();
    let mut passed = 0_u64;
    for judgement in &judged.judgements {
        if !seen.insert(judgement.assertion) {
            return Err(FactQualityError::JudgedTwice {
                assertion: judgement.assertion,
            });
        }
        if judgement.verdict == Verdict::Pass {
            passed += 1;
        }
    }
    let total = judged.judgements.len() as u64;
    let z = wilson_z(confidence);
    let interval = (total > 0).then(|| wilson_interval(passed, total, z));
    let document = FactQualityV1 {
        meta: FactQualityMeta {
            format: FACT_QUALITY_FORMAT,
            confidence,
            z,
            sample: judged.sample.as_ref(),
        },
        judged: total,
        passed,
        failed: total - passed,
        rate: interval.map(|(rate, _, _)| rate),
        lower: interval.map_or(0.0, |(_, lower, _)| lower),
        upper: interval.map_or(1.0, |(_, _, upper)| upper),
    };
    // Finite numbers, integers and texts: the document always encodes.
    let bytes = serde_json::to_vec(&document).expect("an ekr.fact-quality/1 document encodes");
    let basis_points = |share: f64| {
        let scaled = (share * 10_000.0).floor();
        // A share is within [0, 1], so this is 0 to 10000.
        scaled.clamp(0.0, 10_000.0) as u64
    };
    let summary = FactQualityReported {
        confidence,
        judged: total,
        passed,
        failed: total - passed,
        rate_bp: (total > 0).then(|| passed * 10_000 / total),
        lower_bp: Some(basis_points(document.lower)),
        upper_bp: Some(basis_points(document.upper)),
        fact_quality_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}

/// `(rate, lower, upper)` of `passed` of `judged` (above 0) at the quantile `z`:
/// `(2k + z² ∓ z·√(z² + 4k(n − k)/n)) / (2(n + z²))`, held within [0, 1], with `lower` exactly 0
/// when `passed` is 0 and `upper` exactly 1 when it is `judged`.
#[must_use]
pub fn wilson_interval(passed: u64, judged: u64, z: f64) -> (f64, f64, f64) {
    let (k, n) = (passed as f64, judged as f64);
    let z2 = z * z;
    let spread = z * (z2 + 4.0 * k * (n - k) / n).sqrt();
    let denominator = 2.0 * (n + z2);
    let lower = if passed == 0 {
        0.0
    } else {
        ((2.0 * k + z2 - spread) / denominator).clamp(0.0, 1.0)
    };
    let upper = if passed == judged {
        1.0
    } else {
        ((2.0 * k + z2 + spread) / denominator).clamp(0.0, 1.0)
    };
    (k / n, lower, upper)
}

/// The standard normal quantile at `(1 + confidence / 10000) / 2`, for a confidence of 1 to 9999
/// basis points: Wichura's AS 241 (PPND16), with the logarithm it needs taken by a series that
/// uses only the operations IEEE 754 rounds exactly, so every host computes the same value.
#[must_use]
#[expect(
    clippy::excessive_precision,
    reason = "AS 241's coefficients as published, so they can be checked digit for digit; each \
              rounds to the binary64 value nearest it"
)]
pub fn wilson_z(confidence: u64) -> f64 {
    let confidence = confidence.clamp(1, 9_999);
    // p − ½ for p = (1 + c) / 2 is c / 2, exactly this quotient.
    let q = confidence as f64 / 20_000.0;
    if q <= 0.425 {
        let r = 0.180_625 - q * q;
        return q
            * (((((((2.509_080_928_730_122_672_7e3 * r + 3.343_057_558_358_812_810_5e4) * r
                + 6.726_577_092_700_870_085_3e4)
                * r
                + 4.592_195_393_154_987_145_7e4)
                * r
                + 1.373_169_376_550_946_112_5e4)
                * r
                + 1.971_590_950_306_551_442_7e3)
                * r
                + 1.331_416_678_917_843_774_5e2)
                * r
                + 3.387_132_872_796_366_608_0)
            / (((((((5.226_495_278_852_854_561_0e3 * r + 2.872_908_573_572_194_267_4e4) * r
                + 3.930_789_580_009_271_061_0e4)
                * r
                + 2.121_379_430_158_659_586_7e4)
                * r
                + 5.394_196_021_424_751_107_7e3)
                * r
                + 6.871_870_074_920_579_083_0e2)
                * r
                + 4.231_333_070_160_091_125_2e1)
                * r
                + 1.0);
    }
    // 1 − p, exactly this quotient: at least 1 / 20000, so r is at most √ln 20000 < 5.
    let tail = (10_000 - confidence) as f64 / 20_000.0;
    let r = (-ln(tail)).sqrt() - 1.6;
    (((((((7.745_450_142_783_414_076_4e-4 * r + 2.272_384_498_926_918_458_33e-2) * r
        + 2.417_807_251_774_506_117_7e-1)
        * r
        + 1.270_458_252_452_368_382_58)
        * r
        + 3.647_848_324_763_204_605_04)
        * r
        + 5.769_497_221_460_691_405_5)
        * r
        + 4.630_337_846_156_545_295_9)
        * r
        + 1.423_437_110_749_683_577_34)
        / (((((((1.050_750_071_644_416_843_24e-9 * r + 5.475_938_084_995_344_946e-4) * r
            + 1.519_866_656_361_645_719_66e-2)
            * r
            + 1.481_039_764_274_800_745_9e-1)
            * r
            + 6.897_673_349_851_000_045_5e-1)
            * r
            + 1.676_384_830_183_803_849_4)
            * r
            + 2.053_191_626_637_758_821_87)
            * r
            + 1.0)
}

/// The natural logarithm of a positive, normal, finite `x`, from `x = m · 2^e` with `m` in
/// [√½, √2): `2·atanh((m − 1)/(m + 1)) + e·ln 2`, the series summed by Horner's rule over a fixed
/// number of terms. Only addition, subtraction, multiplication and division, which IEEE 754 rounds
/// exactly, so the value does not depend on the host's `libm`.
fn ln(x: f64) -> f64 {
    const MANTISSA: u64 = (1 << 52) - 1;
    const TERMS: u32 = 16;
    let bits = x.to_bits();
    let biased = i32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let mut exponent = biased - 1023;
    let mut m = f64::from_bits((bits & MANTISSA) | (1023 << 52));
    if m > std::f64::consts::SQRT_2 {
        m /= 2.0;
        exponent += 1;
    }
    let s = (m - 1.0) / (m + 1.0);
    let s2 = s * s;
    let mut series = 1.0 / f64::from(2 * TERMS + 1);
    for k in (0..TERMS).rev() {
        series = series * s2 + 1.0 / f64::from(2 * k + 1);
    }
    2.0 * s * series + f64::from(exponent) * std::f64::consts::LN_2
}

#[cfg(test)]
mod tests {
    use super::{base64, ln};

    #[test]
    fn the_logarithm_is_within_two_units_in_the_last_place_of_the_hosts() {
        for x in [
            5e-5,
            1e-4,
            0.001,
            0.025,
            0.05,
            0.075,
            0.1,
            0.3,
            0.5,
            std::f64::consts::FRAC_1_SQRT_2,
            0.9,
            1.0,
            1.5,
            2.0,
            10.0,
        ] {
            let (ours, host) = (ln(x), x.ln());
            assert!(
                (ours - host).abs() <= 2.0 * f64::EPSILON * host.abs().max(1.0),
                "ln {x}: {ours} against {host}"
            );
        }
    }

    #[test]
    fn base64_is_rfc_4648_padded() {
        for (bytes, text) in [
            (&b""[..], ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (&[0xff, 0xfe, 0x00][..], "//4A"),
        ] {
            assert_eq!(base64(bytes), text);
        }
    }
}
