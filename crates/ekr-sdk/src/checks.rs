//! Sample facts through `ekr sample`, judge them in the caller, and report their Wilson interval.
//! These wire types follow `ekr.views.DrawFactSample` and `ekr.views.ReportFactQuality`.

use std::num::NonZeroUsize;

use ekr_core::{AssertionId, EvidenceId, TypeId};
use serde::{Deserialize, Serialize};

use crate::read::{OneShotReader, ReadError, Reader, ViewAssertion};
use crate::transport::{Request, Transport};

/// The `ekr.fact-sample/1` document, with items in deterministic draw order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactSample {
    /// The resolved revision and request.
    pub meta: FactSampleMeta,
    /// The sampled assertions and their evidence.
    pub items: Vec<SampledFact>,
}

/// A draw's request and population.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactSampleMeta {
    /// `ekr.fact-sample/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
    /// The reproducible draw seed.
    pub seed: i64,
    /// The requested size.
    pub size: u64,
    /// Optional exact subject type filter.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<TypeId>,
    /// Active assertions satisfying the filter.
    pub population: u64,
    /// Items drawn.
    pub drawn: u64,
}

/// An assertion with the names of its subject, predicate and object where available.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampledFact {
    /// `Node`, `Edge` or `Type`.
    pub subject_kind: String,
    /// The subject's identifier.
    pub subject: String,
    /// The subject's exact type.
    pub subject_type: TypeId,
    /// The node or type name, absent for an edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_name: Option<String>,
    /// The predicate's declared name, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicate_name: Option<String>,
    /// The node object's canonical name, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_name: Option<String>,
    /// The sampled assertion.
    pub assertion: ViewAssertion,
    /// Its retained evidence records, in identifier order.
    pub evidence: Vec<SampledEvidence>,
}

/// Evidence accompanying a sampled assertion. Bytes are UTF-8 text or padded base64.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampledEvidence {
    /// Evidence identifier.
    pub id: EvidenceId,
    /// Evidence source kind.
    pub kind: String,
    /// Source locator.
    pub locator: String,
    /// Section of the source, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
    /// Hash of the retained bytes.
    pub content_hash: String,
    /// UTF-8 bytes as text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Non-UTF-8 bytes as standard padded base64.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
}

/// The caller's verdict; the SDK contains no model or evidence evaluator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// The evidence supports the assertion.
    Pass,
    /// The evidence does not support the assertion.
    Fail,
}

/// One judged assertion.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Judgement {
    /// Assertion identifier from the sample.
    pub assertion: AssertionId,
    /// The caller's decision.
    pub verdict: Verdict,
}

/// Origin echoed by the report; the reporting verb reads no store.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleOrigin {
    /// The sampled revision.
    pub revision: u64,
    /// The draw's seed.
    pub seed: i64,
    /// The requested size.
    pub size: u64,
    /// The exact type filter, when present.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<TypeId>,
}

/// The `ekr.fact-judgements/1` input to `ekr fact-quality -`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactJudgements {
    /// `ekr.fact-judgements/1`.
    pub format: String,
    /// Optional source draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleOrigin>,
    /// Verdicts, one per judged assertion.
    pub judgements: Vec<Judgement>,
}

/// The `ekr.fact-quality/1` report, retaining the verb's binary64 values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactQuality {
    /// Confidence and optional source draw.
    pub meta: FactQualityMeta,
    /// Number judged.
    pub judged: u64,
    /// Number passed.
    pub passed: u64,
    /// Number failed.
    pub failed: u64,
    /// Pass rate; explicit null when nothing was judged.
    pub rate: Option<f64>,
    /// Wilson lower bound, zero when nothing was judged.
    pub lower: f64,
    /// Wilson upper bound, one when nothing was judged.
    pub upper: f64,
}

/// The report's confidence and source.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactQualityMeta {
    /// `ekr.fact-quality/1`.
    pub format: String,
    /// Confidence in basis points.
    pub confidence: u64,
    /// Normal quantile used by the report.
    pub z: f64,
    /// Optional source draw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample: Option<SampleOrigin>,
}

/// A caller-supplied judge of batches of sampled facts.
pub trait Judge {
    /// Why this judge could not complete a batch.
    type Error;

    /// One verdict per fact in the same order, or an error; no report is sent here.
    ///
    /// # Errors
    /// The judge's own failure to complete this batch.
    fn judge(&mut self, facts: &[SampledFact]) -> Result<Vec<Verdict>, Self::Error>;
}

/// A failed judge produces no completed judgement document.
#[derive(Debug, thiserror::Error)]
pub enum JudgeError<E> {
    /// The caller's judge failed.
    #[error("judge failed at sample offset {offset}: {source}")]
    Failed {
        /// The failed batch's first sample index.
        offset: usize,
        /// The caller's error.
        source: E,
    },
    /// A batch did not return exactly one verdict per fact.
    #[error("judge returned {actual} verdicts for {expected} facts at sample offset {offset}")]
    Count {
        /// The batch's first sample index.
        offset: usize,
        /// Facts supplied.
        expected: usize,
        /// Verdicts returned.
        actual: usize,
    },
}

/// Judge consecutive batches in draw order, preserving every assertion's identity.
///
/// # Errors
/// [`JudgeError`] on the first failing batch or verdict-count mismatch. No partial result is
/// returned and no report is submitted.
pub fn judge_sample<J: Judge>(
    sample: &FactSample,
    batch_size: NonZeroUsize,
    judge: &mut J,
) -> Result<FactJudgements, JudgeError<J::Error>> {
    let mut judgements = Vec::with_capacity(sample.items.len());
    for facts in sample.items.chunks(batch_size.get()) {
        let offset = judgements.len();
        let verdicts = judge
            .judge(facts)
            .map_err(|source| JudgeError::Failed { offset, source })?;
        if verdicts.len() != facts.len() {
            return Err(JudgeError::Count {
                offset,
                expected: facts.len(),
                actual: verdicts.len(),
            });
        }
        judgements.extend(facts.iter().zip(verdicts).map(|(fact, verdict)| Judgement {
            assertion: fact.assertion.id,
            verdict,
        }));
    }
    Ok(FactJudgements {
        format: "ekr.fact-judgements/1".to_owned(),
        sample: Some(SampleOrigin {
            revision: sample.meta.revision,
            seed: sample.meta.seed,
            size: sample.meta.size,
            type_id: sample.meta.type_id,
        }),
        judgements,
    })
}

impl<T: Transport> Reader<T> {
    /// Draw `size` facts with `seed`, optionally filtered by exact type and revision.
    ///
    /// # Errors
    /// [`ReadError`], including the verb's size and missing-revision refusals.
    pub fn draw_sample(
        &mut self,
        seed: i64,
        size: i64,
        filter: Option<TypeId>,
        revision: Option<u64>,
    ) -> Result<FactSample, ReadError> {
        let mut argv = vec![
            "sample".to_owned(),
            "--seed".to_owned(),
            seed.to_string(),
            "--size".to_owned(),
            size.to_string(),
        ];
        if let Some(filter) = filter {
            argv.extend(["--type".to_owned(), filter.to_string()]);
        }
        if let Some(revision) = revision {
            argv.extend(["--revision".to_owned(), revision.to_string()]);
        }
        self.read_request(Request::new(argv))
    }

    /// Report the caller's judgements at optional confidence in basis points (default 9500).
    ///
    /// # Errors
    /// [`ReadError`], including invalid confidence and duplicate-assertion refusals.
    pub fn report_judged(
        &mut self,
        judgements: &FactJudgements,
        confidence: Option<i64>,
    ) -> Result<FactQuality, ReadError> {
        let mut argv = vec!["fact-quality".to_owned(), "-".to_owned()];
        if let Some(confidence) = confidence {
            argv.extend(["--confidence".to_owned(), confidence.to_string()]);
        }
        let stdin = serde_json::to_string(judgements).map_err(|source| ReadError::Document {
            verb: "fact-quality".to_owned(),
            source,
        })?;
        self.read_request(Request::new(argv).with_stdin(stdin))
    }
}

impl OneShotReader {
    /// [`Reader::draw_sample`], through a one-shot process.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn draw_sample(
        &mut self,
        seed: i64,
        size: i64,
        filter: Option<TypeId>,
        revision: Option<u64>,
    ) -> Result<FactSample, ReadError> {
        self.reader.draw_sample(seed, size, filter, revision)
    }

    /// [`Reader::report_judged`], through a one-shot process.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn report_judged(
        &mut self,
        judgements: &FactJudgements,
        confidence: Option<i64>,
    ) -> Result<FactQuality, ReadError> {
        self.reader.report_judged(judgements, confidence)
    }
}
