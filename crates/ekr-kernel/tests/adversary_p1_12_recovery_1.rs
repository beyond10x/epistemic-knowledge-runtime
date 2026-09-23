//! Adversary pass 1 on unit p1-12-recovery: § 94 paths the unit's five cases do not reach.
//!
//! Labels, as § 94.3 requires:
//! * **fresh handle** — the retry reopens the provider from a new handle in this process.
//! * **process boundary** — `adv1_two_processes_*`: two child processes of this binary.
//! * **port-level fault injection** — `support::Probe` withholding the elected request
//!   (`Fault::BeforeWrite`); not a native crash witness.
#[allow(dead_code)]
#[path = "recovery/support.rs"]
mod support;

use ekr_core::{AgentId, ContentHash, EventId, RevisionNumber, Timestamp, TransactionId};
use ekr_store::{Publication, PublicationPreparationV1};
use std::cell::Cell;
use std::path::Path;
use std::rc::Rc;
use support::*;

fn elect_with(
    path: &Path,
    file: bool,
    f: impl FnOnce(&dyn Commands) -> Outcome,
) -> PublicationPreparationV1 {
    let hooks = Hooks::new(Fault::BeforeWrite);
    let first = probed(path, file, &hooks, f);
    assert_eq!(
        first,
        Outcome::Unknown,
        "file={file}: withheld first attempt"
    );
    let resumed = hooks.resumed.borrow();
    assert_eq!(resumed.len(), 1, "file={file}: exactly one resume");
    resumed[0].clone()
}
fn elect(path: &Path, plan: &Plan) -> PublicationPreparationV1 {
    elect_with(path, plan.file, |k| run(k, plan, at(plan.kind.time())))
}
/// A clock that records whether it was sampled instead of panicking, so both providers run.
fn watched(flag: &Rc<Cell<bool>>) -> Box<dyn FnOnce() -> Timestamp> {
    let flag = flag.clone();
    Box::new(move || {
        flag.set(true);
        Timestamp::from_millis(99)
    })
}
fn field(outcome: &Outcome, name: &str) -> serde_json::Value {
    match outcome {
        Outcome::Record(value) => value[name].clone(),
        _ => serde_json::Value::Null,
    }
}
fn ms(at: i64) -> serde_json::Value {
    serde_json::to_value(Timestamp::from_millis(at)).unwrap()
}
fn named(events: &[serde_json::Value], name: &str) -> Vec<serde_json::Value> {
    events
        .iter()
        .filter(|e| e["stream_type"] == "ekr.revision" && e["name"] == name)
        .cloned()
        .collect()
}
fn slot_count(events: &[serde_json::Value], prepared: &PublicationPreparationV1) -> usize {
    let slot = slot(&prepared.command_key);
    prepared_events(events)
        .iter()
        .filter(|e| e["stream_id"] == slot.as_str())
        .count()
}
/// Proposes and validates a second admissible transaction against the seed.
fn second_validated(path: &Path, file: bool, plan: &Plan) -> TransactionId {
    let other = transaction(&plan.seed(), true);
    let kernel = open(path, file);
    kernel
        .propose(&encode(&other), context().operator, || {
            Timestamp::from_millis(20)
        })
        .unwrap();
    kernel
        .validate(other.id, RevisionNumber::SEED, || {
            Timestamp::from_millis(30)
        })
        .unwrap();
    other.id
}

/// § 94.3 "different bootstrap inputs sharing one slot" / § 94.1 "Seed binds ... actual
/// authority anchor": a pending Bootstrap slot retried under a changed, valid host anchor must
/// return the typed publication input conflict, write nothing and sample no clock.
#[test]
fn adv1_a_pending_bootstrap_under_a_changed_host_anchor_is_a_publication_input_conflict() {
    let mut failures = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let plan = stage(path, file, Kind::Seed);
        let elected = elect(path, &plan);
        let before = events(path, file);
        let mut changed = anchor();
        let extra: AgentId = "00000000-0000-4000-8000-000000000007".parse().unwrap();
        changed.agents.insert(
            extra,
            ekr_kernel::Agent {
                id: extra,
                name: "observer".into(),
                capabilities: Default::default(),
            },
        );
        let opened = if file {
            ekr_kernel::Runtime::file(path, TENANT, context(), changed)
        } else {
            ekr_kernel::Runtime::sqlite(&path.join("state.db"), TENANT, context(), changed)
        };
        let kernel = match opened {
            Ok(kernel) => kernel,
            Err(error) => {
                failures.push(format!(
                    "file={file}: changed anchor refused at open: {error:?}"
                ));
                continue;
            }
        };
        let sampled = Rc::new(Cell::new(false));
        let outcome = kernel.seed_command(plan.seed(), watched(&sampled));
        if outcome != Outcome::Refused("PublicationInputConflict".into()) {
            failures.push(format!(
                "file={file}: changed-anchor retry of a pending seed got {outcome:?}, \
                 want Refused(\"PublicationInputConflict\")"
            ));
        }
        if sampled.get() {
            failures.push(format!(
                "file={file}: changed-anchor retry sampled the clock"
            ));
        }
        if events(path, file) != before {
            failures.push(format!("file={file}: changed-anchor retry wrote"));
        }
        let original = open(path, file).seed_command(plan.seed(), no_clock("original anchor"));
        if original != elected_record(Kind::Seed, &elected) {
            failures.push(format!(
                "file={file}: original anchor did not resume: {original:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// § 94.3 "different bootstrap inputs sharing one slot", same anchor, different seed document.
#[test]
fn adv1_a_pending_bootstrap_with_a_different_seed_document_is_a_publication_input_conflict() {
    let mut failures = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let plan = stage(path, file, Kind::Seed);
        let elected = elect(path, &plan);
        let before = events(path, file);
        let sampled = Rc::new(Cell::new(false));
        let outcome = open(path, file).seed_command(seed_fixture(), watched(&sampled));
        if outcome != Outcome::Refused("PublicationInputConflict".into()) || sampled.get() {
            failures.push(format!(
                "file={file}: {outcome:?} sampled={}",
                sampled.get()
            ));
        }
        if events(path, file) != before {
            failures.push(format!("file={file}: different-input retry wrote"));
        }
        let original = open(path, file).seed_command(plan.seed(), no_clock("original seed"));
        if original != elected_record(Kind::Seed, &elected) {
            failures.push(format!(
                "file={file}: original did not resume: {original:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// § 94.2: an unresolved Commit is retried exactly; only the definite conflict caused by a
/// competing canonical winner authorizes a distinct Stale successor, which keeps the elected time.
/// Fresh handle + port-level fault.
#[test]
fn adv1_an_unresolved_commit_overtaken_by_a_competing_commit_becomes_stale_at_its_elected_time() {
    let mut failures = Vec::new();
    for file in [false, true] {
        let label = format!("file={file}");
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let plan = stage(path, file, Kind::Committed);
        let other = second_validated(path, file, &plan);
        let elected = elect(path, &plan);
        let winner = open(path, file).commit_command(other, at(50));
        if field(&winner, "committed_at") != ms(50) {
            failures.push(format!("{label}: competitor did not commit: {winner:?}"));
            continue;
        }
        let hooks = Hooks::new(Fault::Pass);
        let retry = probed(path, file, &hooks, |k| {
            k.commit_command(plan.tx, no_clock("overtaken retry"))
        });
        if field(&retry, "stale_at") != ms(40) {
            failures.push(format!(
                "{label}: want Stale at the elected 40 ms, got {retry:?}"
            ));
        }
        if field(&retry, "event_id")
            == serde_json::to_value(elected.decision.event.event_id).unwrap()
        {
            failures.push(format!("{label}: Stale reused the Commit occurrence id"));
        }
        let resumed = hooks.resumed.borrow().clone();
        if resumed.first() != Some(&elected) {
            failures.push(format!(
                "{label}: the exact elected attempt was not retried first"
            ));
        }
        match resumed.last() {
            Some(last)
                if last.attempt_number == 1
                    && last.previous_attempt_hash == Some(preparation_hash(&elected)) => {}
            other => failures.push(format!(
                "{label}: no attempt-1 successor: {:?}",
                other.map(|p| (p.attempt_number, p.previous_attempt_hash))
            )),
        }
        let events = events(path, file);
        let committed = named(&events, "ekr.kernel.RevisionCommitted");
        if committed.len() != 1
            || committed[0]["data"]["payload"]["transaction_id"]
                != serde_json::to_value(other).unwrap()
        {
            failures.push(format!("{label}: {} RevisionCommitted", committed.len()));
        }
        if kind_events(&events, Kind::Stale, plan.tx).len() != 1 {
            failures.push(format!(
                "{label}: not exactly one TransactionStale for the loser"
            ));
        }
        if slot_count(&events, &elected) != 2 {
            failures.push(format!(
                "{label}: slot holds {} selections, want 2",
                slot_count(&events, &elected)
            ));
        }
        let again = open(path, file).commit_command(plan.tx, no_clock("resolved stale"));
        if again != Outcome::State("Stale".into()) {
            failures.push(format!("{label}: later retry {again:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// § 94.2 "With an unchanged canonical basis, retain the domain occurrence/time and prepare a new
/// native attempt": unrelated proposal movement while a decision is unresolved.
/// Fresh handle + port-level fault.
#[test]
fn adv1_unrelated_movement_during_uncertainty_keeps_the_elected_occurrence_for_every_kind() {
    let mut failures = Vec::new();
    for file in [false, true] {
        for kind in [
            Kind::Propose,
            Kind::Validated,
            Kind::Rejected,
            Kind::Committed,
        ] {
            let label = format!("{kind:?} file={file}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, kind);
            let elected = elect(path, &plan);
            let unrelated = transaction(&plan.seed(), true);
            open(path, file)
                .propose(&encode(&unrelated), context().operator, || {
                    Timestamp::from_millis(45)
                })
                .unwrap();
            let hooks = Hooks::new(Fault::Pass);
            let retry = probed(path, file, &hooks, |k| {
                run(k, &plan, no_clock("moved retry"))
            });
            let expected = elected_record(kind, &elected);
            if retry != expected {
                failures.push(format!("{label}: {retry:?} != elected {expected:?}"));
            }
            let resumed = hooks.resumed.borrow().clone();
            if resumed.first() != Some(&elected)
                || resumed
                    .last()
                    .map(|p| (p.attempt_number, &p.decision.event))
                    != Some((1, &elected.decision.event))
            {
                failures.push(format!(
                    "{label}: resumed attempts {:?}",
                    resumed.iter().map(|p| p.attempt_number).collect::<Vec<_>>()
                ));
            }
            let events = events(path, file);
            if occurrences(&events, elected.decision.event.event_id).len() != 1
                || kind_events(&events, kind, plan.tx).len() != 1
            {
                failures.push(format!("{label}: not exactly the elected occurrence"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// F3, deterministic: both transactions elect a revision-1 Commit and neither is resolved; the
/// retries then run in either order. Exactly one RevisionCommitted for number 1.
#[test]
fn adv1_two_unresolved_revision_one_commits_resolve_to_one_commit_in_either_order() {
    let mut failures = Vec::new();
    for file in [false, true] {
        for first_a in [true, false] {
            let label = format!("file={file} a_first={first_a}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, Kind::Committed);
            let b = second_validated(path, file, &plan);
            let a = plan.tx;
            let _ea = elect_with(path, file, |k| k.commit_command(a, at(40)));
            let _eb = elect_with(path, file, |k| k.commit_command(b, at(41)));
            let order = if first_a { [a, b] } else { [b, a] };
            let outcomes =
                order.map(|tx| open(path, file).commit_command(tx, no_clock("both pending")));
            if field(&outcomes[0], "committed_at").is_null()
                || field(&outcomes[1], "stale_at").is_null()
            {
                failures.push(format!("{label}: {outcomes:?}"));
            }
            let events = events(path, file);
            let committed = named(&events, "ekr.kernel.RevisionCommitted");
            if committed.len() != 1 || committed[0]["data"]["payload"]["number"] != 1 {
                failures.push(format!("{label}: {} RevisionCommitted", committed.len()));
            }
            match open(path, file).head() {
                Ok(Some(root)) if root.revision == RevisionNumber::new(1) => {}
                other => failures.push(format!("{label}: head {other:?}")),
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

const CHILD: &str = "EKR_ADV1_RACE_DIR";
const WHICH: &str = "EKR_ADV1_RACE_WHICH";
const RACE: &str = "adv1_two_processes_racing_for_revision_one_commit_it_exactly_once";

/// F3 across a real process boundary: two child processes, each with its own provider handle,
/// released together, commit different transactions against revision 0.
#[test]
fn adv1_two_processes_racing_for_revision_one_commit_it_exactly_once() {
    if let Some(dir) = std::env::var_os(CHILD) {
        let dir = Path::new(&dir);
        let which: usize = std::env::var(WHICH).unwrap().parse().unwrap();
        let (file, targets): (bool, [TransactionId; 2]) =
            serde_json::from_slice(&std::fs::read(dir.join("race.json")).unwrap()).unwrap();
        let hooks = Hooks::new(Fault::Pass);
        let outcome = probed(dir, file, &hooks, |kernel| {
            std::fs::write(dir.join(format!("ready-{which}")), b"").unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while !dir.join("go").exists() {
                assert!(std::time::Instant::now() < deadline, "never released");
                std::thread::sleep(std::time::Duration::from_micros(200));
            }
            kernel.commit_command(targets[which], at(40 + which as i64))
        });
        let candidates = hooks.candidates.borrow().len();
        std::fs::write(
            dir.join(format!("out-{which}.json")),
            serde_json::to_vec(&(outcome, candidates)).unwrap(),
        )
        .unwrap();
        return;
    }
    let mut failures = Vec::new();
    let mut contended = [0usize; 2];
    for file in [false, true] {
        for round in 0..4 {
            let label = format!("file={file} round={round}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, Kind::Committed);
            let b = second_validated(path, file, &plan);
            std::fs::write(
                path.join("race.json"),
                serde_json::to_vec(&(file, [plan.tx, b])).unwrap(),
            )
            .unwrap();
            let children = [0, 1].map(|which| {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args([RACE, "--exact", "--nocapture", "--test-threads=1"])
                    .env(CHILD, path)
                    .env(WHICH, which.to_string())
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            });
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
            while !(path.join("ready-0").exists() && path.join("ready-1").exists()) {
                assert!(
                    std::time::Instant::now() < deadline,
                    "{label}: children never ready"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
            std::fs::write(path.join("go"), b"").unwrap();
            let mut outcomes = Vec::new();
            for (which, child) in children.into_iter().enumerate() {
                let output = child.wait_with_output().unwrap();
                if !output.status.success() {
                    failures.push(format!(
                        "{label}: child {which} failed\n{}\n{}",
                        String::from_utf8_lossy(&output.stdout),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                    continue;
                }
                let (outcome, candidates): (Outcome, usize) = serde_json::from_slice(
                    &std::fs::read(path.join(format!("out-{which}.json"))).unwrap(),
                )
                .unwrap();
                outcomes.push((outcome, candidates));
            }
            if outcomes.len() != 2 {
                continue;
            }
            if outcomes.iter().all(|(_, c)| *c > 0) {
                contended[usize::from(file)] += 1;
            }
            let committed_n = outcomes
                .iter()
                .filter(|(o, _)| !field(o, "committed_at").is_null())
                .count();
            let lost = outcomes
                .iter()
                .filter(|(o, _)| {
                    !field(o, "stale_at").is_null() || *o == Outcome::Refused("Conflict".into())
                })
                .count();
            if committed_n != 1 || lost != 1 {
                failures.push(format!("{label}: outcomes {outcomes:?}"));
            }
            let events = events(path, file);
            let committed = named(&events, "ekr.kernel.RevisionCommitted");
            if committed.len() != 1 || committed[0]["data"]["payload"]["number"] != 1 {
                failures.push(format!("{label}: {} RevisionCommitted", committed.len()));
            }
        }
    }
    assert!(
        contended.iter().all(|&n| n > 0),
        "no cross-process round had both children prepare: [sqlite, file] {contended:?}"
    );
    assert!(failures.is_empty(), "{failures:#?}");
}

/// § 94.3 "Read commands never recover by writing", with every kind's decision pending.
#[test]
fn adv1_reads_over_a_pending_preparation_write_nothing_for_every_kind() {
    let mut failures = Vec::new();
    for file in [false, true] {
        for kind in KINDS {
            let label = format!("{kind:?} file={file}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, kind);
            let elected = elect(path, &plan);
            let before = events(path, file);
            let kernel = open(path, file);
            let _ = kernel.head();
            let _ = kernel.snapshot();
            let _ = kernel.read(None);
            let _ = kernel.transactions();
            let _ = kernel.replay(RevisionNumber::SEED);
            let _ = kernel.content(&elected.decision.event.record_hash);
            let _ = kernel.retained_record(kind, plan.tx);
            if events(path, file) != before {
                failures.push(format!("{label}: a read wrote"));
            }
            if !occurrences(&events(path, file), elected.decision.event.event_id).is_empty() {
                failures.push(format!("{label}: a read published the pending decision"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Appends a successor selection to a real elected slot, exactly as the store's own chain does.
fn install_successor(path: &Path, file: bool, next: &PublicationPreparationV1) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId};
    let hash = preparation_hash(next);
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        provider
            .put_blob(
                &tenant(),
                &private_key(hash),
                &serde_json::to_vec(next).unwrap(),
            )
            .await
            .unwrap();
        let stream = StreamId::new(tenant(), "ekr.preparation", slot(&next.command_key)).unwrap();
        let key = format!("fixture.{hash}");
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        let event = NewEvent::new(
            "ekr.store.PublicationPrepared",
            1,
            serde_json::json!({
                "preparation_hash": hash,
                "attempt_number": next.attempt_number,
                "previous_attempt_hash": next.previous_attempt_hash,
            }),
        )
        .unwrap();
        provider
            .append(
                &stream,
                Expected::Exact(next.attempt_number),
                &[event],
                &meta,
            )
            .await
            .unwrap();
    });
}
/// A self-consistent decision for the same command under a fresh occurrence id.
fn reminted(decision: &Publication) -> Publication {
    let mut next = decision.clone();
    let old = next.event.record_hash;
    let object = next.objects.remove(&old).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&object.bytes).unwrap();
    let id = EventId::mint();
    value["event_id"] = serde_json::to_value(id).unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    let hash = ContentHash::of_bytes(&bytes);
    next.objects
        .insert(hash, ekr_store::PublicationObject { bytes, ..object });
    next.event.record_hash = hash;
    next.event.event_id = id;
    next
}

/// § 91.6 "never ... blindly republish under a new id": a forged attempt-1 successor carrying a
/// complete, kernel-admissible decision under a fresh occurrence id is refused by name.
#[test]
fn adv1_a_forged_successor_under_a_new_occurrence_id_is_refused_and_changes_nothing() {
    let mut failures = Vec::new();
    for file in [false, true] {
        for kind in [Kind::Propose, Kind::Validated, Kind::Committed] {
            let label = format!("{kind:?} file={file}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, kind);
            let genuine = elect(path, &plan);
            let mut next = would_be(&genuine, &reminted(&genuine.decision));
            next.attempt_number = 1;
            next.previous_attempt_hash = Some(preparation_hash(&genuine));
            let key = format!("ekr.occurrence.{}.1", next.decision.event.event_id);
            next.native_request.meta.idempotency_key = key.clone();
            next.native_request.meta.request_id = key.clone();
            next.native_request.meta.trace_id = key;
            next.native_fingerprint = fingerprint(&next.native_request);
            install_successor(path, file, &next);
            let before = events(path, file);
            let retry = run(&open(path, file), &plan, no_clock("forged successor"));
            if retry != Outcome::Refused("preparation-decision-changed".into()) {
                failures.push(format!("{label}: {retry:?}"));
            }
            if events(path, file) != before {
                failures.push(format!("{label}: the refused recovery wrote"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Writes a first selection for `bytes` into the slot of `key`, whatever key the bytes name.
fn install_under(path: &Path, file: bool, key: &ekr_store::PublicationCommandKey, bytes: &[u8]) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId};
    let hash = ContentHash::of_bytes(bytes);
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        provider
            .put_blob(&tenant(), &private_key(hash), bytes)
            .await
            .unwrap();
        let stream = StreamId::new(tenant(), "ekr.preparation", slot(key)).unwrap();
        let id = format!("fixture.{hash}");
        let meta = CommandMeta {
            idempotency_key: id.clone(),
            request_hash: id.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: id.clone(),
            trace_id: id,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        let event = NewEvent::new(
            "ekr.store.PublicationPrepared",
            1,
            serde_json::json!({
                "preparation_hash": hash,
                "attempt_number": 0,
                "previous_attempt_hash": null,
            }),
        )
        .unwrap();
        provider
            .append(&stream, Expected::NoStream, &[event], &meta)
            .await
            .unwrap();
    });
}

/// A preparation elected by one decision kind (Validate) placed in the slot of another (the
/// Commit slot its own published validation opens), as-is and relabelled to that key.
#[test]
fn adv1_a_validate_preparation_in_the_commit_slot_is_refused_by_name_and_changes_nothing() {
    let mut failures = Vec::new();
    for file in [false, true] {
        for relabel in [false, true] {
            let label = format!("file={file} relabel={relabel}");
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, Kind::Validated);
            let validation = elect(path, &plan);
            let published = run(&open(path, file), &plan, no_clock("publish validation"));
            if published != elected_record(Kind::Validated, &validation) {
                failures.push(format!("{label}: validation not published: {published:?}"));
                continue;
            }
            let commit_key = ekr_store::PublicationCommandKey {
                kind: ekr_store::PublicationCommandKind::Commit,
                transaction_id: Some(plan.tx),
                predecessor_event_id: Some(validation.decision.event.event_id),
                predecessor_record_hash: Some(validation.decision.event.record_hash),
            };
            let mut moved = validation.clone();
            if relabel {
                moved.command_key = commit_key.clone();
            }
            install_under(
                path,
                file,
                &commit_key,
                &serde_json::to_vec(&moved).unwrap(),
            );
            let before = events(path, file);
            let sampled = Rc::new(Cell::new(false));
            let retry = open(path, file).commit_command(plan.tx, watched(&sampled));
            let want = if relabel {
                "preparation-command-decision"
            } else {
                "preparation-record-chain"
            };
            if retry != Outcome::Refused(want.into()) || sampled.get() {
                failures.push(format!(
                    "{label}: {retry:?} sampled={}, want {want}",
                    sampled.get()
                ));
            }
            if events(path, file) != before {
                failures.push(format!("{label}: the refused recovery wrote"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
