//! A stage's tenant (design § 107.1, `systems/ekr/domains/store.yaml`, `ekr.store.StageId`): the
//! Eventlog tenant a stage of a store is written in, derived from the store's own tenant and the
//! stage id, and carrying a marker no store's tenant may carry.
//!
//! The derivation is deterministic, gives different names for different stage ids and for
//! different store tenants, and is a valid Eventlog tenant for every store tenant: 111 ASCII
//! graphic bytes. Because every derived name carries [`STAGE_TENANT_MARKER`] and a store is never
//! opened under a tenant that carries it ([`admit_store_tenant`], which the kernel's openers apply
//! to every host configuration's tenant), the derivation can produce neither the store's own
//! tenant nor another store's.
use ekr_core::ContentHash;
use eventlog_core::TenantId;

use crate::StoreError;

/// The marker every stage tenant carries and no store's tenant may carry anywhere.
pub const STAGE_TENANT_MARKER: &str = "ekr.stage:";

/// The tenant of stage `stage` of the store whose tenant is `store`:
/// `ekr.stage:<store>:<stage>`, where `<store>` is the 64 lowercase hex characters of the store
/// tenant's payload address ([`ContentHash::of_bytes`]) and `<stage>` the stage id in its
/// lowercase hyphenated UUID form.
///
/// `stage` is the stage id's 128-bit value (`ekr.store.StageId`, a UUID newtype, which begin
/// mints); the caller passes the id begin minted and never one of its own choosing.
///
/// # Errors
/// [`StoreError::StageTenantReserved`] for a `store` tenant that carries the marker, which is no
/// store's tenant, and the provider's refusal of a `store` tenant Eventlog does not admit.
pub fn stage_tenant(store: &str, stage: u128) -> Result<String, StoreError> {
    admit_store_tenant(store)?;
    TenantId::new(store)?;
    let name = format!(
        "{STAGE_TENANT_MARKER}{}:{}",
        ContentHash::of_bytes(store.as_bytes()).to_hex(),
        hyphenated(stage)
    );
    TenantId::new(name.as_str())?;
    Ok(name)
}

/// Admits `tenant` as a store's own tenant: refuses one that carries [`STAGE_TENANT_MARKER`]
/// anywhere, before anything is read, created or written.
///
/// # Errors
/// [`StoreError::StageTenantReserved`], `stage-tenant-reserved` (`ekr.store.StageTenantReserved`).
pub fn admit_store_tenant(tenant: &str) -> Result<(), StoreError> {
    if tenant.contains(STAGE_TENANT_MARKER) {
        Err(StoreError::StageTenantReserved(tenant.to_owned()))
    } else {
        Ok(())
    }
}

/// `id` as a UUID's lowercase hyphenated text, 8-4-4-4-12, as `ekr-core`'s ids display.
fn hyphenated(id: u128) -> String {
    let hex = format!("{id:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
