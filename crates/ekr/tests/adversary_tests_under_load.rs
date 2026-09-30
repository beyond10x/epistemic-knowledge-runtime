//! Adversary pass 1 on wave ops-05 unit T: `task:two-tests-pass-under-load`.
//!
//! `adversary_sdk01_h_replaced_store.rs` now skips its "deleted and created again" variant when no
//! directory created after the store's deletion is handed the inode the store freed, and says the
//! cause is another process taking it. Nothing bounds how often that happens: where the filesystem
//! never hands a freed inode back at all (tmpfs, btrfs), the variant skips on every run, and the
//! file stays green without ever asserting the reopen it was written for.

use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

/// Attempts at staging a freed inode; the variant under test makes one per run.
const ATTEMPTS: usize = 10;

fn inode(path: &Path) -> (u64, u64) {
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

/// The staging `recreate_on_the_freed_inode` does, on a small store-shaped directory in a fresh
/// temporary directory: delete it, then create up to 20 000 directories beside it looking for the
/// inode it freed. Whether one was handed that inode.
fn freed_inode_comes_back() -> bool {
    let world = tempfile::tempdir().unwrap();
    let store = world.path().join("store");
    std::fs::create_dir_all(store.join("log")).unwrap();
    std::fs::write(store.join("log/0.yaml"), "x").unwrap();
    let freed = inode(&store);
    std::fs::remove_dir_all(&store).unwrap();
    let mut spare = Vec::new();
    let mut found = false;
    for n in 0..20_000 {
        let candidate = world.path().join(format!(".inode-{n}"));
        std::fs::create_dir(&candidate).unwrap();
        if inode(&candidate) == freed {
            found = true;
            break;
        }
        spare.push(candidate);
    }
    for candidate in spare {
        std::fs::remove_dir(candidate).unwrap();
    }
    found
}

/// The skip is bounded: of `ATTEMPTS` stagings on the filesystem the suite's temporary
/// directories live on, at least one is handed the freed inode, so the "deleted and created
/// again" variant asserts its reopen on some run rather than on none.
#[test]
fn the_replaced_store_variant_that_can_skip_is_staged_at_least_once_in_ten() {
    let staged = (0..ATTEMPTS).filter(|_| freed_inode_comes_back()).count();
    eprintln!(
        "freed inode handed back in {staged} of {ATTEMPTS} stagings under {}",
        std::env::temp_dir().display()
    );
    assert!(
        staged > 0,
        "no staging of {ATTEMPTS} was handed the freed inode under {}: the \"deleted and created \
         again\" variant of adversary_h_mcp_answers_from_a_file_store_replaced_under_the_same_\
         inode_without_a_restart skips on every run here and asserts nothing, while the file \
         passes",
        std::env::temp_dir().display()
    );
}
