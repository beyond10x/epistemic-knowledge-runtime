//! Adversary pass 2 on `story:ess-conformance-kernel`, against the correction `ddd83d1`.
//!
//! Each case wraps the real [`KernelTarget`] over the file provider and changes one thing, either
//! in the request before the real handler runs or in the observation after it answered. A case
//! asserting `Failed` for some scenario states that the admitted suite holds that observation; a
//! suite that stays green under the wrapper does not.

use std::collections::BTreeMap;
use std::path::PathBuf;

use ekr::conformance::{KernelTarget, Provider};
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

fn admitted() -> AdmittedSuite {
    let path = workspace_root().join("systems/ekr/conformance/suite.json");
    AdmittedSuite::from_json(&std::fs::read_to_string(path).expect("suite")).expect("admits")
}

type RequestHook = fn(&mut SemanticCommandRequest);
type CommandHook = fn(&str, &mut SemanticCommandResult);
type ViewHook = fn(&str, &mut SemanticViewResult);

struct Wrap<'a> {
    inner: &'a KernelTarget,
    request: RequestHook,
    command: CommandHook,
    view: ViewHook,
}

impl ConformanceTarget for Wrap<'_> {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        self.inner.identity()
    }
    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.inner.begin_scenario(scenario)
    }
    fn execute_command(
        &self,
        mut request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        (self.request)(&mut request);
        let command = request.command.to_string();
        let mut result = self.inner.execute_command(request)?;
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

fn no_request(_: &mut SemanticCommandRequest) {}
fn no_command(_: &str, _: &mut SemanticCommandResult) {}
fn no_view(_: &str, _: &mut SemanticViewResult) {}

fn run(request: RequestHook, command: CommandHook, view: ViewHook) -> BTreeMap<String, Status> {
    let admitted = admitted();
    let work = tempfile::TempDir::new().expect("work");
    let fixtures = workspace_root().join("crates/ekr/tests/fixtures/conformance");
    let inner = KernelTarget::new(Provider::File, &fixtures, work.path()).expect("target");
    let wrap = Wrap {
        inner: &inner,
        request,
        command,
        view,
    };
    let executed = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &wrap);
    executed
        .scenarios
        .iter()
        .map(|s| (s.scenario.to_string(), s.status))
        .collect()
}

fn not_passed(statuses: &BTreeMap<String, Status>) -> Vec<&String> {
    statuses
        .iter()
        .filter(|(_, status)| **status != Status::Passed)
        .map(|(name, _)| name)
        .collect()
}

/// A different, equally well-formed hash: the last hex digit changed.
fn perturb(node: &Node) -> Node {
    match node {
        Node::Text(text) => {
            let mut chars: Vec<char> = text.chars().collect();
            if let Some(last) = chars.last_mut() {
                *last = if *last == '0' { '1' } else { '0' };
            }
            Node::Text(chars.into_iter().collect())
        }
        other => other.clone(),
    }
}

// ---- 1. the store-event payloads the correction now projects -------------------------------

/// Every store event the real provider log gained keeps its name, with a payload a kernel that
/// stored its canonical records as deletable Ephemeral bytes, of length zero, under another hash
/// would publish.
fn forge_store_payloads(_: &str, result: &mut SemanticCommandResult) {
    for event in &mut result.direct_events {
        match event.event.to_string().as_str() {
            "ekr.store.ObjectStored" => {
                let hash = perturb(event.payload.get("content_hash").unwrap_or(&Node::Null));
                event.payload.insert("content_hash".to_owned(), hash);
                event.payload.insert(
                    "storage_class".to_owned(),
                    Node::Text("Ephemeral".to_owned()),
                );
                event
                    .payload
                    .insert("byte_len".to_owned(), Node::Number(Number::from(0_i64)));
            }
            "ekr.store.PublicationPrepared" => {
                let hash = perturb(event.payload.get("preparation_hash").unwrap_or(&Node::Null));
                event.payload.insert("preparation_hash".to_owned(), hash);
                event.payload.insert(
                    "attempt_number".to_owned(),
                    Node::Number(Number::from(99_i64)),
                );
            }
            _ => {}
        }
    }
}

/// The correction's `provider_events` projects each store event's logged payload into the
/// declared fields. Rewriting every one of those values to a wrong, well-typed value must fail a
/// scenario, or the projection (and the storage class the kernel wrote) is checked by nothing.
#[test]
fn forged_store_event_payloads_fail_a_scenario() {
    let statuses = run(no_request, forge_store_payloads, no_view);
    assert!(!statuses.is_empty(), "the suite ran no scenario");
    assert!(
        !not_passed(&statuses).is_empty(),
        "every one of {} scenarios passed with every ObjectStored reporting storage_class \
         Ephemeral, byte_len 0 and another content_hash, and every PublicationPrepared reporting \
         attempt 99 under another preparation_hash",
        statuses.len()
    );
}

// ---- 2. the authored Snapshot scenarios, against a kernel that ignores `at` ----------------

fn drop_snapshot_at(request: &mut SemanticCommandRequest) {
    if request.command.to_string() == "ekr.kernel.Snapshot" {
        request.input.insert("at".to_owned(), Node::Null);
    }
}

/// A kernel that reads the newest revision whatever `at` names is caught by the authored
/// past-revision scenario: the request reaches the real handler without `at`.
#[test]
fn a_snapshot_that_ignores_at_fails_the_past_revision_scenario() {
    let statuses = run(drop_snapshot_at, no_command, no_view);
    assert_eq!(
        statuses.get("ekr.kernel/authored/snapshot-at-a-past-revision-reports-that-revision"),
        Some(&Status::Failed),
        "a Snapshot that ignores `at` passed; not passing: {:?}",
        not_passed(&statuses)
    );
}

// ---- 3. the Transactions row's two fields held only by `defined(...)` ----------------------

/// Every Transactions row keeps a value in each Optional field, but `canonical_transaction_hash`
/// and `validation_basis.previous_root_hash` are another, well-formed hash.
fn misreport_row_hashes(view: &str, result: &mut SemanticViewResult) {
    if view != "ekr.kernel.Transactions" {
        return;
    }
    for row in &mut result.rows {
        if let Some(hash) = row.get("canonical_transaction_hash").map(perturb) {
            row.insert("canonical_transaction_hash".to_owned(), hash);
        }
        if let Some(Node::Map(basis)) = row.get_mut("validation_basis") {
            if let Some(hash) = basis.get("previous_root_hash").map(perturb) {
                basis.insert("previous_root_hash".to_owned(), hash);
            }
            if let Some(hash) = basis.get("previous_record_hash").map(perturb) {
                basis.insert("previous_record_hash".to_owned(), hash);
            }
        }
    }
}

/// The authored scenario `a-committed-transaction-row-carries-every-record-it-names` holds these
/// by `defined(...)` only, and today no scenario can hold their values.
///
/// Adversary pass 2, finding B, decided no-op by the coordinator. No event publishes
/// `canonical_transaction_hash` or the validation basis. Those values reach a caller only in the
/// Propose and Validate responses, and ESS 0.29.0 cannot observe a command response from any
/// scenario (`task:conformance-cannot-observe-command-responses`). So this case states today's
/// state: the misreported row passes every scenario. When a pinned ESS can observe responses,
/// this case goes red, and it should then become the negative it was written as.
#[test]
fn a_transactions_row_with_the_wrong_canonical_and_basis_hashes_is_not_yet_observable() {
    let statuses = run(no_request, no_command, misreport_row_hashes);
    assert!(!statuses.is_empty(), "the suite ran no scenario");
    assert_eq!(
        not_passed(&statuses),
        Vec::<&String>::new(),
        "a scenario now catches a Transactions row reporting another canonical_transaction_hash \
         and validation_basis: close task:conformance-cannot-observe-command-responses and turn \
         this case back into the negative that the misreported row fails a scenario"
    );
}
