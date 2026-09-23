//! Adversary pass 1 on `story:ess-conformance-kernel`.
//!
//! Two kinds of case live here. The first drives the real kernel from the committed suite's own
//! declared observations (`systems/ekr/conformance/suite.json`) without the target in between.
//! The rest wrap the real [`KernelTarget`] and break one observation the story says the target
//! must carry losslessly; a suite that stays green under such a wrapper does not test that
//! observation, so a kernel or projection regression there ships with `task conform-check` green.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ekr::conformance::{KernelTarget, Provider};
use ekr::host::CliHostConfigurationV1;
use ekr_core::Timestamp;
use ekr_kernel::{Runtime, SeedDocument};
use ess_conformance::report::Status;
use ess_conformance::target::{
    ConformanceTarget, EventObservationRequest, ExternalOutcomeControl, ImplementationIdentity,
    ObservedEvent, RedeliveryRequest, ScenarioContext, SemanticCommandRequest,
    SemanticCommandResult, SemanticViewRequest, SemanticViewResult, TargetError,
};
use ess_conformance::{AdmittedSuite, Runner};
use ess_primitives::facts::Number;
use ess_primitives::node::Node;

fn workspace_root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn fixtures() -> PathBuf {
    workspace_root().join("crates/ekr/tests/fixtures/conformance")
}

fn admitted() -> AdmittedSuite {
    let path = workspace_root().join("systems/ekr/conformance/suite.json");
    AdmittedSuite::from_json(&std::fs::read_to_string(path).expect("suite")).expect("admits")
}

type CommandHook = fn(&str, &mut SemanticCommandResult);
type ViewHook = fn(&str, &mut SemanticViewResult);

/// The real target with one observation rewritten after the real handler answered.
struct Tamper<'a> {
    inner: &'a KernelTarget,
    command: CommandHook,
    view: ViewHook,
    seen: RefCell<Vec<(String, SemanticCommandResult)>>,
}

impl<'a> Tamper<'a> {
    fn new(inner: &'a KernelTarget, command: CommandHook, view: ViewHook) -> Self {
        Self {
            inner,
            command,
            view,
            seen: RefCell::new(Vec::new()),
        }
    }
}

impl ConformanceTarget for Tamper<'_> {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        self.inner.identity()
    }
    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.inner.begin_scenario(scenario)
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let mut result = self.inner.execute_command(request)?;
        self.seen
            .borrow_mut()
            .push((command.clone(), result.clone()));
        (self.command)(&command, &mut result);
        Ok(result)
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        let view = request.view.to_string();
        let mut result = self.inner.query_view(request)?;
        (self.view)(&view, &mut result);
        Ok(result)
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        self.inner.observe_events(request)
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        self.inner.configure_external_outcome(request)
    }
    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        self.inner.redeliver_event(request)
    }
    fn end_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.inner.end_scenario(scenario)
    }
}

fn no_command(_: &str, _: &mut SemanticCommandResult) {}
fn no_view(_: &str, _: &mut SemanticViewResult) {}

/// Runs the admitted suite through a tampered real file target; returns statuses and what the
/// real target answered for every command.
fn run(
    command: CommandHook,
    view: ViewHook,
) -> (
    BTreeMap<String, Status>,
    Vec<(String, SemanticCommandResult)>,
) {
    let admitted = admitted();
    let work = tempfile::TempDir::new().expect("work");
    let inner = KernelTarget::new(Provider::File, &fixtures(), work.path()).expect("target");
    let tamper = Tamper::new(&inner, command, view);
    let executed = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &tamper);
    let statuses = executed
        .scenarios
        .iter()
        .map(|s| (s.scenario.to_string(), s.status))
        .collect();
    (statuses, tamper.seen.into_inner())
}

fn failed(statuses: &BTreeMap<String, Status>) -> Vec<&String> {
    statuses
        .iter()
        .filter(|(_, status)| **status != Status::Passed)
        .map(|(name, _)| name)
        .collect()
}

// ---- the suite's own declared observation, driven against the real kernel ------------------

fn store_event_count(root: &Path, name: &str) -> usize {
    let needle = format!("\"{name}\"");
    let mut count = 0;
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "events.jsonl") {
                let text = std::fs::read_to_string(&path).expect("events.jsonl");
                count += text.matches(needle.as_str()).count();
            }
        }
    }
    count
}

/// The `expect_event` / `expect_no_event` steps the admitted suite's `scenario` makes about `event`.
fn suite_expectations(scenario: &str, event: &str) -> (usize, usize) {
    let path = workspace_root().join("systems/ekr/conformance/suite.json");
    let suite: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("suite")).expect("suite JSON");
    let steps = suite["scenarios"][scenario]["steps"]
        .as_array()
        .unwrap_or_else(|| panic!("the admitted suite has no scenario {scenario}"));
    let count = |kind: &str| {
        steps
            .iter()
            .filter(|step| step["step"] == kind && step["event"] == event)
            .count()
    };
    (count("expect_event"), count("expect_no_event"))
}

/// Adversary pass 1 found the real Propose appending `ekr.store.ObjectStored` and
/// `ekr.store.PublicationPrepared` to the provider log while the suite asserted `expect_no_event`
/// for both and passed only because the target never reported them. The coordinator decided the
/// contract instead (kernel.yaml: every durable kernel outcome emits both, with generated fields).
/// So this holds both halves of that decision. The same real Runtime the target and the CLI open,
/// with the target's own fixtures, appends exactly one of each to the provider's physical log per
/// Propose. And `ekr.kernel.Propose/outcome/proposed` expects exactly those two and no longer
/// asserts their absence.
#[test]
fn a_real_propose_publishes_exactly_the_store_events_the_suite_expects() {
    let host = CliHostConfigurationV1::from_json(
        &std::fs::read(fixtures().join("host.json")).expect("host"),
    )
    .expect("host document");
    let work = tempfile::TempDir::new().expect("work");
    let store = work.path().join("store");
    std::fs::create_dir_all(&store).expect("store");
    let open = || {
        Runtime::file(&store, &host.tenant, host.context, host.authority.clone()).expect("runtime")
    };
    let seed = std::fs::read_to_string(fixtures().join("seed.yaml")).expect("seed");
    open()
        .seed(SeedDocument::from_yaml(&seed).expect("seed parses"), || {
            Timestamp::from_millis(1_800_000_000_000)
        })
        .expect("seeded");
    let before_stored = store_event_count(&store, "ekr.store.ObjectStored");
    let before_prepared = store_event_count(&store, "ekr.store.PublicationPrepared");
    let document = std::fs::read(fixtures().join("propose-node.yaml")).expect("proposal");
    open()
        .propose_reader(document.as_slice(), host.context.operator, || {
            Timestamp::from_millis(1_800_000_001_000)
        })
        .expect("proposed");
    let after_stored = store_event_count(&store, "ekr.store.ObjectStored");
    let after_prepared = store_event_count(&store, "ekr.store.PublicationPrepared");
    assert_eq!(
        (
            after_stored - before_stored,
            after_prepared - before_prepared
        ),
        (1, 1),
        "one real Propose appended {} ekr.store.ObjectStored and {} ekr.store.PublicationPrepared \
         occurrences to the provider log; the decided contract is exactly one of each",
        after_stored - before_stored,
        after_prepared - before_prepared,
    );
    for event in ["ekr.store.ObjectStored", "ekr.store.PublicationPrepared"] {
        assert_eq!(
            suite_expectations("ekr.kernel.Propose/outcome/proposed", event),
            (1, 0),
            "ekr.kernel.Propose/outcome/proposed must expect {event} exactly once and never assert \
             its absence, as the store events the real Propose publishes"
        );
    }
}

// ---- observations the suite does not hold the target to --------------------------------------

fn forge_snapshot(command: &str, result: &mut SemanticCommandResult) {
    if command == "ekr.kernel.Snapshot" {
        for event in &mut result.direct_events {
            if event.event.to_string() == "ekr.kernel.SnapshotTaken" {
                event
                    .payload
                    .insert("number".to_owned(), Node::Number(Number::from(424_242_i64)));
                event
                    .payload
                    .insert("knowledge_root".to_owned(), Node::Text("forged".to_owned()));
            }
        }
    }
}

/// A Snapshot that reports a revision it was not asked for and a root no revision holds.
///
/// The generated `ekr.kernel.Snapshot/outcome/taken` cannot hold this: `SnapshotTaken.number`
/// cannot be declared `input.at`, because an absent `at` means the newest revision and ESS
/// refuses the Optional-to-required crossing (ESS-COMMAND-002). The correction holds it with two
/// authored scenarios instead, one naming a past revision and one naming none, and each must
/// fail under the forgery.
#[test]
fn a_snapshot_reporting_the_wrong_revision_fails_the_authored_snapshot_scenarios() {
    let (statuses, _) = run(forge_snapshot, no_view);
    for scenario in [
        "ekr.kernel/authored/snapshot-at-a-past-revision-reports-that-revision",
        "ekr.kernel/authored/snapshot-without-a-revision-reports-the-newest",
    ] {
        assert_eq!(
            statuses.get(scenario),
            Some(&Status::Failed),
            "SnapshotTaken.number 424242 and a forged root passed {scenario}; failing: {:?}",
            failed(&statuses)
        );
    }
}

fn drop_responses(command: &str, result: &mut SemanticCommandResult) {
    if matches!(command, "ekr.kernel.Propose" | "ekr.kernel.Validate") {
        result.response = None;
    }
}

/// Propose and Validate answer no declared response at all, and today no scenario can see it.
///
/// Adversary pass 1, finding 3: ESS 0.29.0 cannot observe a command response from a generated or
/// an authored scenario. The authored format has no response claim, and a payload maps only from a
/// top-level response field. The coordinator filed that as
/// `task:conformance-cannot-observe-command-responses` and decided this case states today's state
/// rather than a negative the suite cannot hold. The lossless projection is held outside the suite,
/// by [`proposal_responses_carry_the_exact_document_bytes`]. When a pinned ESS can observe
/// responses, this case goes red, and it should then become the negative it was written as.
#[test]
fn dropping_the_propose_and_validate_responses_is_not_yet_observable_by_the_suite() {
    let (statuses, seen) = run(drop_responses, no_view);
    assert!(
        seen.iter().any(|(command, result)| command == "ekr.kernel.Propose"
            && result.response.is_some())
            && seen.iter().any(|(command, result)| command == "ekr.kernel.Validate"
                && result.response.is_some()),
        "the real target answered no Propose or Validate response to drop, so this run shows nothing"
    );
    assert!(!statuses.is_empty(), "the suite ran no scenario");
    assert_eq!(
        failed(&statuses),
        Vec::<&String>::new(),
        "a scenario now observes the Propose or Validate response (ESS can observe command \
         responses): close task:conformance-cannot-observe-command-responses and turn this case \
         back into the negative that dropping the responses fails a scenario"
    );
}

fn drop_optionals(view: &str, result: &mut SemanticViewResult) {
    if view == "ekr.kernel.Transactions" {
        for row in &mut result.rows {
            for field in [
                "canonical_transaction_hash",
                "canonical_operations_hash",
                "validation_basis",
                "validation_hash",
                "validation_record_hash",
                "terminal_record_hash",
            ] {
                row.insert(field.to_owned(), Node::Null);
            }
        }
    }
}

/// The all-state Transactions view drops every Optional field it declares except
/// `validated_against`.
#[test]
fn dropping_the_optional_transaction_fields_fails_a_scenario() {
    let (statuses, _) = run(no_command, drop_optionals);
    let failing = failed(&statuses);
    assert!(
        !failing.is_empty(),
        "every scenario passed with validation_basis, validation_hash, validation_record_hash, \
         terminal_record_hash and both canonical hashes nulled on every Transactions row"
    );
}

fn resample_retained_seed(command: &str, result: &mut SemanticCommandResult) {
    if command == "ekr.kernel.Seed"
        && result
            .outcome
            .as_ref()
            .is_some_and(|o| o.outcome.to_string() == "retained-seed")
    {
        if let Some(Node::Map(seed)) = result.response.as_mut().and_then(|r| r.get_mut("result")) {
            seed.insert(
                "committed_at".to_owned(),
                Node::Text("2100-01-01T00:00:00.000Z".to_owned()),
            );
        }
    }
}

/// The Seed analogue of the unit's `ResampledRetainedCommit` control, which it did not write.
#[test]
fn a_retained_seed_retry_with_a_resampled_time_fails_retained_seed() {
    let (statuses, _) = run(resample_retained_seed, no_view);
    assert_eq!(
        statuses.get("ekr.kernel.Seed/outcome/retained-seed"),
        Some(&Status::Failed)
    );
}

// ---- lossless projection of the real record ----------------------------------------------------

fn decode_base64(text: &str) -> Vec<u8> {
    let value = |c: u8| -> u32 {
        match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' => 62,
            b'/' => 63,
            other => panic!("{other} is not base64"),
        }
    };
    assert_eq!(text.len() % 4, 0, "padded base64");
    let mut out = Vec::new();
    for chunk in text.as_bytes().chunks(4) {
        let pad = chunk.iter().filter(|c| **c == b'=').count();
        let mut acc = 0_u32;
        for c in &chunk[..4 - pad] {
            acc = (acc << 6) | value(*c);
        }
        acc <<= 6 * pad as u32;
        let bytes = acc.to_be_bytes();
        out.extend_from_slice(&bytes[1..4 - pad]);
    }
    out
}

/// Every `proposed` answer carries the exact staged document bytes, an integer operation count
/// and millisecond RFC 3339 time.
#[test]
fn proposal_responses_carry_the_exact_document_bytes() {
    let (_, seen) = run(no_command, no_view);
    let node = std::fs::read(fixtures().join("propose-node.yaml")).expect("fixture");
    let mut checked = 0;
    for (command, result) in &seen {
        if command != "ekr.kernel.Propose"
            || result.outcome.as_ref().map(|o| o.outcome.to_string()) != Some("proposed".into())
        {
            continue;
        }
        let Some(Node::Map(record)) = result.response.as_ref().and_then(|r| r.get("result")) else {
            panic!("proposed without a ProposalRecordV1");
        };
        let Some(Node::Text(encoded)) = record.get("document_bytes") else {
            panic!("document_bytes is not text");
        };
        let decoded = decode_base64(encoded);
        let Some(Node::Number(count)) = record.get("operation_count") else {
            panic!("operation_count is not a number");
        };
        if decoded == node {
            checked += 1;
            assert_eq!(count.as_i64(), Some(1));
        }
        let Some(Node::Text(at)) = record.get("submitted_at") else {
            panic!("submitted_at is not text");
        };
        assert_eq!(at.len(), "2027-01-15T08:00:00.000Z".len(), "{at}");
    }
    assert!(
        checked > 0,
        "no proposed response decoded to propose-node.yaml"
    );
}

fn store_bytes(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("read store dir") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "events.jsonl") {
                out.push((path.clone(), std::fs::read(&path).expect("events.jsonl")));
            }
        }
    }
    out.sort();
    out
}

/// `kernel.yaml` retained-seed: "allocate no new occurrence, sample no new time and write
/// nothing". The target checks only kernel occurrences; this reads the provider's whole log.
#[test]
fn a_retained_seed_retry_writes_nothing_to_the_provider_log() {
    let host = CliHostConfigurationV1::from_json(
        &std::fs::read(fixtures().join("host.json")).expect("host"),
    )
    .expect("host document");
    let work = tempfile::TempDir::new().expect("work");
    let store = work.path().join("store");
    std::fs::create_dir_all(&store).expect("store");
    let open = || {
        Runtime::file(&store, &host.tenant, host.context, host.authority.clone()).expect("runtime")
    };
    let seed = std::fs::read_to_string(fixtures().join("seed.yaml")).expect("seed");
    let first = open()
        .seed(SeedDocument::from_yaml(&seed).expect("seed parses"), || {
            Timestamp::from_millis(1_800_000_000_000)
        })
        .expect("seeded");
    let before = store_bytes(&store);
    let again = open()
        .seed(SeedDocument::from_yaml(&seed).expect("seed parses"), || {
            Timestamp::from_millis(1_800_000_009_000)
        })
        .expect("retained");
    assert_eq!(first, again);
    assert!(
        store_bytes(&store) == before,
        "the retained seed retry changed the provider log"
    );
}
