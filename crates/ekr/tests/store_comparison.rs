//! `story:store-equivalence-check`: `ekr compare` reads two stores, on any providers, and prints
//! the `ekr.store-comparison/1` document (`ekr.cli.CompareStores`, systems/ekr/domains/cli.yaml):
//! equal or not, and per category the counts on each side and the first differing ids, each with
//! the fields that differ. Commit times and transaction ids are not compared.
//!
//! Every store here is built from the example seed with one more Organization (`Initech`), then the
//! example transaction, proposed, validated and committed. Each is built through the `ekr::cli::run`
//! host seam with its own clock, so two stores built from the same documents carry different commit
//! times — their heads' roots differ, and the first case checks that they do — while the
//! comparison itself always runs as a fresh `ekr` process.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ekr_core::Timestamp;
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const BOB: &str = "00000000-0000-4000-8000-000000000302";
const INITECH: &str = "00000000-0000-4000-8000-000000000304";
const CAROL: &str = "00000000-0000-4000-8000-000000000305";
const BOB_CEO: &str = "00000000-0000-4000-8000-000000000511";
const BOB_EVIDENCE: &str = "00000000-0000-4000-8000-000000000402";
const BOB_PAYLOAD_HASH: &str = "bd1d4dc9ba5012df5e43505418aa7728c15bca052a1ad43b1a2d00d04b75bdd6";
/// The six categories, in the order the document lists them.
const CATEGORIES: [&str; 6] = [
    "types",
    "properties",
    "nodes",
    "edges",
    "assertions",
    "evidence",
];

/// One verb through the host seam, at the fixed instant `at`; its stdout.
fn seam(args: &[String], at: i64) -> String {
    let argv = std::iter::once("ekr".to_owned()).chain(args.iter().cloned());
    ekr::cli::run(argv, &|| Timestamp::from_millis(at), &mut std::io::empty())
        .unwrap_or_else(|failure| panic!("{args:?}: {failure}"))
}

/// Static text a verb that opens no store prints.
fn text(args: &[&str]) -> String {
    seam(&args.iter().map(|a| (*a).to_owned()).collect::<Vec<_>>(), 0)
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in [
        "EKR_HOST",
        "EKR_STORE",
        "EKR_BACKEND",
        "EKR_FULL_REPLAY",
        "EKR_STAGE",
    ] {
        command.env_remove(var);
    }
    command
}

/// What sets a variant store apart from the base one.
#[derive(Clone, Copy, Debug)]
enum Variant {
    Base,
    /// Assertion 0511 (Bob is CEO of Acme) names Initech as its object instead.
    Object,
    /// Bob carries the alias `Robert`.
    Alias,
    /// Evidence 0402's payload, and so its payload hash, says 2026-03-13 instead of 2026-03-12.
    PayloadHash,
    /// One more Person, Carol.
    ExtraNode,
}

/// `text` with the first `from` after the first `anchor` replaced by `to`.
fn replace_after(text: &str, anchor: &str, from: &str, to: &str) -> String {
    let start = text.find(anchor).unwrap_or_else(|| panic!("no {anchor:?}"));
    let at = start
        + text[start..]
            .find(from)
            .unwrap_or_else(|| panic!("no {from:?}"));
    format!("{}{to}{}", &text[..at], &text[at + from.len()..])
}

fn node(id: &str, type_id: &str, name: &str) -> String {
    format!(
        "      {id}:\n        id: {id}\n        root_id: 00000000-0000-4000-8000-000000000002\n        \
         type_id: {type_id}\n        canonical_name: {name}\n        aliases: []\n        \
         type_state: null\n        properties: {{}}\n"
    )
}

/// The seed document of `variant`.
fn seed(variant: Variant, directory: &Path) -> String {
    let example = text(&["example", "ekr-seed/2"]);
    let acme = "canonical_name: Acme\n        aliases: []\n        type_state: null\n        \
                properties: {}\n";
    let initech = node(INITECH, "00000000-0000-4000-8000-000000000202", "Initech");
    let mut seed = example.replacen(acme, &format!("{acme}{initech}"), 1);
    assert_ne!(seed, example, "the example seed's Acme node moved");
    match variant {
        Variant::Base => {}
        Variant::Object => {
            seed = replace_after(
                &seed,
                &format!("{BOB_CEO}:"),
                "object: !Node 00000000-0000-4000-8000-000000000303",
                &format!("object: !Node {INITECH}"),
            );
        }
        Variant::Alias => {
            seed = replace_after(
                &seed,
                "canonical_name: Bob",
                "aliases: []",
                "aliases: [Robert]",
            );
        }
        Variant::PayloadHash => {
            let payload = directory.join("bob.txt");
            std::fs::write(&payload, "Bob became CEO of Acme on 2026-03-13.").unwrap();
            let hashed: Value =
                serde_json::from_str(&text(&["hash", payload.to_str().unwrap()])).unwrap();
            let hash = hashed["content_hash"].as_str().unwrap();
            assert_ne!(hash, BOB_PAYLOAD_HASH);
            seed = seed
                .replace(BOB_PAYLOAD_HASH, hash)
                .replace(", 45, 49, 50, 46]", ", 45, 49, 51, 46]");
        }
        Variant::ExtraNode => {
            let carol = node(CAROL, "00000000-0000-4000-8000-000000000201", "Carol");
            seed = seed.replacen(&initech, &format!("{initech}{carol}"), 1);
        }
    }
    seed
}

/// One store: its own directory, provider and host.
struct Built {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl Built {
    /// Seeds `variant` and commits the example transaction, every step at an instant after
    /// `clock`.
    fn new(backend: &'static str, variant: Variant, clock: i64) -> Self {
        let built = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        std::fs::write(built.host(), text(&["example", "ekr.cli-host/1"])).unwrap();
        let seed_path = built.directory.path().join("seed.yaml");
        std::fs::write(&seed_path, seed(variant, built.directory.path())).unwrap();
        let transaction = built.directory.path().join("transaction.yaml");
        std::fs::write(
            &transaction,
            text(&["example", "ekr.transaction-document/2"]),
        )
        .unwrap();
        let seeded: Value =
            serde_json::from_str(&built.at(clock, &["seed", seed_path.to_str().unwrap()])).unwrap();
        assert_eq!(seeded["result"]["revision"], 0, "{variant:?}: {seeded}");
        let proposed: Value =
            serde_json::from_str(&built.at(clock + 1, &["propose", transaction.to_str().unwrap()]))
                .unwrap();
        let id = proposed["result"]["transaction_id"]
            .as_str()
            .or_else(|| proposed["transaction_id"].as_str())
            .unwrap_or_else(|| panic!("no transaction id in {proposed}"))
            .to_owned();
        built.at(clock + 2, &["validate", &id]);
        built.at(clock + 3, &["commit", &id]);
        assert_eq!(built.head()["revision"], 1);
        built
    }

    fn host(&self) -> PathBuf {
        self.directory.path().join("host.json")
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn configuration(&self) -> Vec<String> {
        vec![
            "--host".to_owned(),
            self.host().display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ]
    }

    /// One store verb through the host seam at instant `at`.
    fn at(&self, at: i64, verb: &[&str]) -> String {
        let mut args = self.configuration();
        args.extend(verb.iter().map(|a| (*a).to_owned()));
        seam(&args, at)
    }

    /// `ekr head`, from a fresh process.
    fn head(&self) -> Value {
        let output = ekr()
            .args(self.configuration())
            .arg("head")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{output:?}");
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `ekr compare` of this store (left) with `other` (right), from a fresh process.
    fn compare(&self, other: &Self, extra: &[&str]) -> Value {
        let output = ekr()
            .args(self.configuration())
            .arg("compare")
            .arg("--other")
            .arg(other.store())
            .args(["--other-backend", other.backend])
            .arg("--other-host")
            .arg(other.host())
            .args(extra)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} vs {}: {}",
            self.backend,
            other.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Every file under this store's directory, by path, with its exact bytes.
    fn bytes(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(path: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in std::fs::read_dir(path).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    walk(&path, into);
                } else {
                    into.insert(path.clone(), std::fs::read(&path).unwrap());
                }
            }
        }
        let mut files = BTreeMap::new();
        walk(self.directory.path(), &mut files);
        files
    }
}

/// The category `name` of a comparison.
fn category<'a>(comparison: &'a Value, name: &str) -> &'a Value {
    comparison["categories"]
        .as_array()
        .unwrap_or_else(|| panic!("no categories in {comparison}"))
        .iter()
        .find(|c| c["category"] == name)
        .unwrap_or_else(|| panic!("no {name} in {comparison}"))
}

/// The shape every comparison has, whatever it found.
fn well_formed(comparison: &Value) {
    assert_eq!(
        comparison["format"], "ekr.store-comparison/1",
        "{comparison}"
    );
    let names: Vec<&str> = comparison["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["category"].as_str().unwrap())
        .collect();
    assert_eq!(names, CATEGORIES, "{comparison}");
    let every = comparison["categories"]
        .as_array()
        .unwrap()
        .iter()
        .all(|c| c["equal"] == true);
    assert_eq!(comparison["equal"], every, "{comparison}");
}

#[test]
fn stores_built_from_the_same_documents_at_different_times_compare_equal_on_every_provider_pair() {
    for left_backend in BACKENDS {
        for right_backend in BACKENDS {
            let left = Built::new(left_backend, Variant::Base, 1_000_000);
            let right = Built::new(right_backend, Variant::Base, 9_000_000);
            assert_ne!(
                left.head()["root"],
                right.head()["root"],
                "the two stores' commit times differ, so their roots must: \
                 otherwise this case does not show that commit times are ignored"
            );
            let comparison = left.compare(&right, &[]);
            well_formed(&comparison);
            assert_eq!(comparison["equal"], true, "{comparison}");
            assert_eq!(comparison["left"]["revision"], 1);
            assert_eq!(comparison["right"]["revision"], 1);
            for (name, count) in [
                ("types", 3),
                ("properties", 1),
                ("nodes", 4),
                ("edges", 1),
                ("assertions", 4),
                ("evidence", 2),
            ] {
                let found = category(&comparison, name);
                assert_eq!(found["left"], count, "{name}: {comparison}");
                assert_eq!(found["right"], count, "{name}: {comparison}");
                assert_eq!(found["differences"], 0, "{name}: {comparison}");
                assert_eq!(found["differing"], serde_json::json!([]), "{name}");
            }
        }
    }
}

#[test]
fn the_compared_revisions_are_the_given_ones() {
    let left = Built::new("file", Variant::Base, 1_000_000);
    let right = Built::new("sqlite", Variant::Base, 9_000_000);
    let seeds = left.compare(&right, &["--at", "0", "--other-at", "0"]);
    well_formed(&seeds);
    assert_eq!(seeds["equal"], true, "{seeds}");
    assert_eq!(seeds["left"]["revision"], 0);
    assert_eq!(seeds["right"]["revision"], 0);
    let ahead = left.compare(&right, &["--other-at", "0"]);
    well_formed(&ahead);
    assert_eq!(ahead["equal"], false, "{ahead}");
    let assertions = category(&ahead, "assertions");
    assert_eq!(assertions["left"], 4);
    assert_eq!(assertions["right"], 3);
    assert_eq!(
        assertions["differing"],
        serde_json::json!([{
            "id": "00000000-0000-4000-8000-000000000501",
            "presence": "left",
            "fields": []
        }])
    );
}

#[test]
fn a_store_that_differs_by_one_thing_compares_unequal_and_names_its_id_and_field() {
    let cases = [
        (
            Variant::Object,
            "assertions",
            BOB_CEO,
            "both",
            vec!["object"],
        ),
        (Variant::Alias, "nodes", BOB, "both", vec!["aliases"]),
        (
            Variant::PayloadHash,
            "evidence",
            BOB_EVIDENCE,
            "both",
            vec!["content_hash"],
        ),
        (Variant::ExtraNode, "nodes", CAROL, "right", vec![]),
    ];
    for (left_backend, right_backend) in [("file", "sqlite"), ("sqlite", "file")] {
        let base = Built::new(left_backend, Variant::Base, 1_000_000);
        for (variant, name, id, presence, fields) in &cases {
            let other = Built::new(right_backend, *variant, 9_000_000);
            let comparison = base.compare(&other, &[]);
            well_formed(&comparison);
            assert_eq!(comparison["equal"], false, "{variant:?}: {comparison}");
            let found = category(&comparison, name);
            assert_eq!(found["equal"], false, "{variant:?}: {comparison}");
            assert_eq!(found["differences"], 1, "{variant:?}: {comparison}");
            assert_eq!(
                found["differing"],
                serde_json::json!([{ "id": id, "presence": presence, "fields": fields }]),
                "{variant:?}: {comparison}"
            );
            for other_name in CATEGORIES.iter().filter(|c| *c != name) {
                assert_eq!(
                    category(&comparison, other_name)["equal"],
                    true,
                    "{variant:?}: {other_name} in {comparison}"
                );
            }
        }
    }
}

#[test]
fn compare_writes_nothing_to_either_store() {
    for (left_backend, right_backend) in [
        ("file", "file"),
        ("sqlite", "sqlite"),
        ("file", "sqlite"),
        ("sqlite", "file"),
    ] {
        let left = Built::new(left_backend, Variant::Base, 1_000_000);
        let right = Built::new(right_backend, Variant::Alias, 9_000_000);
        let (left_head, right_head) = (left.head(), right.head());
        let (left_bytes, right_bytes) = (left.bytes(), right.bytes());
        let comparison = left.compare(&right, &[]);
        assert_eq!(comparison["equal"], false, "{comparison}");
        let at_zero = left.compare(&right, &["--at", "0", "--other-at", "0"]);
        well_formed(&at_zero);
        assert_eq!(
            left.bytes(),
            left_bytes,
            "{left_backend}: compare changed the left store's files"
        );
        assert_eq!(
            right.bytes(),
            right_bytes,
            "{right_backend}: compare changed the other store's files"
        );
        assert_eq!(left.head(), left_head);
        assert_eq!(right.head(), right_head);
    }
}
