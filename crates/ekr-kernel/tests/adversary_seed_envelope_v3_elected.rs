//! Adversary pass, wave ingest-02 unit V: decisions elected and never published (design §§ 100.2,
//! 100.3), built with the recovery probe (`recovery/support.rs`) on both providers.
#[allow(dead_code)]
#[path = "recovery/support.rs"]
mod support;

use std::path::Path;

use ekr_core::ContentHash;
use support::{at, blob, no_clock, open, probed, run, stage, Fault, Hooks, Kind, Outcome};

fn destination(directory: &Path, file: bool) -> ekr_kernel::Runtime {
    std::fs::create_dir_all(directory).unwrap();
    open(directory, file)
}

#[test]
fn migrate_refuses_an_elected_unpublished_decision_writes_nothing_and_succeeds_once_it_is_resumed()
{
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let plan = stage(directory.path(), file, Kind::Propose);
        let outcome = probed(
            directory.path(),
            file,
            &Hooks::new(Fault::BeforeWrite),
            |k| run(k, &plan, at(Kind::Propose.time())),
        );
        assert_eq!(outcome, Outcome::Unknown, "file={file}");

        let source = open(directory.path(), file);
        let target = tempfile::tempdir().unwrap();
        let into = destination(&target.path().join("migrated"), file);
        let refused = source.migrate_into(&into).unwrap_err().to_string();
        assert!(
            refused.contains("migrate-unresolved-preparation"),
            "file={file}: {refused}"
        );
        assert!(
            into.published_events().unwrap().is_empty(),
            "file={file}: a refused migration wrote to its destination"
        );

        // "run that command again first": the same command resumes it, and the same destination
        // then takes the migration.
        assert!(
            matches!(run(&source, &plan, no_clock("resumed")), Outcome::Record(_)),
            "file={file}"
        );
        let report = source.migrate_into(&into).unwrap();
        assert_eq!(report.occurrences.len(), 2, "file={file}");
    }
}

#[test]
fn an_elected_seed_never_published_leaves_its_payload_bound_but_unreadable_and_unmigratable() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let plan = stage(directory.path(), file, Kind::Seed);
        let outcome = probed(
            directory.path(),
            file,
            &Hooks::new(Fault::BeforeWrite),
            |k| run(k, &plan, at(Kind::Seed.time())),
        );
        assert_eq!(outcome, Outcome::Unknown, "file={file}");
        let (hash, bytes) = plan
            .seed()
            .evidence_payloads
            .into_iter()
            .next()
            .expect("the fixture seed carries a payload");
        assert_eq!(ContentHash::of_bytes(&bytes), hash);

        // Bound by the /3 preparation's own group, as § 100.2 says ...
        assert_eq!(
            blob(directory.path(), file, &hash.to_hex()),
            Some(bytes.to_vec()),
            "file={file}"
        );
        // ... and reached by no read.
        let source = open(directory.path(), file);
        assert_eq!(source.content(&hash).unwrap(), None, "file={file}");
        assert!(source.read(None).is_err(), "file={file}");
        assert!(
            !source
                .published_events()
                .unwrap()
                .iter()
                .any(|event| event.name == "ekr.store.ObjectStored"),
            "file={file}"
        );

        let target = tempfile::tempdir().unwrap();
        let into = destination(&target.path().join("migrated"), file);
        let refused = source.migrate_into(&into).unwrap_err().to_string();
        assert!(
            refused.contains("migrate-unresolved-preparation"),
            "file={file}: {refused}"
        );
        assert!(into.published_events().unwrap().is_empty(), "file={file}");
    }
}
