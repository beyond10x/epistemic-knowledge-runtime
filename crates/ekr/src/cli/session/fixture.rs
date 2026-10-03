//! Stores seeded in-process from the example host and seed, for the long-running readers' unit
//! tests, and the rename a host replaces a store with.

use std::path::{Path, PathBuf};

use ekr_core::Timestamp;

use super::super::{Backend, Configured, Store};

/// Both providers.
pub(in crate::cli) const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];

/// What `ekr <argv>` prints, which must succeed.
fn run(argv: &[&str]) -> String {
    let clock = || Timestamp::from_millis(0);
    super::super::run(argv.iter().copied(), &clock, &mut std::io::empty())
        .unwrap_or_else(|failure| panic!("{argv:?}: {failure}"))
}

fn text(path: &Path) -> &str {
    path.to_str().expect("a temporary path is UTF-8")
}

/// The example host at `directory/host.json`, and at `directory/<name>` a `backend` store seeded
/// from the example seed: its configuration, resolved as a store verb resolves it.
pub(in crate::cli) fn seeded(directory: &Path, backend: Backend, name: &str) -> Store {
    let host = directory.join("host.json");
    std::fs::write(&host, run(&["ekr", "example", "ekr.cli-host/1"])).expect("writing the host");
    let seed = directory.join("seed.yaml");
    std::fs::write(&seed, run(&["ekr", "example", "ekr-seed/2"])).expect("writing the seed");
    let store = directory.join(name);
    let backend_name = match backend {
        Backend::File => "file",
        Backend::Sqlite => "sqlite",
        Backend::Postgres => panic!("local fixture requires a filesystem backend"),
    };
    run(&[
        "ekr",
        "--host",
        text(&host),
        "--store",
        text(&store),
        "--backend",
        backend_name,
        "seed",
        text(&seed),
    ]);
    Configured {
        host: Some(host),
        store: Some(store),
        backend: Some(backend),
        full_replay: false,
        access: crate::cli::Access::Read,
    }
    .resolve("test")
    .expect("the configuration resolves")
}

/// [`seeded`], then one committed transaction creating an organization: head 1.
pub(in crate::cli) fn seeded_with_a_commit(
    directory: &Path,
    backend: Backend,
    name: &str,
) -> Store {
    let store = seeded(directory, backend, name);
    let transaction = "00000000-0000-4000-8000-00000000f902";
    let document = directory.join(format!("{name}-create.yaml"));
    std::fs::write(
        &document,
        format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: \
             00000000-0000-4000-8000-000000000101\n  operations:\n  - !CreateNode\n    id: \
             00000000-0000-4000-8000-00000000f901\n    root_id: \
             00000000-0000-4000-8000-000000000002\n    type_id: \
             00000000-0000-4000-8000-000000000202\n    canonical_name: Initech\n    properties: \
             {{}}\n    aliases: [Initech]\n  evidence: []\n"
        ),
    )
    .expect("writing the transaction");
    let backend_name = match backend {
        Backend::File => "file",
        Backend::Sqlite => "sqlite",
        Backend::Postgres => panic!("local fixture requires a filesystem backend"),
    };
    let host = directory.join("host.json");
    for verb in [
        vec!["propose", text(&document)],
        vec!["validate", transaction],
        vec!["commit", transaction],
    ] {
        let mut argv = vec![
            "ekr",
            "--host",
            text(&host),
            "--store",
            text(&store.store),
            "--backend",
            backend_name,
        ];
        argv.extend(verb);
        run(&argv);
    }
    store
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_owned();
    text.push(suffix);
    PathBuf::from(text)
}

/// Replaces every entry of the file store directory `store` with `from`'s, as
/// `rsync -a --delete from/ store/` does: the root keeps its device and inode.
pub(in crate::cli) fn replace_inside(store: &Path, from: &Path) {
    for entry in std::fs::read_dir(store).expect("the store directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).expect("removing a directory");
        } else {
            std::fs::remove_file(&path).expect("removing a file");
        }
    }
    copy(from, store);
}

fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).expect("a directory");
    for entry in std::fs::read_dir(from).expect("the replacement") {
        let path = entry.expect("an entry").path();
        let target = to.join(path.file_name().expect("a name"));
        if path.is_dir() {
            copy(&path, &target);
        } else {
            std::fs::copy(&path, &target).expect("copying a file");
        }
    }
}

/// Moves what is at `store` to `aside` and `from` into its place, each by rename, a SQLite
/// database with its `-wal` and `-shm` files.
pub(in crate::cli) fn replace(store: &Path, aside: &Path, from: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(store, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(aside, suffix)).expect("moving the store aside");
        }
    }
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(from, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(store, suffix)).expect("moving the replacement in");
        }
    }
}
