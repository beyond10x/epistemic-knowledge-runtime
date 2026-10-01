//! `ekr sample` and `ekr fact-quality`: fact quality by a judged sample.
//!
//! `sample` prints the `ekr.fact-sample/1` document of one revision
//! (`ekr.views.DrawFactSample`, through `ekr_views::draw_sample`): a reproducible sample of its
//! facts, each with its evidence bytes, for a judge. `fact-quality` reads the judged sample, an
//! `ekr.fact-judgements/1` document, and prints the `ekr.fact-quality/1` report
//! (`ekr.views.ReportFactQuality`, through `ekr_views::report_fact_quality`): the pass rate and
//! its Wilson interval. It opens no store. The runtime judges nothing; the verdicts are the
//! caller's.

use std::io::Read;
use std::path::Path;

use ekr_core::{RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{FactJudgements, FactQualityError, LimitExceeded, SampleRequest};
use serde_json::Value;

use super::view::project_refusal;
use crate::exit::Failure;

/// The most bytes of a judged sample read: 1000 judgements of the largest sample take well under
/// a hundred kilobytes.
const LIMIT: u64 = 8 << 20;

/// The named refusal of an input outside its bound.
const LIMIT_EXCEEDED: &str = "ekr.views.LimitExceeded";

fn limit_exceeded(error: &LimitExceeded) -> Failure {
    Failure::refused(LIMIT_EXCEEDED, error)
}

/// The size, checked before any store is opened: out of its bounds it is
/// `ekr.views.LimitExceeded` (exit 2).
pub(super) fn request(
    seed: i64,
    size: i64,
    type_id: Option<TypeId>,
) -> Result<SampleRequest, Failure> {
    SampleRequest::new(seed, size, type_id).map_err(|error| limit_exceeded(&error))
}

/// The sample `request` names from `revision`, the head when absent. An unseeded store and a
/// revision beyond the head are `ekr.views.NotSeeded` and `ekr.views.RevisionNotFound` (exit 2); a
/// store that could not be read is a fault.
pub(super) fn run(
    runtime: &Runtime,
    revision: Option<u64>,
    request: &SampleRequest,
) -> Result<Value, Failure> {
    let answer = ekr_views::draw_sample(runtime, revision.map(RevisionNumber::new), request)
        .map_err(|error| match project_refusal(&error) {
            Some(name) => Failure::refused(name, error),
            None => Failure::fault(error),
        })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}

/// Reads the judged sample at `document`, or stdin for `-`. A document that cannot be read or is
/// not an `ekr.fact-judgements/1` is a fault naming it.
pub(super) fn read(document: &Path, stdin: &mut dyn Read) -> Result<FactJudgements, Failure> {
    let fault = |error: &dyn std::fmt::Display| {
        Failure::fault(format!("judged sample {}: {error}", document.display()))
    };
    let mut bytes = Vec::new();
    super::input::open(document, stdin)?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| fault(&error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > LIMIT {
        return Err(fault(&format_args!("over {LIMIT} bytes")));
    }
    FactJudgements::from_json(&bytes).map_err(|error| fault(&error))
}

/// The report of `judged` at `confidence` basis points. A confidence outside 1 to 9999 is
/// `ekr.views.LimitExceeded` and an assertion judged twice `ekr.views.JudgedTwice` (exit 2).
pub(super) fn report(judged: &FactJudgements, confidence: Option<i64>) -> Result<Value, Failure> {
    let answer =
        ekr_views::report_fact_quality(judged, confidence).map_err(|error| match error {
            FactQualityError::LimitExceeded(error) => limit_exceeded(&error),
            error @ FactQualityError::JudgedTwice { .. } => {
                Failure::refused("ekr.views.JudgedTwice", error)
            }
        })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}
