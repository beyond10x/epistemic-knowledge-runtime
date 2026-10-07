//! A run staged and published whole (design § 107, `story:a-run-is-staged-and-published-whole`).
//!
//! Every case runs on File and SQLite, and on PostgreSQL where the PostgreSQL tests run: set
//! EKR_TEST_POSTGRES_CONFIG and EKR_TEST_POSTGRES_OWNER to file references;
//! EKR_REQUIRE_POSTGRES=1 makes their absence a failure.
#[allow(dead_code)]
mod current_fixture;

use current_fixture::{anchor, context, seed, SEEDED_AT};
use ekr_core::TypeId;
use ekr_kernel::{runtime::PostgresConfiguration, Commit, PersistenceError, Runtime};
use ekr_store::{SqliteStore, STAGE_TENANT_MARKER};
use std::collections::BTreeSet;

/// The hosted PostgreSQL configuration, with its owner schema applied once, or `None` where the
/// PostgreSQL tests do not run.
fn postgres() -> Option<PostgresConfiguration> {
    let path = match std::env::var_os("EKR_TEST_POSTGRES_CONFIG") {
        Some(path) => path,
        None => {
            assert_ne!(
                std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
                Ok("1"),
                "required real PostgreSQL fixture is missing"
            );
            eprintln!("SKIP real PostgreSQL: EKR_TEST_POSTGRES_CONFIG is unset");
            return None;
        }
    };
    static SCHEMA: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        let owner =
            std::env::var_os("EKR_TEST_POSTGRES_OWNER").expect("owner configuration reference");
        Runtime::postgres_schema(
            &PostgresConfiguration::read(std::path::Path::new(&owner)).unwrap(),
        )
        .unwrap();
    });
    Some(PostgresConfiguration::read(std::path::Path::new(&path)).unwrap())
}

/// `opened` was refused as `stage-tenant-reserved`, and is reported as that refusal.
fn refused(what: &str, opened: Result<Runtime, PersistenceError>) {
    let error = opened.err().unwrap_or_else(|| panic!("{what} opened"));
    assert!(
        matches!(error, PersistenceError::StageTenantReserved(_)),
        "{what}: {error}"
    );
    assert!(
        error.to_string().starts_with("stage-tenant-reserved: "),
        "{what}: {error}"
    );
}

/// Design § 107.10, unit C: the derivation never yields a store's tenant — every name it gives
/// carries the stage marker and fits an Eventlog tenant for every store tenant, a 512-byte one
/// included — and a host configuration whose tenant carries the marker opens no store, on any
/// provider, and writes nothing. Until `ekr.store.StageId` has its carrier (unit P), a stage id
/// is a minted id's value.
#[test]
fn a_stage_tenant_is_never_a_store_tenant() {
    let minted = TypeId::mint();
    let id = minted.as_u128();
    let other = TypeId::mint().as_u128();
    let longest = "t".repeat(512);
    let stores = [
        "hosted-store",
        "a",
        "a tenant with spaces",
        "colons:and/slashes",
        "~",
        longest.as_str(),
    ];
    let mut derived = BTreeSet::new();
    for store in stores {
        let staged = ekr_store::stage_tenant(store, id).unwrap();
        assert_ne!(staged, store);
        assert!(staged.starts_with(STAGE_TENANT_MARKER), "{staged}");
        assert!(
            staged.ends_with(&format!(":{minted}")),
            "the name reads the stage id: {staged}"
        );
        assert_eq!(
            staged.len(),
            STAGE_TENANT_MARKER.len() + 64 + 1 + 36,
            "a fixed length whatever the store tenant's: {staged}"
        );
        assert!(
            staged.len() <= 512 && staged.bytes().all(|b| b.is_ascii_graphic() || b == b' '),
            "an Eventlog tenant: {staged}"
        );
        assert_eq!(staged, ekr_store::stage_tenant(store, id).unwrap());
        assert_ne!(staged, ekr_store::stage_tenant(store, other).unwrap());
        assert!(derived.insert(staged), "another store tenant, another name");
    }
    // A store tenant carrying the marker has no stage.
    assert!(ekr_store::stage_tenant(&format!("x{STAGE_TENANT_MARKER}y"), id).is_err());

    // A host configuration whose tenant carries the marker opens no store, on any provider, and
    // writes nothing: so no store's tenant is ever a stage's.
    let directory = tempfile::tempdir().unwrap();
    let file_store = directory.path().join("file-store");
    let sqlite_store = directory.path().join("store.db");
    Runtime::file(&file_store, "hosted-store", context(), anchor())
        .unwrap()
        .seed(seed(), || SEEDED_AT)
        .unwrap();
    Runtime::sqlite(&sqlite_store, "hosted-store", context(), anchor())
        .unwrap()
        .seed(seed(), || SEEDED_AT)
        .unwrap();
    let staged = derived.first().unwrap().clone();
    let reserved = [
        staged.clone(),
        STAGE_TENANT_MARKER.to_owned(),
        format!("team {STAGE_TENANT_MARKER} tenant"),
    ];
    for tenant in &reserved {
        let new_file = directory.path().join("new-file-store");
        let new_sqlite = directory.path().join("new.db");
        refused(
            "file",
            Runtime::file(&new_file, tenant, context(), anchor()),
        );
        refused(
            "sqlite",
            Runtime::sqlite(&new_sqlite, tenant, context(), anchor()),
        );
        assert!(
            !new_file.exists() && !new_sqlite.exists(),
            "nothing created"
        );
        refused(
            "file_existing",
            Runtime::file_existing(&file_store, tenant, context(), anchor()),
        );
        refused(
            "file_reading",
            Runtime::file_reading(&file_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_existing",
            Runtime::sqlite_existing(&sqlite_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_reading",
            Runtime::sqlite_reading(&sqlite_store, tenant, context(), anchor()),
        );
        refused(
            "sqlite_snapshot",
            Runtime::sqlite_snapshot(&sqlite_store, tenant, context(), anchor()),
        );
    }
    // The stage's own tenant is an Eventlog tenant the store level opens: the derivation, not a
    // host configuration, is how a stage's tenant is reached.
    Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(SqliteStore::sqlite(&sqlite_store, &staged, None)?.under(authority))
    })
    .unwrap();

    let Some(config) = postgres() else { return };
    let derived = ekr_store::stage_tenant(&format!("hosted-{}", TypeId::mint()), id).unwrap();
    for tenant in [derived, format!("team {STAGE_TENANT_MARKER} tenant")] {
        for reading in [false, true] {
            refused(
                "postgres",
                Runtime::postgres(&config, &tenant, context(), anchor(), reading),
            );
        }
    }
}
