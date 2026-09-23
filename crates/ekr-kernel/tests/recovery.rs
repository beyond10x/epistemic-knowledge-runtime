//! Design § 94 recovery on both native providers, for every decision kind.
//!
//! Evidence is labelled by what produced it, as § 94.3 requires:
//!
//! * **process boundary** — the retry runs in a child process of this test binary, which opens
//!   the provider fresh and shares nothing in memory with the parent (cases 1 and 2).
//! * **fresh handle** — the retry reopens the provider from a new handle in this process (case 4).
//! * **port-level fault injection** — `support::Probe` wraps the real store at the kernel's
//!   `RevisionLog` port and answers `UnknownCommit` before or after delegating the exact elected
//!   request (cases 1 and 2). It is not a native crash witness; no case here is one.
//! * **native concurrency** — independent handles on separate threads, no fault injection
//!   (cases 3 and 5); case 3 also has a deterministic port-level interleaving.
#[path = "recovery/support.rs"]
mod support;

use ekr_core::{ContentHash, EventId, Timestamp, TransactionId};
use ekr_store::{NativeExpectedKind, NativeNewEvent, NativeStreamAppend, PublicationPreparationV1};
use std::path::Path;
use support::*;

const CHILD: &str = "EKR_RECOVERY_CHILD";

/// In a child process: reopen fresh, repeat the command with a clock that must not be sampled,
/// and write back what it reported together with what the lineage retains.
fn child() -> bool {
    let Some(path) = std::env::var_os(CHILD) else {
        return false;
    };
    let path = Path::new(&path);
    let plan: Plan =
        serde_json::from_slice(&std::fs::read(path.join("plan.json")).unwrap()).unwrap();
    let kernel = open(path, plan.file);
    let retry = run(&kernel, &plan, no_clock("fresh-process retry"));
    let retained = kernel.retained_record(plan.kind, plan.tx);
    std::fs::write(
        path.join("child.json"),
        serde_json::to_vec(&(retry, retained)).unwrap(),
    )
    .unwrap();
    true
}
fn retry_in_child(path: &Path, plan: &Plan, test: &str) -> (Outcome, Outcome) {
    std::fs::write(path.join("plan.json"), serde_json::to_vec(plan).unwrap()).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child for {:?} file={} failed\n{}\n{}",
        plan.kind,
        plan.file,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&std::fs::read(path.join("child.json")).unwrap()).unwrap()
}
fn time_field(kind: Kind) -> &'static str {
    match kind {
        Kind::Seed | Kind::Committed => "committed_at",
        Kind::Propose => "submitted_at",
        Kind::Validated => "validated_at",
        Kind::Rejected => "rejected_at",
        Kind::Stale => "stale_at",
    }
}
fn record_time(outcome: &Outcome, kind: Kind) -> serde_json::Value {
    match outcome {
        Outcome::Record(value) => value[time_field(kind)].clone(),
        other => panic!("{kind:?}: no record: {other:?}"),
    }
}
fn record_event_id(outcome: &Outcome) -> serde_json::Value {
    match outcome {
        Outcome::Record(value) => value["event_id"].clone(),
        other => panic!("no record: {other:?}"),
    }
}
fn millis(at: i64) -> serde_json::Value {
    serde_json::to_value(Timestamp::from_millis(at)).unwrap()
}
fn slot_events(events: &[serde_json::Value], prepared: &PublicationPreparationV1) -> usize {
    let slot = slot(&prepared.command_key);
    prepared_events(events)
        .iter()
        .filter(|e| e["stream_id"] == slot.as_str())
        .count()
}
/// Elects the command's decision through the probe, which then withholds or loses the response.
fn elect(path: &Path, plan: &Plan, fault: Fault) -> PublicationPreparationV1 {
    let hooks = Hooks::new(fault);
    let first = probed(path, plan.file, &hooks, |kernel| {
        run(kernel, plan, at(plan.kind.time()))
    });
    assert_eq!(
        first,
        Outcome::Unknown,
        "{:?} file={}: the probed first attempt must report an uncertain outcome",
        plan.kind,
        plan.file
    );
    let resumed = hooks.resumed.borrow();
    assert_eq!(resumed.len(), 1, "{:?}: exactly one resume", plan.kind);
    resumed[0].clone()
}

/// Case 1 — process boundary + port-level fault (elected, never sent).
#[test]
fn an_elected_unpublished_decision_resumes_exactly_in_a_fresh_process_for_every_kind() {
    if child() {
        return;
    }
    for file in [false, true] {
        for kind in KINDS {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, kind);
            let elected = elect(path, &plan, Fault::BeforeWrite);
            let event_id = elected.decision.event.event_id;
            let expected = elected_record(kind, &elected);
            let before = events(path, file);
            assert_eq!(slot_events(&before, &elected), 1, "{kind:?} file={file}");
            assert!(
                occurrences(&before, event_id).is_empty(),
                "{kind:?} file={file}: the withheld attempt must not have published"
            );
            let (retry, retained) = retry_in_child(
                path,
                &plan,
                "an_elected_unpublished_decision_resumes_exactly_in_a_fresh_process_for_every_kind",
            );
            assert_eq!(retry, expected, "{kind:?} file={file}: retry decision");
            assert_eq!(
                retained, expected,
                "{kind:?} file={file}: retained decision"
            );
            assert_eq!(
                record_event_id(&retry),
                serde_json::to_value(event_id).unwrap()
            );
            assert_eq!(record_time(&retry, kind), millis(kind.time()));
            let after = events(path, file);
            let published = occurrences(&after, event_id);
            assert_eq!(published.len(), 1, "{kind:?} file={file}: one occurrence");
            assert_eq!(
                published[0]["request_id"],
                elected.native_request.meta.request_id.as_str(),
                "{kind:?} file={file}: the retry must send the elected native request"
            );
            assert_eq!(
                kind_events(&after, kind, plan.tx).len(),
                1,
                "{kind:?} file={file}: no second occurrence of this decision"
            );
            assert_eq!(
                after.len(),
                before.len() + elected.native_request.appends.len(),
                "{kind:?} file={file}: the retry appended exactly the elected request"
            );
            assert_eq!(slot_events(&after, &elected), 1);
            let again = run(&open(path, file), &plan, no_clock("resolved retry"));
            match kind.state() {
                None => assert_eq!(again, expected, "{kind:?} file={file}"),
                Some(state) => assert_eq!(again, Outcome::State(state.into()), "{kind:?}"),
            }
            assert_eq!(events(path, file), after, "{kind:?} file={file}");
        }
    }
}

/// Case 2 — process boundary + port-level fault (sent and applied, response lost).
#[test]
fn a_lost_response_after_the_native_write_reports_the_original_decision_in_a_fresh_process() {
    if child() {
        return;
    }
    for file in [false, true] {
        for kind in KINDS {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let plan = stage(path, file, kind);
            let elected = elect(path, &plan, Fault::AfterWrite);
            let event_id = elected.decision.event.event_id;
            let expected = elected_record(kind, &elected);
            let before = events(path, file);
            assert_eq!(
                occurrences(&before, event_id).len(),
                1,
                "{kind:?} file={file}"
            );
            let (retry, retained) = retry_in_child(
                path,
                &plan,
                "a_lost_response_after_the_native_write_reports_the_original_decision_in_a_fresh_process",
            );
            match kind.state() {
                None => assert_eq!(retry, expected, "{kind:?} file={file}: original success"),
                Some(state) => assert_eq!(
                    retry,
                    Outcome::State(state.into()),
                    "{kind:?} file={file}: the declared retained-state refusal"
                ),
            }
            assert_eq!(
                retained, expected,
                "{kind:?} file={file}: never a new decision"
            );
            let after = events(path, file);
            assert_eq!(
                after, before,
                "{kind:?} file={file}: the retry wrote nothing"
            );
            assert_eq!(kind_events(&after, kind, plan.tx).len(), 1);
        }
    }
}

fn loser_bound_nothing(
    path: &Path,
    file: bool,
    winner: &PublicationPreparationV1,
    candidate: &ekr_store::Publication,
    failures: &mut Vec<String>,
    label: &str,
) {
    let events = events(path, file);
    let hash = candidate.event.record_hash;
    let id = serde_json::to_value(hash).unwrap();
    if !occurrences(&events, candidate.event.event_id).is_empty() {
        failures.push(format!("{label}: losing occurrence published"));
    }
    if blob(path, file, &hash.to_hex()).is_some()
        || events
            .iter()
            .any(|e| e["name"] == "ekr.store.ObjectStored" && e["data"]["content_hash"] == id)
    {
        failures.push(format!("{label}: losing record bound"));
    }
    let loser = would_be(winner, candidate);
    if blob(path, file, &private_key(preparation_hash(&loser))).is_some() {
        failures.push(format!("{label}: losing private preparation bound"));
    }
}
fn check_winner(
    path: &Path,
    file: bool,
    winner: &PublicationPreparationV1,
    failures: &mut Vec<String>,
    label: &str,
) {
    // The reconstruction used for losers must reproduce the winner exactly, or its absence
    // checks prove nothing.
    if &would_be(winner, &winner.decision) != winner {
        failures.push(format!("{label}: preparation reconstruction diverges"));
    }
    if blob(path, file, &private_key(preparation_hash(winner))).is_none() {
        failures.push(format!("{label}: winning private preparation absent"));
    }
    if slot_events(&events(path, file), winner) != 1 {
        failures.push(format!(
            "{label}: slot does not hold exactly one preparation"
        ));
    }
}

/// Case 3 — same-slot contention: two deterministic port-level interleavings, then a native race.
///
/// * `before-read`: the rival publishes before the loser's `prepare` reads the slot, so the loser
///   adopts the winner on that read.
/// * `inside-prepare`: the rival publishes from inside the loser's own `prepare`, after its slot
///   read and before its conditional selection write (the store's admission of its own candidate
///   calls the injected kernel authority there). The loser's write must be refused and it must
///   adopt the winner. `fired_inside_prepare` shows the seam was reached, every time.
#[test]
fn same_slot_contention_elects_one_preparation_and_the_loser_binds_nothing() {
    let mut failures = Vec::new();
    for seam in ["before-read", "inside-prepare"] {
        for file in [false, true] {
            for kind in KINDS {
                let label = format!("{seam} {kind:?} file={file}");
                interleaved(seam, file, kind, &label, &mut failures);
            }
        }
    }
    raced(&mut failures);
    assert!(failures.is_empty(), "{failures:#?}");
}
fn interleaved(seam: &str, file: bool, kind: Kind, label: &str, failures: &mut Vec<String>) {
    let label = label.to_owned();
    {
        {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().to_path_buf();
            let plan = stage(&path, file, kind);
            let first = Hooks::new(Fault::Pass);
            let second = Hooks::new(Fault::Pass);
            let second_outcome = std::rc::Rc::new(std::cell::RefCell::new(None));
            {
                let (path, plan, second, out) = (
                    path.clone(),
                    plan.clone(),
                    second.clone(),
                    second_outcome.clone(),
                );
                let rival: Box<dyn FnMut()> = Box::new(move || {
                    *out.borrow_mut() = Some(probed(&path, file, &second, |kernel| {
                        run(kernel, &plan, at(plan.kind.time() + 1))
                    }));
                });
                let at = if seam == "before-read" {
                    &first.before_prepare
                } else {
                    &first.inside_prepare
                };
                *at.borrow_mut() = Some(rival);
            }
            let first_outcome = probed(&path, file, &first, |kernel| {
                run(kernel, &plan, at(kind.time()))
            });
            if seam == "inside-prepare" && !first.fired_inside_prepare.get() {
                failures.push(format!("{label}: the rival never ran inside prepare"));
                return;
            }
            let Some(second_outcome) = second_outcome.borrow().clone() else {
                failures.push(format!("{label}: the rival never ran"));
                return;
            };
            let Some(winner) = second.resumed.borrow().first().cloned() else {
                failures.push(format!("{label}: the interleaved handle never resumed"));
                return;
            };
            let expected = elected_record(kind, &winner);
            if second_outcome != expected || first_outcome != expected {
                failures.push(format!(
                    "{label}: {first_outcome:?} / {second_outcome:?} != {expected:?}"
                ));
            }
            if first.resumed.borrow().iter().any(|p| p != &winner) {
                failures.push(format!("{label}: the loser resumed its own candidate"));
            }
            check_winner(&path, file, &winner, failures, &label);
            let events = events(&path, file);
            if kind_events(&events, kind, plan.tx).len() != 1 {
                failures.push(format!("{label}: not exactly one occurrence"));
            }
            for candidate in first.candidates.borrow().iter() {
                loser_bound_nothing(&path, file, &winner, candidate, failures, &label);
            }
        }
    }
}
/// Native threads, no seam. What this can show is only that both handles built a candidate
/// before either saw the other's election; whether the loser reached its conditional write is
/// not observable at the port, which is why `inside-prepare` exists.
fn raced(failures: &mut Vec<String>) {
    let mut both_built_candidates = [0usize; 2];
    for file in [false, true] {
        for kind in KINDS {
            for round in 0..3 {
                let label = format!("raced {kind:?} file={file} round={round}");
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path();
                let plan = stage(path, file, kind);
                let barrier = std::sync::Barrier::new(2);
                let results = std::thread::scope(|scope| {
                    let handles = [0, 1].map(|offset| {
                        let (plan, barrier) = (&plan, &barrier);
                        scope.spawn(move || {
                            let hooks = Hooks::new(Fault::Pass);
                            let outcome = probed(path, file, &hooks, |kernel| {
                                barrier.wait();
                                run(kernel, plan, at(plan.kind.time() + offset))
                            });
                            let candidates = hooks.candidates.borrow().clone();
                            let resumed = hooks.resumed.borrow().clone();
                            (outcome, candidates, resumed)
                        })
                    });
                    handles.map(|handle| handle.join().unwrap())
                });
                // Both handles built a candidate before either saw an election; nothing more.
                if results.iter().all(|r| !r.1.is_empty()) {
                    both_built_candidates[usize::from(file)] += 1;
                }
                let resumed: Vec<_> = results.iter().flat_map(|r| r.2.clone()).collect();
                let Some(winner) = resumed.first().cloned() else {
                    failures.push(format!("{label}: nobody resumed: {:?}", results[0].0));
                    continue;
                };
                if resumed.iter().any(|p| p != &winner) {
                    failures.push(format!("{label}: two different preparations resumed"));
                }
                let expected = elected_record(kind, &winner);
                for (outcome, _, _) in &results {
                    let retained_refusal = kind.state().map(|s| Outcome::State(s.into()));
                    if outcome != &expected && Some(outcome) != retained_refusal.as_ref() {
                        failures.push(format!("{label}: {outcome:?} is not the elected decision"));
                    }
                }
                check_winner(path, file, &winner, failures, &label);
                if kind_events(&events(path, file), kind, plan.tx).len() != 1 {
                    failures.push(format!("{label}: not exactly one occurrence"));
                }
                for candidate in results.iter().flat_map(|r| r.1.iter()) {
                    if candidate.event.event_id != winner.decision.event.event_id {
                        loser_bound_nothing(path, file, &winner, candidate, failures, &label);
                    }
                }
            }
        }
    }
    // Rounds where one handle finished before the other started exercise nothing concurrent.
    if both_built_candidates.contains(&0) {
        failures.push(format!(
            "raced: no round had both handles build a candidate: [sqlite, file] \
             {both_built_candidates:?}"
        ));
    }
}

/// Case 4 — fresh handle. A genuine elected preparation is copied into an identical pristine
/// store, intact or tampered, exactly where the store's own selection would put it.
#[test]
fn missing_corrupt_and_forged_private_preparations_refuse_by_name_and_change_nothing() {
    const MODES: [&str; 9] = [
        "genuine",
        "missing",
        "corrupt",
        "fingerprint",
        "extra-blob",
        "foreign-stream",
        "metadata",
        "input",
        "record",
    ];
    let mut failures = Vec::new();
    for file in [false, true] {
        for kind in KINDS {
            let work = tempfile::tempdir().unwrap();
            let staged = work.path().join("staged");
            let pristine = work.path().join("pristine");
            std::fs::create_dir_all(&staged).unwrap();
            let plan = stage(&staged, file, kind);
            copy_tree(&staged, &pristine);
            let genuine = elect(&staged, &plan, Fault::BeforeWrite);
            for mode in MODES {
                let label = format!("{kind:?} file={file} {mode}");
                let path = work.path().join(mode);
                copy_tree(&pristine, &path);
                let mut forged = genuine.clone();
                let mut bytes = Some(serde_json::to_vec(&genuine).unwrap());
                let expected = match mode {
                    "genuine" => elected_record(kind, &genuine),
                    "missing" => {
                        bytes = None;
                        Outcome::Refused("preparation-bytes-missing".into())
                    }
                    "corrupt" => {
                        let held = bytes.as_mut().unwrap();
                        held.pop();
                        held.extend_from_slice(b" }");
                        Outcome::Refused("preparation-address".into())
                    }
                    "fingerprint" => {
                        forged.native_fingerprint =
                            ContentHash::of_bytes(b"another request").to_hex();
                        Outcome::Refused("preparation-fingerprint".into())
                    }
                    "extra-blob" => {
                        forged
                            .native_request
                            .blobs
                            .push(ekr_store::NativeBlobWrite {
                                digest: ContentHash::of_bytes(b"unrelated").to_hex(),
                                bytes: b"unrelated".to_vec(),
                            });
                        forged.native_fingerprint = fingerprint(&forged.native_request);
                        Outcome::Refused("preparation-blob-set".into())
                    }
                    "foreign-stream" => {
                        let mut stream = forged.native_request.appends[0].stream.clone();
                        stream.stream_type = "ekr.foreign".into();
                        forged.native_request.appends.push(NativeStreamAppend {
                            stream,
                            expected: ekr_store::NativeExpected {
                                kind: NativeExpectedKind::NoStream,
                                version: None,
                            },
                            events: vec![NativeNewEvent {
                                name: "ekr.foreign.Event".into(),
                                schema_version: 1,
                                data: b"{}".to_vec(),
                            }],
                        });
                        forged.native_fingerprint = fingerprint(&forged.native_request);
                        Outcome::Refused("preparation-extra-stream".into())
                    }
                    "metadata" => {
                        forged.native_request.meta.actor = "forger".into();
                        forged.native_fingerprint = fingerprint(&forged.native_request);
                        Outcome::Refused("preparation-native-metadata".into())
                    }
                    "input" => {
                        forged.input_hash = ContentHash::of_bytes(b"different input");
                        Outcome::Refused("PublicationInputConflict".into())
                    }
                    "record" => {
                        // A complete, self-consistent forgery: new record bytes, new addresses
                        // and the provider's own fingerprint. Only kernel admission can refuse it.
                        let old = forged.decision.event.record_hash;
                        let object = forged.decision.objects.remove(&old).unwrap();
                        let mut value: serde_json::Value =
                            serde_json::from_slice(&object.bytes).unwrap();
                        value["event_id"] = serde_json::to_value(EventId::mint()).unwrap();
                        let changed = serde_json::to_vec(&value).unwrap();
                        let hash = ContentHash::of_bytes(&changed);
                        forged.decision.objects.insert(
                            hash,
                            ekr_store::PublicationObject {
                                bytes: changed,
                                ..object
                            },
                        );
                        forged.decision.event.record_hash = hash;
                        forged = would_be(&genuine, &forged.decision);
                        Outcome::Refused(
                            match kind {
                                Kind::Seed => "seed-result-disagrees",
                                Kind::Propose => "proposal-record-disagrees",
                                Kind::Validated => "validation-record-disagrees",
                                Kind::Rejected => "rejection-record-disagrees",
                                Kind::Committed => "commit-record-linkage",
                                Kind::Stale => "stale-record-disagrees",
                            }
                            .into(),
                        )
                    }
                    _ => unreachable!(),
                };
                if !matches!(mode, "genuine" | "missing" | "corrupt") {
                    bytes = Some(serde_json::to_vec(&forged).unwrap());
                }
                install_preparation(&path, file, &forged, bytes.as_deref());
                let before = events(&path, file);
                let reopened = open(&path, file);
                let head = reopened.head().map_err(|e| format!("{e:?}"));
                let retry = run(&reopened, &plan, no_clock("refused recovery"));
                if retry != expected {
                    failures.push(format!("{label}: got {retry:?}, want {expected:?}"));
                }
                if mode == "genuine" {
                    continue;
                }
                if events(&path, file) != before {
                    failures.push(format!("{label}: the refused recovery wrote"));
                }
                if reopened.head().map_err(|e| format!("{e:?}")) != head {
                    failures.push(format!("{label}: canonical head moved"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Case 5 — review of `165a577`, finding F3: two kernel handles over one store race for
/// revision 1. Native concurrency, no fault injection.
#[test]
fn two_handles_racing_for_revision_one_commit_it_exactly_once_on_both_providers() {
    let mut failures = Vec::new();
    let mut refused_append = [0usize; 2];
    for file in [false, true] {
        for same in [false, true] {
            for round in 0..5 {
                let label = format!("file={file} same={same} round={round}");
                let directory = tempfile::tempdir().unwrap();
                let path = directory.path();
                let plan = stage(path, file, Kind::Committed);
                let seed = plan.seed();
                let other = transaction(&seed, true);
                {
                    let kernel = open(path, file);
                    kernel
                        .propose(&encode(&other), context().operator, || {
                            Timestamp::from_millis(20)
                        })
                        .unwrap();
                    kernel
                        .validate(other.id, ekr_core::RevisionNumber::SEED, || {
                            Timestamp::from_millis(30)
                        })
                        .unwrap();
                }
                let targets: [TransactionId; 2] = if same {
                    [plan.tx, plan.tx]
                } else {
                    [plan.tx, other.id]
                };
                let barrier = std::sync::Barrier::new(2);
                let results = std::thread::scope(|scope| {
                    let handles = [0, 1].map(|i| {
                        let barrier = &barrier;
                        let target = targets[i];
                        scope.spawn(move || {
                            let hooks = Hooks::new(Fault::Pass);
                            let outcome = probed(path, file, &hooks, |kernel| {
                                barrier.wait();
                                kernel.commit_command(target, at(40 + i as i64))
                            });
                            let candidates = hooks.candidates.borrow().len();
                            let resumed = hooks.resumed.borrow().len();
                            (outcome, candidates, resumed)
                        })
                    });
                    handles.map(|handle| handle.join().unwrap())
                });
                // A handle resumed a second time only after its conditional revision append was
                // refused (`Conflict`) and a successor attempt was elected: the race reached the
                // provider's write and lost there. Observable exactly, so it is what is counted.
                if !same && results.iter().any(|r| r.2 >= 2) {
                    refused_append[usize::from(file)] += 1;
                }
                let results = results.map(|r| r.0);
                let events = events(path, file);
                let committed: Vec<_> = events
                    .iter()
                    .filter(|e| e["name"] == "ekr.kernel.RevisionCommitted")
                    .collect();
                if committed.len() != 1 || committed[0]["data"]["payload"]["number"] != 1 {
                    failures.push(format!(
                        "{label}: {} RevisionCommitted events: {:?}",
                        committed.len(),
                        committed
                            .iter()
                            .map(|e| e["data"]["payload"]["number"].clone())
                            .collect::<Vec<_>>()
                    ));
                }
                let receipts = results
                    .iter()
                    .filter(|o| matches!(o, Outcome::Record(v) if v.get("committed_at").is_some()))
                    .count();
                let stale_or_conflict = results
                    .iter()
                    .filter(|o| match o {
                        Outcome::Record(v) => v.get("stale_at").is_some(),
                        Outcome::Refused(code) => code == "Conflict",
                        _ => false,
                    })
                    .count();
                let consistent = if same {
                    receipts == 2 && results[0] == results[1]
                } else {
                    receipts == 1 && stale_or_conflict == 1
                };
                if !consistent {
                    failures.push(format!("{label}: outcomes {results:?}"));
                }
                match open(path, file).head() {
                    Ok(Some(root)) if root.revision == ekr_core::RevisionNumber::new(1) => {}
                    other => failures.push(format!("{label}: head {other:?}")),
                }
            }
        }
    }
    assert!(
        refused_append.iter().all(|&n| n > 0),
        "no distinct-transaction round reached a refused revision append: [sqlite, file] \
         {refused_append:?}"
    );
    assert!(failures.is_empty(), "{failures:#?}");
}
