//! Adversary, story:store-reading-code-names-no-contents, pass 1: `ekr code-names` through the
//! real binary on stores and file arguments the unit's own tests do not build.
//!
//! Each store is `tests/fixtures/code-names/store/seed.yaml` with one edit, so the rest of the
//! fixture (types Volume and Folio, nodes Codex Aurum, Ledger of Tides, Folio Alpha, Folio Beta,
//! Folio Gamma) holds as the unit's tests describe it.

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/code-names")
    .join(name)
}

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    /// The fixture seed with `edit` applied, seeded on `backend`.
    fn seeded(backend: &'static str, edit: impl FnOnce(String) -> String) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let seed = world.directory.path().join("seed.yaml");
        let text = std::fs::read_to_string(fixture("store/seed.yaml")).unwrap();
        std::fs::write(&seed, edit(text)).unwrap();
        let output = world.run(&["seed", &seed.display().to_string()]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn run(&self, verb: &[&str]) -> Output {
        let mut args = vec![
            "--host".to_owned(),
            fixture("store/host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        ekr().args(args).stdin(Stdio::null()).output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn source(&self, name: &str, text: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.display().to_string()
    }
}

const OLD_ID_ALIAS: &str = "        - 00000000-0000-4000-8000-00000000b301\n";

/// views.yaml: a store id is exempt as "the text, in any case, of an id the revision holds".
/// The unit's fixture holds only a lowercase id alias, so dropping the case fold in `exempt`
/// (`ids.contains(&name.to_lowercase())` -> `ids.contains(name)`) leaves its suite green; this
/// case holds the fold.
#[test]
fn an_alias_that_is_a_store_id_in_upper_case_is_exempt_not_found() {
    let upper = "00000000-0000-4000-8000-00000000B301";
    let world = World::seeded("file", |seed| {
        assert!(seed.contains(OLD_ID_ALIAS));
        seed.replacen(OLD_ID_ALIAS, &format!("        - {upper}\n"), 1)
    });
    let path = world.source("reader.ts", &format!("const legacy = \"{upper}\";\n"));
    let found = world.ok(&["code-names", &path]);
    assert_eq!(found["meta"]["literals"], 1, "{found}");
    assert_eq!(found["meta"]["exempt"], 1, "{found}");
    assert_eq!(found["meta"]["findings"], 0, "{found}");
}

/// views.yaml: "an empty or blank name equals nothing". Held through a store that carries a blank
/// alias, which the unit's fixtures never do.
#[test]
fn a_blank_alias_equals_no_blank_literal() {
    let world = World::seeded("file", |seed| {
        assert!(seed.contains("        aliases: []\n"));
        seed.replacen(
            "        aliases: []\n",
            "        aliases:\n        - \"  \"\n",
            1,
        )
    });
    let path = world.source("reader.ts", "const pad = \"  \";\n");
    let found = world.ok(&["code-names", &path]);
    assert_eq!(found["meta"]["literals"], 1, "{found}");
    assert_eq!(found["meta"]["exempt"], 0, "{found}");
    assert_eq!(found["meta"]["findings"], 0, "{found}");
}

/// A directory is not a source: a fault naming it, nothing on stdout.
#[test]
fn a_directory_argument_is_a_fault_naming_it() {
    let world = World::seeded("file", |seed| seed);
    let directory = world.directory.path().join("src");
    std::fs::create_dir(&directory).unwrap();
    let path = directory.display().to_string();
    let output = world.run(&["code-names", &path]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains(&path));
}

/// A symlink is read through, and reported under the path as given, not its target.
#[test]
fn a_symlinked_source_is_reported_under_the_link_path() {
    let world = World::seeded("file", |seed| seed);
    let target = world.source("real.ts", "const kind = \"Volume\";\n");
    let link = world.directory.path().join("link.ts");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let link = link.display().to_string();
    let found = world.ok(&["code-names", &link]);
    assert_eq!(found["meta"]["findings"], 1, "{found}");
    assert_eq!(found["findings"][0]["file"], link.as_str());
}

fn make_read_only(at: &Path, mode: u32) {
    for entry in walk(at) {
        let file_mode = if entry.is_dir() { mode } else { mode & 0o444 };
        std::fs::set_permissions(&entry, std::fs::Permissions::from_mode(file_mode)).unwrap();
    }
    std::fs::set_permissions(at, std::fs::Permissions::from_mode(mode)).unwrap();
}

fn walk(at: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(at).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            found.extend(walk(&path));
        }
        found.push(path);
    }
    found
}

/// The verb writes nothing, so it needs no write permission: on a store directory nobody may
/// write, it answers as it does on a writable one, on both providers
/// (`task:read-verbs-open-a-read-only-store`; `tests/read_only_store.rs` holds the other read
/// verbs and the write refusal).
#[test]
fn ekr_code_names_answers_on_a_read_only_store_on_both_providers() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend, |seed| seed);
        let path = world.source("reader.ts", "const kind = \"Volume\";\n");
        let writable = world.ok(&["code-names", &path]);
        make_read_only(world.directory.path(), 0o555);
        let output = world.run(&["code-names", &path]);
        let status = output.status.code();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        make_read_only(world.directory.path(), 0o755);
        assert_eq!(status, Some(0), "{backend}: {stderr}");
        let read_only: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(read_only, writable, "{backend}");
    }
}

/// A store with thousands of aliases and a tree of thousands of files: the verb stays within a
/// few seconds, and every planted alias is found once per file.
#[test]
fn five_thousand_aliases_and_two_thousand_files_answer_in_bounded_time() {
    let aliases: String = (0..5_000)
        .map(|n| format!("        - Register {n:05}\n"))
        .collect();
    let world = World::seeded("file", |seed| {
        assert!(seed.contains("        - Tidal Register\n"));
        seed.replacen(
            "        - Tidal Register\n",
            &format!("        - Tidal Register\n{aliases}"),
            1,
        )
    });
    let tree = world.directory.path().join("tree");
    std::fs::create_dir(&tree).unwrap();
    let body: String = (0..200)
        .map(|n| {
            if n % 10 == 0 {
                format!("const v{n} = read(node, \"field_{n}\", 'Register {n:05}');\n")
            } else {
                format!("const v{n} = read(node, \"field_{n}\");\n")
            }
        })
        .collect();
    let files: Vec<String> = (0..2_000)
        .map(|n| {
            let path = tree.join(format!("m{n:04}.ts"));
            std::fs::write(&path, &body).unwrap();
            path.display().to_string()
        })
        .collect();
    let mut verb = vec!["code-names"];
    verb.extend(files.iter().map(String::as_str));
    let started = Instant::now();
    let found = world.ok(&verb);
    let elapsed = started.elapsed();
    assert_eq!(found["meta"]["files"], 2_000);
    assert_eq!(found["meta"]["literals"], 440_000);
    assert_eq!(found["meta"]["findings"], 40_000);
    assert!(
        elapsed < Duration::from_secs(60),
        "5 000 aliases over 2 000 files took {elapsed:?}"
    );
}
