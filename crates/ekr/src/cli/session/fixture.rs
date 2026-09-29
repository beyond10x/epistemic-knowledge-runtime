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
    }
    .resolve("test")
    .expect("the configuration resolves")
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_owned();
    text.push(suffix);
    PathBuf::from(text)
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
