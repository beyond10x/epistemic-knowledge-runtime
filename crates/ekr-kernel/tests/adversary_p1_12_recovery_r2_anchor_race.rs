//! Adversary pass 2 on unit p1-12-recovery: the Bootstrap anchor mapping (`commit.rs` seed path).
//!
//! The correction maps `AuthorityMismatch` to `PublicationInputConflict` on the *pre-read* of the
//! Bootstrap slot only. A second host with a different anchor can elect between that read and
//! this host's own `prepare` — the window is exactly the trusted clock sample, which the seed
//! path takes after the read and before the election. This case puts the other host's election
//! inside that clock.
//!
//! Labels (§ 94.3): **fresh handle** in-process; the other host's election is **port-level fault
//! injection** (`support::Probe`, `Fault::BeforeWrite`), not a native crash witness.
#[allow(dead_code)]
#[path = "recovery/support.rs"]
mod support;

use ekr_core::{AgentId, Timestamp};
use ekr_store::PublicationPreparationV1;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use support::*;

/// The other host's elected preparation and the native events it observed.
type Observed = Rc<RefCell<Option<(PublicationPreparationV1, Vec<serde_json::Value>)>>>;

fn changed_anchor() -> ekr_kernel::AuthorityStateV1 {
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
    changed
}

fn open_changed(path: &Path, file: bool) -> ekr_kernel::Runtime {
    if file {
        ekr_kernel::Runtime::file(path, TENANT, context(), changed_anchor())
    } else {
        ekr_kernel::Runtime::sqlite(&path.join("state.db"), TENANT, context(), changed_anchor())
    }
    .unwrap()
}

/// § 94.1 / § 94.3: different bootstrap input sharing one slot is a publication input conflict,
/// whether the other host's election is seen by the pre-read or lands just after it.
#[test]
fn adv2_a_bootstrap_elected_under_another_anchor_during_the_clock_sample_is_an_input_conflict() {
    let mut failures = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path: PathBuf = directory.path().to_path_buf();
        let plan = stage(&path, file, Kind::Seed);
        let host = open_changed(&path, file);

        let other: Observed = Rc::default();
        let clock = {
            let other = other.clone();
            let path = path.clone();
            let plan = plan.clone();
            Box::new(move || {
                // The other host, original anchor, elects its seed and withholds the write.
                let hooks = Hooks::new(Fault::BeforeWrite);
                let first = probed(&path, file, &hooks, |k| run(k, &plan, at(10)));
                assert_eq!(first, Outcome::Unknown, "file={file}: other host withheld");
                let elected = hooks.resumed.borrow()[0].clone();
                *other.borrow_mut() = Some((elected, events(&path, file)));
                Timestamp::from_millis(99)
            }) as Box<dyn FnOnce() -> Timestamp>
        };
        let outcome = host.seed_command(plan.seed(), clock);
        let Some((elected, after_other)) = other.borrow_mut().take() else {
            failures.push(format!(
                "file={file}: the clock was never sampled: {outcome:?}"
            ));
            continue;
        };
        if outcome != Outcome::Refused("PublicationInputConflict".into()) {
            failures.push(format!(
                "file={file}: a bootstrap elected under another anchor inside the clock window \
                 got {outcome:?}, want Refused(\"PublicationInputConflict\")"
            ));
        }
        if events(&path, file) != after_other {
            failures.push(format!("file={file}: the losing host wrote"));
        }
        drop(host);
        let original = open(&path, file).seed_command(plan.seed(), no_clock("original anchor"));
        if original != elected_record(Kind::Seed, &elected) {
            failures.push(format!(
                "file={file}: the other host's elected seed no longer resumes: {original:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
