//! Adversary pass 2 on `story:ekr-cli`: every arm of the `exit.rs` mapping, pinned one by one, and
//! the library seam `ekr::cli::run` against the binary's clap behaviour.

use ekr::exit::Failure;
use ekr_kernel::{
    CommitError, DocumentError, PersistenceError, ProjectionError, SeedError, TransactionState,
};

fn pinned(failure: Failure, code: u8, name: Option<&str>, case: &str) {
    assert_eq!(failure.code(), code, "{case}: {failure}");
    assert_eq!(failure.name(), name, "{case}: {failure}");
}

#[test]
fn every_kernel_error_maps_to_its_declared_exit_and_name() {
    let transaction = "00000000-0000-4000-8000-000000000601".parse().unwrap();
    let assertion = "00000000-0000-4000-8000-000000000501".parse().unwrap();
    let agent = "00000000-0000-4000-8000-000000000101".parse().unwrap();
    let io = || std::io::Error::new(std::io::ErrorKind::IsADirectory, "a directory");

    let cases: Vec<(Failure, u8, Option<&str>, &str)> = vec![
        (
            SeedError::Invalid("seed-unsupported-source".into()).into(),
            2,
            Some("ekr.kernel.InvalidSeed"),
            "seed invalid",
        ),
        (
            SeedError::Store(PersistenceError::AlreadySeeded).into(),
            2,
            Some("ekr.kernel.AlreadySeeded"),
            "seed already seeded",
        ),
        (
            SeedError::Store(PersistenceError::Backend("disk".into())).into(),
            1,
            None,
            "seed store fault",
        ),
        (
            SeedError::Store(PersistenceError::Document("corrupt".into())).into(),
            1,
            None,
            "seed corrupt retained document",
        ),
        (
            CommitError::Document(DocumentError::Io(io())).into(),
            1,
            None,
            "document read fault",
        ),
        (
            CommitError::Document(DocumentError::InvalidUtf8).into(),
            2,
            Some("ekr.kernel.StructurallyInvalid"),
            "document not utf-8",
        ),
        (
            CommitError::ProposalAttribution { actor: agent }.into(),
            2,
            Some("ekr.kernel.ProposalAttribution"),
            "attribution",
        ),
        (
            CommitError::RevisionNotFound {
                against: ekr_core::RevisionNumber::new(9),
            }
            .into(),
            2,
            Some("ekr.kernel.RevisionNotFound"),
            "revision",
        ),
        (
            CommitError::TransactionStateConflict {
                transaction_id: transaction,
                state: TransactionState::Rejected,
            }
            .into(),
            2,
            Some("ekr.kernel.TransactionStateConflict"),
            "state",
        ),
        (
            CommitError::TransactionNotFound {
                transaction_id: transaction,
            }
            .into(),
            2,
            Some("ekr.kernel.TransactionNotFound"),
            "transaction",
        ),
        (CommitError::NotSeeded.into(), 1, None, "not seeded"),
        (
            CommitError::Store(PersistenceError::Backend("disk".into())).into(),
            1,
            None,
            "commit store fault",
        ),
        (
            ProjectionError::AssertionNotFound {
                requested: assertion,
            }
            .into(),
            2,
            Some("ekr.kernel.AssertionNotFound"),
            "assertion",
        ),
        (
            ProjectionError::Unverified {
                code: "evidence-payload-mismatch".into(),
            }
            .into(),
            1,
            None,
            "unverified support",
        ),
        (
            Failure::Usage {
                message: "usage".into(),
            },
            2,
            None,
            "usage",
        ),
    ];
    for (failure, code, name, case) in cases {
        pinned(failure, code, name, case);
    }
}

#[test]
fn the_library_seam_keeps_the_binarys_help_and_version_exit() {
    for flag in ["--help", "--version"] {
        let binary = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
            .arg(flag)
            .output()
            .unwrap();
        let seam = match ekr::cli::run(
            ["ekr", flag],
            &|| ekr_core::Timestamp::from_millis(0),
            &mut std::io::empty(),
        ) {
            Ok(_) => 0,
            Err(failure) => i32::from(failure.code()),
        };
        assert_eq!(
            Some(seam),
            binary.status.code(),
            "{flag}: `ekr::cli::run` reports a different exit than the binary"
        );
    }
}
