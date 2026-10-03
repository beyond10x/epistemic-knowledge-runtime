//! The CLI's independently provisioned reviewer binding. Request flags and environment
//! variables cannot choose this path or its trusted owner.
use ekr_core::contract_data::EkrKernelTrustedReviewHostBinding;
use ekr_core::contracts::kernel as m;
#[cfg(any(unix, test))]
use ekr_core::ContentHash;
use ekr_kernel::{human_review, PersistenceError};

fn fault(reason: impl std::fmt::Display) -> PersistenceError {
    PersistenceError::Document(format!("review-host-provisioning: {reason}"))
}

fn decode(tenant: &str, bytes: &[u8]) -> Result<m::TrustedReviewHostBinding, PersistenceError> {
    let wire: EkrKernelTrustedReviewHostBinding = serde_json::from_slice(bytes).map_err(fault)?;
    if wire.audience.tenant != tenant {
        return Err(fault("binding tenant differs from the selected store"));
    }
    let binding = m::TrustedReviewHostBinding {
        audience: m::HumanDecisionAudience {
            tenant: wire.audience.tenant.clone(),
            seed_anchor: m::ContentHash(wire.audience.seed_anchor.0.clone()),
        },
        reviewer_policy_digest: m::ContentHash(wire.reviewer_policy_digest.0.clone()),
    };
    human_review::host_binding_bytes(&binding).map_err(|e| fault(e.code))?;
    Ok(binding)
}

pub(super) fn binding(
    tenant: &str,
) -> Result<Option<m::TrustedReviewHostBinding>, PersistenceError> {
    system_bytes(tenant)?
        .map(|bytes| decode(tenant, &bytes))
        .transpose()
}

#[cfg(unix)]
fn system_bytes(tenant: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
    // macOS's /etc is a symlink. Use its fixed system location, without accepting symlinks
    // supplied by a requester or relaxing the ownership check for any ancestor.
    let etc = if cfg!(target_os = "macos") {
        "private/etc"
    } else {
        "etc"
    };
    let relative = std::path::PathBuf::from(etc)
        .join("ekr/review")
        .join(format!("{}.json", ContentHash::of_bytes(tenant.as_bytes())));
    read_chain(std::path::Path::new("/"), &relative, 0)
}

#[cfg(not(unix))]
fn system_bytes(_: &str) -> Result<Option<Vec<u8>>, PersistenceError> {
    // Legacy stores remain usable. Upgrades require an independently trusted embedding on
    // platforms where this CLI has no implemented system-provisioning boundary.
    Ok(None)
}

#[cfg(unix)]
fn check_metadata(
    metadata: &std::fs::Metadata,
    owner: u32,
    directory: bool,
) -> Result<(), PersistenceError> {
    use std::os::unix::fs::MetadataExt;
    if metadata.uid() != owner || metadata.mode() & 0o022 != 0 {
        return Err(fault("binding and ancestors must be owned by the trusted system user and not writable by group or others"));
    }
    if (directory && !metadata.is_dir()) || (!directory && !metadata.is_file()) {
        return Err(fault(
            "binding path must contain only directories and a regular file, without symlinks",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn read_chain(
    root: &std::path::Path,
    relative: &std::path::Path,
    owner: u32,
) -> Result<Option<Vec<u8>>, PersistenceError> {
    use std::io::Read;
    use std::os::unix::fs::MetadataExt;
    use std::path::Component;
    const LIMIT: u64 = 64 * 1024;
    check_metadata(
        &std::fs::symlink_metadata(root).map_err(fault)?,
        owner,
        true,
    )?;
    let mut path = root.to_path_buf();
    let mut components = relative.components().peekable();
    while let Some(component) = components.next() {
        let Component::Normal(name) = component else {
            return Err(fault("binding path must stay beneath the system root"));
        };
        path.push(name);
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(fault(e)),
        };
        let directory = components.peek().is_some();
        check_metadata(&metadata, owner, directory)?;
        if directory {
            continue;
        }
        let mut file = std::fs::File::open(&path).map_err(fault)?;
        let opened = file.metadata().map_err(fault)?;
        check_metadata(&opened, owner, false)?;
        if opened.dev() != metadata.dev() || opened.ino() != metadata.ino() {
            return Err(fault("binding was replaced while opening"));
        }
        if opened.len() > LIMIT {
            return Err(fault("binding exceeds 64 KiB"));
        }
        let mut bytes = Vec::new();
        (&mut file)
            .take(LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(fault)?;
        if bytes.len() as u64 > LIMIT {
            return Err(fault("binding exceeds 64 KiB"));
        }
        return Ok(Some(bytes));
    }
    Err(fault("binding path has no file"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_binding_refuses_wrong_tenant_unknown_fields_and_invalid_hashes() {
        let mut document = serde_json::json!({
            "audience": {"tenant": "fixture", "seed_anchor": ContentHash::of_bytes(b"seed").to_string()},
            "reviewer_policy_digest": ContentHash::of_bytes(b"policy").to_string()
        });
        let encoded = serde_json::to_vec(&document).unwrap();
        let binding = decode("fixture", &encoded).unwrap();
        assert_eq!(binding.audience.tenant, "fixture");
        assert!(decode("other", &encoded).is_err());
        document["self_approved"] = true.into();
        assert!(decode("fixture", &serde_json::to_vec(&document).unwrap()).is_err());
        document.as_object_mut().unwrap().remove("self_approved");
        document["reviewer_policy_digest"] = "not-a-digest".into();
        assert!(decode("fixture", &serde_json::to_vec(&document).unwrap()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn protected_chain_checks_every_owner_mode_kind_and_size_before_reading() {
        use std::os::unix::fs::{symlink, MetadataExt, PermissionsExt};
        use std::path::Path;
        let root = tempfile::tempdir().unwrap();
        let owner = std::fs::metadata(root.path()).unwrap().uid();
        let directory = root.path().join("review");
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let file = directory.join("fixture.json");
        std::fs::write(&file, b"fixture").unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        let relative = Path::new("review/fixture.json");
        assert_eq!(
            read_chain(root.path(), relative, owner).unwrap(),
            Some(b"fixture".to_vec())
        );
        assert!(read_chain(root.path(), relative, owner.wrapping_add(1)).is_err());
        assert_eq!(
            read_chain(root.path(), Path::new("missing.json"), owner).unwrap(),
            None
        );
        for mode in [0o620, 0o602] {
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(mode)).unwrap();
            assert!(read_chain(root.path(), relative, owner).is_err());
        }
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o720)).unwrap();
        assert!(read_chain(root.path(), relative, owner).is_err());
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        symlink(&file, root.path().join("link.json")).unwrap();
        assert!(read_chain(root.path(), Path::new("link.json"), owner).is_err());
        symlink(&directory, root.path().join("linked")).unwrap();
        assert!(read_chain(root.path(), Path::new("linked/fixture.json"), owner).is_err());
        assert!(read_chain(root.path(), Path::new("review"), owner).is_err());
        assert!(read_chain(root.path(), Path::new("../fixture.json"), owner).is_err());
        std::fs::write(&file, vec![b'x'; 64 * 1024 + 1]).unwrap();
        assert!(read_chain(root.path(), relative, owner).is_err());
    }
}
