//! A stage begin whose record committed and whose copy did not (design § 107.2).
//!
//! `ekr stage begin` writes `StageBegun` first and copies the store into the stage's tenant after
//! it. When the copy's writes cannot land, the begin abandons the stage as far as that lands and
//! refuses with `stage-copy-failed`, naming the stage, so the caller finishes it with
//! `ekr stage abandon <stage id>`. It must not answer `publication outcome is unknown`, which
//! names no stage and tells the caller to resolve an occurrence that does not exist.
//!
//! The copy is held back by a lock the schema owner takes on the provider's blob table before the
//! begin: the record's group writes no blob, every copy group does. The lock wait is configured
//! longer than the provider's operation deadline, so each copy attempt ends at that deadline
//! before it reaches `COMMIT` — the shape of a copy group too large or too slow for the deadline.
//! No assertion reads a clock: the deadline only decides when an attempt that cannot land stops.
//!
//! PostgreSQL only: set EKR_TEST_POSTGRES_CONFIG and EKR_TEST_POSTGRES_OWNER to file references;
//! EKR_REQUIRE_POSTGRES=1 makes their absence a failure. Run with `--test-threads=1`.
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_kernel::{runtime::PostgresConfiguration, CommitError, Runtime};
use ekr_store::{StageState, StoreError};
use std::path::Path;
use std::sync::Arc;

/// The application and owner configurations, the owner schema applied, or `None` where the
/// PostgreSQL tests do not run.
fn postgres() -> Option<(PostgresConfiguration, PostgresConfiguration)> {
    let Some(path) = std::env::var_os("EKR_TEST_POSTGRES_CONFIG") else {
        assert_ne!(
            std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
            Ok("1"),
            "required real PostgreSQL fixture is missing"
        );
        eprintln!("SKIP real PostgreSQL: EKR_TEST_POSTGRES_CONFIG is unset");
        return None;
    };
    let owner = std::env::var_os("EKR_TEST_POSTGRES_OWNER").expect("owner configuration reference");
    let owner = PostgresConfiguration::read(Path::new(&owner)).unwrap();
    Runtime::postgres_schema(&owner).unwrap();
    Some((PostgresConfiguration::read(Path::new(&path)).unwrap(), owner))
}

/// A connection as the schema owner, over the owner configuration's verified TLS.
fn owner_connection(
    runtime: &tokio::runtime::Runtime,
    owner: &PostgresConfiguration,
) -> tokio_postgres::Client {
    let dsn = std::fs::read_to_string(&owner.connection_file).unwrap();
    assert!(
        owner.password_file.is_none(),
        "the owner fixture carries its password in its connection file"
    );
    let mut roots = rustls::RootCertStore::empty();
    let pem = std::fs::read(&owner.ca_file).unwrap();
    for certificate in
        <rustls::pki_types::CertificateDer<'_> as rustls::pki_types::pem::PemObject>::pem_slice_iter(
            &pem,
        )
    {
        roots.add(certificate.unwrap()).unwrap();
    }
    let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let config: tokio_postgres::Config = dsn.trim().parse().unwrap();
    let (client, connection) = runtime
        .block_on(config.connect(tokio_postgres_rustls::MakeRustlsConnect::new(tls)))
        .unwrap();
    runtime.spawn(connection);
    client
}

#[test]
fn a_begin_whose_copy_cannot_land_names_and_abandons_the_stage_it_began() {
    let Some((mut config, owner)) = postgres() else {
        return;
    };
    // The provider's lock wait outlasts its operation deadline, so a blocked copy attempt ends at
    // the deadline, before COMMIT, as a slow one does.
    config.pool.transaction_timeout_ms = 2_000;
    config.pool.statement_timeout_ms = 600_000;
    config.pool.lock_timeout_ms = 600_000;
    let tenant = format!("begin-copy-{}", ekr_core::TypeId::mint());
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    store.seed(seed(), || SEEDED_AT).unwrap();
    let head = store.head().unwrap().unwrap().revision;

    let io = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let blocker = owner_connection(&io, &owner);
    let blobs = format!("\"{}\".ekr_blobs", owner.schema);
    io.block_on(blocker.batch_execute(&format!(
        "BEGIN; LOCK TABLE {blobs} IN EXCLUSIVE MODE"
    )))
    .unwrap();
    let begun = store.begin_stage();
    io.block_on(blocker.batch_execute("COMMIT")).unwrap();
    // The begin recorded its stage and, under the lock, its abandonment: neither record's group
    // writes a blob. The head did not move.
    let stages = store.stages().unwrap();

    // The begin's answer is not an unknown publication outcome: it names the stage it began.
    let error = match begun {
        Ok(result) => panic!("the copy landed under the lock: {result:?}"),
        Err(error) => error,
    };
    let text = error.to_string();
    assert!(
        !matches!(error, CommitError::Store(StoreError::UnknownCommit)),
        "a begin whose record committed answered an unknown publication outcome, naming no \
         stage: {text}"
    );
    assert_eq!(stages.len(), 1, "the begin's record committed: {stages:?}");
    let left = &stages[0];
    assert_eq!(left.stage.base, head);
    assert_eq!(store.head().unwrap().unwrap().revision, head, "the head did not move");
    assert!(
        text.starts_with("stage-copy-failed: ")
            && text.contains(&format!("stage {} was begun", left.stage.stage_id))
            && text.contains(&format!("ekr stage abandon {}", left.stage.stage_id)),
        "the begin's refusal is stage-copy-failed and names stage {}: {text}",
        left.stage.stage_id
    );
    // The begin abandoned the stage; its tenant's forgetting writes the blob table, which the
    // lock held, so the refusal says the tenant may hold part of the copy.
    assert_eq!(left.state, StageState::Abandoned, "the begin abandoned its stage");
    assert!(
        text.contains("it is Abandoned and its tenant may hold part of the copy"),
        "{text}"
    );

    // `ekr stage abandon <stage id>` finishes it once the lock is gone: Abandoned, tenant empty.
    assert_eq!(
        store.abandon_stage(left.stage.stage_id).unwrap().state,
        StageState::Abandoned
    );
    assert_eq!(store.stage_tenant_events(left.stage.stage_id).unwrap(), 0);
    // The next begin copies and lands.
    let next = store.begin_stage().expect("the next begin lands");
    assert_eq!(next.state, StageState::Begun);
    assert_ne!(next.stage_id, left.stage.stage_id);
    assert_eq!(
        store.abandon_stage(next.stage_id).unwrap().state,
        StageState::Abandoned
    );
}
