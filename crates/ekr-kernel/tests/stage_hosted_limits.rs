//! Two stage defects on PostgreSQL (design § 107), reproduced before they are fixed.
//!
//! - A begin's copy of a store with many evidence payloads is refused `stage-copy-failed` with
//!   "out of shared memory" on a server whose lock table is the stock size, because the copy
//!   publishes each occurrence as one provider group and that group takes one transaction-scoped
//!   advisory lock per object it carries.
//! - A Begun stage is missing from `ekr stage list` while any other transaction on the server
//!   holds a transaction id, because the listing enumerates stages through the provider's
//!   watermarked change feed, which withholds every event committed after that transaction began.
//!
//! PostgreSQL only: set EKR_TEST_POSTGRES_CONFIG and EKR_TEST_POSTGRES_OWNER to file references;
//! EKR_REQUIRE_POSTGRES=1 makes their absence a failure. The lock case needs a server started with
//! `max_locks_per_transaction=64` and `max_connections=20`; on a server with a larger lock table it
//! is skipped with its reason. Run with `--test-threads=1`. No case reads a clock.
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::{ContentHash, Timestamp};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_kernel::{runtime::PostgresConfiguration, Runtime, SeedDocument};
use ekr_store::{ObjectStore, PostgresStore, StageState, StorageClass};
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
    Some((
        PostgresConfiguration::read(Path::new(&path)).unwrap(),
        owner,
    ))
}

/// A connection as the schema owner, over the owner configuration's verified TLS.
fn owner_connection(
    runtime: &tokio::runtime::Runtime,
    owner: &PostgresConfiguration,
) -> tokio_postgres::Client {
    let dsn = std::fs::read_to_string(&owner.connection_file).unwrap();
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

fn current_thread() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// How many lock-table entries the server has: `max_locks_per_transaction` times the backends
/// and prepared transactions it is sized for (PostgreSQL's `NLOCKENTS`).
fn lock_table(owner: &PostgresConfiguration) -> u64 {
    let io = current_thread();
    let client = owner_connection(&io, owner);
    let setting = |name: &str| -> u64 {
        let row = io
            .block_on(client.query_one(&format!("SHOW {name}"), &[]))
            .unwrap();
        row.get::<_, String>(0).parse().unwrap()
    };
    let backends = setting("max_connections")
        + setting("autovacuum_max_workers")
        + 1
        + setting("max_worker_processes")
        + setting("max_wal_senders");
    setting("max_locks_per_transaction") * (backends + setting("max_prepared_transactions"))
}

/// The current seed with `count` further unreferenced evidence entries, each with its own payload.
fn seed_with_payloads(count: u64) -> SeedDocument {
    let mut document = seed();
    for n in 0..count {
        let bytes = format!("synthetic human statement {n}").into_bytes();
        let hash = ContentHash::of_bytes(&bytes);
        let evidence = Evidence {
            id: current_fixture::id(0x1_0000 + n),
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: hash,
            extracted_by: context().operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        document.graph.evidence.insert(evidence.id, evidence);
        document.evidence_payloads.insert(hash, bytes.into());
    }
    document
}

/// Payloads in one seed: more than the lock table of the fixture server can hold. It is sized
/// for 64 × 42 = 2688 entries (`max_locks_per_transaction=64`, `max_connections=20`), and
/// PostgreSQL grows it into spare shared memory: one transaction there held 4,900 advisory locks
/// and not 5,000, and a seed of 4,096 payloads was staged.
const PAYLOADS: u64 = 8_192;

/// A store whose seed carries more evidence payloads than the server's lock table has entries
/// is staged: `ekr stage begin` copies it and answers a Begun stage. The payloads are retained
/// in the store's tenant before the seed, as a store holds evidence it observed before it seeded,
/// so the seed's own group carries no object stream and lands; the copy carries every one of
/// them into the empty stage tenant in one group.
#[test]
fn a_store_with_more_seed_payloads_than_the_lock_table_is_staged() {
    let Some((config, owner)) = postgres() else {
        return;
    };
    let entries = lock_table(&owner);
    if entries * 2 >= PAYLOADS {
        eprintln!(
            "SKIP the server's lock table is sized for {entries} entries, not fewer than half \
             of {PAYLOADS}"
        );
        return;
    }
    let tenant = format!("stage-locks-{}", ekr_core::TypeId::mint());
    let document = seed_with_payloads(PAYLOADS);
    let raw = PostgresStore::postgres(&config, &tenant, false).unwrap();
    for payload in document.evidence_payloads.values() {
        raw.put(StorageClass::Provenance, payload, SEEDED_AT)
            .unwrap();
    }
    drop(raw);
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    store.seed(document, || SEEDED_AT).unwrap();

    let begun = store.begin_stage();
    let stage = match begun {
        Ok(stage) => stage,
        Err(error) => panic!(
            "a store whose seed carries {PAYLOADS} payloads, on a server with {entries} \
             lock-table entries, was not staged: {error}"
        ),
    };
    assert_eq!(stage.state, StageState::Begun);
    assert_eq!(
        store.abandon_stage(stage.stage_id).unwrap().state,
        StageState::Abandoned
    );
}

/// The stage a begin recorded is listed while another, unrelated transaction on the same server
/// holds a transaction id: `ekr stage list` reads its own writes (`ekr.cli.Stages`).
#[test]
fn a_begun_stage_is_listed_while_another_transaction_is_open() {
    let Some((config, owner)) = postgres() else {
        return;
    };
    let tenant = format!("stage-list-{}", ekr_core::TypeId::mint());
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    store.seed(seed(), || SEEDED_AT).unwrap();

    // Another session's transaction, on no table of this store, holds a transaction id.
    let io = current_thread();
    let other = owner_connection(&io, &owner);
    io.block_on(other.batch_execute("BEGIN; SELECT pg_current_xact_id()"))
        .unwrap();

    let begun = store.begin_stage().expect("the begin lands");
    assert_eq!(begun.state, StageState::Begun);
    let listed = store.stages().unwrap();
    let ids: Vec<_> = listed
        .iter()
        .map(|listing| listing.stage.stage_id)
        .collect();
    io.block_on(other.batch_execute("COMMIT")).unwrap();
    assert_eq!(
        ids,
        vec![begun.stage_id],
        "stage {} was begun and is Begun ({:?} by its own record), and the listing read while \
         another transaction was open is {ids:?}",
        begun.stage_id,
        store
            .stages()
            .unwrap()
            .iter()
            .map(|listing| (listing.stage.stage_id, listing.state))
            .collect::<Vec<_>>()
    );
    store.abandon_stage(begun.stage_id).unwrap();
}

/// The stage a begin recorded is listed when no other transaction is open.
#[test]
fn a_begun_stage_is_listed_with_no_other_transaction_open() {
    let Some((config, _owner)) = postgres() else {
        return;
    };
    let tenant = format!("stage-list-{}", ekr_core::TypeId::mint());
    let store = Runtime::postgres(&config, &tenant, context(), anchor(), false).unwrap();
    store.seed(seed(), || SEEDED_AT).unwrap();
    let begun = store.begin_stage().expect("the begin lands");
    let ids: Vec<_> = store
        .stages()
        .unwrap()
        .iter()
        .map(|listing| listing.stage.stage_id)
        .collect();
    assert_eq!(ids, vec![begun.stage_id]);
    store.abandon_stage(begun.stage_id).unwrap();
}
