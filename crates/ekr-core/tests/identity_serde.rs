//! Every id type round-trips through serde as a string and refuses a malformed one.
//!
//! The third of `story:kernel-identity-and-hashing`'s "Tests the story ships". An id crosses a
//! process boundary as text — a stored record, an event payload, a CLI argument — so the text form
//! is part of the type, and a type that accepts `"not-a-uuid"` has no identity to speak of.
//!
//! The class this states is "every id type", so the cases enumerate the id types rather than
//! sampling them, and `every_ess_id_type_exists_in_the_crate` holds the enumeration to the ESS
//! declarations it comes from: an id added to `systems/ekr/domains/` and not to `identity.rs` is a
//! red case, not a discovery for a later reader.

use std::fmt::{Debug, Display};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use ekr_core::identity::*;
use proptest::prelude::*;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[path = "support/identity_types.rs"]
mod identity_types;

/// The contract every minted id type keeps, stated once.
fn assert_id_contract<T>(minted: T)
where
    T: Serialize + DeserializeOwned + FromStr + Display + Debug + PartialEq + Copy,
    <T as FromStr>::Err: Debug,
{
    let text = minted.to_string();
    let json = serde_json::to_string(&minted).expect("an id serialises");

    assert_eq!(json, format!("\"{text}\""), "an id is serde'd as its text");
    assert_eq!(text.len(), 36, "the hyphenated UUID form: {text}");

    let from_json: T = serde_json::from_str(&json).expect("an id deserialises");
    assert_eq!(from_json, minted, "serde round-trip");

    let from_text: T = text.parse().expect("an id parses from its own text");
    assert_eq!(from_text, minted, "FromStr round-trip");

    for malformed in [
        "\"\"",
        "\"not-a-uuid\"",
        "\"0d90ba2a-2b6c-7e5e-9a5c\"",
        "\"0d90ba2a-2b6c-7e5e-9a5c-1f0c2b3d4e5f6\"",
        "\"0d90ba2a2b6c7e5e9a5c1f0c2b3d4e5g\"",
        "42",
        "null",
        "[]",
    ] {
        assert!(
            serde_json::from_str::<T>(malformed).is_err(),
            "deserialising {malformed} must be refused"
        );
    }

    for malformed in ["", " ", "not-a-uuid", "0d90ba2a-2b6c-7e5e-9a5c"] {
        assert!(
            malformed.parse::<T>().is_err(),
            "parsing {malformed:?} must be refused"
        );
    }
}

/// One case per id type, plus one quantified over arbitrary bits — leading zeros and the
/// all-ones value included, which a minted sample never produces.
macro_rules! id_cases {
    ($($module:ident => $type:ident),+ $(,)?) => {
        $(
            mod $module {
                use super::*;

                #[test]
                fn round_trips_as_a_string_and_refuses_a_malformed_one() {
                    assert_id_contract($type::mint());
                }

                proptest! {
                    #[test]
                    fn round_trips_for_any_bits(bits in any::<u128>()) {
                        let id = $type::from_uuid(uuid::Uuid::from_u128(bits));
                        let json = serde_json::to_string(&id).expect("an id serialises");
                        prop_assert_eq!(serde_json::from_str::<$type>(&json).unwrap(), id);
                        prop_assert_eq!(id.to_string().parse::<$type>().unwrap(), id);
                    }
                }
            }
        )+

        /// The names the cases above enumerate, for the ESS comparison below.
        const ENUMERATED: &[&str] = &[$(stringify!($type)),+];
    };
}

identity_types::identity_types!(id_cases);

/// Exercise directory discovery through the real guard, rather than a second scanner.
mod adversary_input08 {
    use super::*;

    struct Fixture(PathBuf);

    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "ekr-input08-identity-adversary-{}",
                uuid::Uuid::now_v7()
            ));
            std::fs::create_dir_all(root.join("crates/ekr-core")).unwrap();
            let domains = root.join("systems/ekr/domains");
            std::fs::create_dir_all(&domains).unwrap();
            for entry in std::fs::read_dir(workspace_root().join("systems/ekr/domains")).unwrap() {
                let path = entry.unwrap().path();
                if path
                    .extension()
                    .is_some_and(|extension| extension == "yaml")
                {
                    std::fs::copy(&path, domains.join(path.file_name().unwrap())).unwrap();
                }
            }
            Self(root)
        }

        fn domains(&self) -> PathBuf {
            self.0.join("systems/ekr/domains")
        }

        fn scan(&self) -> (bool, String) {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "every_ess_id_type_exists_in_the_crate",
                    "--nocapture",
                ])
                .env("CARGO_MANIFEST_DIR", self.0.join("crates/ekr-core"))
                .output()
                .unwrap();
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                text.contains("running 1 test"),
                "guard was not selected: {text}"
            );
            (output.status.success(), text)
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).expect("remove this test's fixture only");
        }
    }

    #[test]
    fn renamed_domain_files_preserve_the_identity_inventory() {
        let fixture = Fixture::new();
        let paths: Vec<_> = std::fs::read_dir(fixture.domains())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        for (index, path) in paths.into_iter().enumerate() {
            std::fs::rename(
                &path,
                fixture.domains().join(format!("renamed-{index}.yaml")),
            )
            .unwrap();
        }
        let (success, output) = fixture.scan();
        assert!(
            success,
            "domain filenames are not identity declarations: {output}"
        );
    }

    #[test]
    fn semantic_yaml_spelling_does_not_hide_a_new_identity() {
        let fixture = Fixture::new();
        let (success, output) = fixture.scan();
        assert!(success, "unchanged fixture control: {output}");
        for declaration in [
            "types:\n  - of: 'Uuid'\n    kind: \"newtype\"\n    name: ekr.adversary.UnimplementedId\n",
            "types: [{name: ekr.adversary.UnimplementedId, of: Uuid, kind: newtype}]\n",
        ] {
            std::fs::write(fixture.domains().join("new-domain.yaml"), declaration).unwrap();
            let (success, output) = fixture.scan();
            assert!(!success && output.contains("UnimplementedId"), "missing identity escaped: {output}");
        }
    }

    #[test]
    fn removing_a_declared_identity_is_not_hidden_by_the_typed_inventory() {
        let fixture = Fixture::new();
        let path = fixture.domains().join("observe.yaml");
        let source = std::fs::read_to_string(&path).unwrap();
        let declaration =
            "  - name: ekr.observe.SourceCheckpointId\n    kind: newtype\n    of: Uuid\n";
        assert_eq!(
            source.matches(declaration).count(),
            1,
            "fixture declaration changed"
        );
        std::fs::write(path, source.replace(declaration, "")).unwrap();
        let (success, output) = fixture.scan();
        assert!(
            !success && output.contains("SourceCheckpointId"),
            "removed declaration escaped: {output}"
        );
    }

    #[test]
    fn public_observation_ids_keep_uuid_bits_and_canonical_text() {
        use ekr_core::{Canonical, SourceCheckpointId, SourceUnitId};
        let bits = uuid::Uuid::from_u128(u128::MAX);
        let unit = SourceUnitId::from_uuid(bits);
        let checkpoint = SourceCheckpointId::from_uuid(bits);
        assert_eq!(unit.to_uuid(), bits);
        assert_eq!(checkpoint.to_uuid(), bits);
        assert_eq!(unit.canonical_bytes(), checkpoint.canonical_bytes());
        assert_eq!(
            serde_json::to_string(&unit).unwrap(),
            "\"ffffffff-ffff-ffff-ffff-ffffffffffff\""
        );
        assert_eq!(
            serde_json::to_string(&checkpoint).unwrap(),
            "\"ffffffff-ffff-ffff-ffff-ffffffffffff\""
        );
        for malformed in [
            "FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF",
            "ffffffffffffffffffffffffffffffff",
        ] {
            assert!(malformed.parse::<SourceUnitId>().is_err());
            assert!(malformed.parse::<SourceCheckpointId>().is_err());
        }
        assert_eq!(SourceUnitId::mint().to_uuid().get_version_num(), 7);
        assert_eq!(SourceCheckpointId::mint().to_uuid().get_version_num(), 7);
    }
}

fn workspace_root() -> PathBuf {
    std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .parent()
    .and_then(Path::parent)
    .expect("crates/ekr-core has a workspace root two levels up")
    .to_path_buf()
}

/// Every `kind: newtype, of: Uuid` declaration of one ESS domain file, by its bare Rust name.
///
/// Read from the document as `serde_yaml_ng` parses it, so each key is found wherever it sits in
/// its mapping: the line scan this replaced saw a declaration only when `name:` came first
/// (adversary pass 1, wave p1-14).
fn ess_uuid_newtypes(path: &Path) -> Vec<String> {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

    let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text)
        .unwrap_or_else(|e| panic!("parsing {}: {e}", path.display()));
    let sections = document
        .as_mapping()
        .unwrap_or_else(|| panic!("{} is not a mapping", path.display()));
    let mut found = Vec::new();
    for section in sections.values() {
        for declared in section.as_sequence().map_or(&[][..], Vec::as_slice) {
            let field = |key: &str| declared.get(key).and_then(serde_yaml_ng::Value::as_str);
            if field("kind") == Some("newtype") && field("of") == Some("Uuid") {
                let name = field("name").unwrap_or_else(|| {
                    panic!(
                        "a Uuid newtype in {} has no name: {declared:?}",
                        path.display()
                    )
                });
                found.push(
                    name.rsplit('.')
                        .next()
                        .expect("a name has a last segment")
                        .to_owned(),
                );
            }
        }
    }
    found
}

#[test]
fn every_ess_id_type_exists_in_the_crate() {
    let domains = workspace_root().join("systems/ekr/domains");
    let mut paths: Vec<_> = std::fs::read_dir(&domains)
        .unwrap_or_else(|error| panic!("listing {}: {error}", domains.display()))
        .map(|entry| entry.expect("a domain directory entry is readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "yaml")
        })
        .collect();
    paths.sort();
    let mut declared: Vec<String> = paths
        .iter()
        .flat_map(|path| ess_uuid_newtypes(path))
        .collect();
    declared.sort();

    let mut enumerated: Vec<String> = ENUMERATED.iter().map(|n| (*n).to_owned()).collect();
    enumerated.sort();

    assert!(
        !declared.is_empty(),
        "the ESS scan found nothing; the scan is broken, not the crate"
    );
    assert_eq!(
        enumerated, declared,
        "the id types this suite enumerates and the ESS declarations must be the same set"
    );
}

/// Exercise the actual inventory guard in a separate process, with a fixture workspace supplied
/// through Cargo's runtime path. A new domain must be found without editing a filename list.
#[test]
fn an_unknown_domain_filename_with_a_missing_identity_fails_the_actual_scan() {
    let fixture = std::env::temp_dir().join(format!("ekr-identity-scan-{}", uuid::Uuid::now_v7()));
    let domains = fixture.join("systems/ekr/domains");
    std::fs::create_dir_all(&domains).unwrap();
    let manifest = fixture.join("crates/ekr-core");
    std::fs::create_dir_all(&manifest).unwrap();
    for entry in std::fs::read_dir(workspace_root().join("systems/ekr/domains")).unwrap() {
        let path = entry.unwrap().path();
        if path
            .extension()
            .is_some_and(|extension| extension == "yaml")
        {
            std::fs::copy(&path, domains.join(path.file_name().unwrap())).unwrap();
        }
    }
    let scan = || {
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "every_ess_id_type_exists_in_the_crate",
                "--nocapture",
            ])
            .env("CARGO_MANIFEST_DIR", &manifest)
            .output()
            .unwrap()
    };
    let control = scan();
    assert!(
        control.status.success(),
        "the unchanged domains must pass: {}{}",
        String::from_utf8_lossy(&control.stdout),
        String::from_utf8_lossy(&control.stderr)
    );
    std::fs::write(
        domains.join("a-new-domain-never-listed-before.yaml"),
        "types:\n  - {of: Uuid, name: ekr.fixture.MissingId, kind: newtype}\n",
    )
    .unwrap();
    let missing = scan();
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&missing.stdout),
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(
        !missing.status.success() && output.contains("MissingId"),
        "the actual scanner must refuse and name an unimplemented identity from a new domain: {output}"
    );
    std::fs::remove_dir_all(fixture).unwrap();
}

#[test]
fn existing_observe_identities_have_typed_contract_cases() {
    for declared in ess_uuid_newtypes(&workspace_root().join("systems/ekr/domains/observe.yaml")) {
        assert!(
            ENUMERATED.contains(&declared.as_str()),
            "existing observe identity {declared} has no typed serde/mint contract case"
        );
    }
}

/// Adversary, wave p2p3p4-01 unit I: the `id_newtype!` rustdoc states how many types share the
/// macro's shape. The count it states must be the count of invocations in `identity.rs`.
#[test]
fn adversary_i_macro_doc_counts_the_id_newtypes_it_declares() {
    const WORDS: [&str; 22] = [
        "Zero",
        "One",
        "Two",
        "Three",
        "Four",
        "Five",
        "Six",
        "Seven",
        "Eight",
        "Nine",
        "Ten",
        "Eleven",
        "Twelve",
        "Thirteen",
        "Fourteen",
        "Fifteen",
        "Sixteen",
        "Seventeen",
        "Eighteen",
        "Nineteen",
        "Twenty",
        "Twenty-one",
    ];
    let path = workspace_root().join("crates/ekr-core/src/identity.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    let invocations = source
        .lines()
        .filter(|line| line.trim_end() == "id_newtype! {")
        .count();
    assert_eq!(
        invocations,
        ENUMERATED.len(),
        "the scan is broken, not the doc"
    );
    let stated = WORDS
        .iter()
        .position(|word| source.contains(&format!("/// {word} types share this shape")))
        .expect("the macro doc states a count");
    assert_eq!(
        stated, invocations,
        "identity.rs says {} types share the id_newtype! shape; it declares {invocations}",
        WORDS[stated]
    );
}

/// `RevisionNumber` is in the story's scope beside the ids and is not one: the ESS declares it
/// `kind: newtype, of: Integer` and design § 34 gives `Root.revision` as a `u64`. So it crosses a
/// boundary as a number, and the case states that rather than letting the id rule cover it.
mod revision_number {
    use super::*;

    #[test]
    fn round_trips_as_a_number_and_refuses_a_malformed_one() {
        let number = RevisionNumber::new(7);

        assert_eq!(
            serde_json::to_string(&number).expect("a revision number serialises"),
            "7"
        );
        assert_eq!(
            serde_json::from_str::<RevisionNumber>("7").expect("a revision number deserialises"),
            number
        );

        for malformed in ["\"7\"", "\"\"", "-1", "null", "7.5", "[]"] {
            assert!(
                serde_json::from_str::<RevisionNumber>(malformed).is_err(),
                "deserialising {malformed} must be refused"
            );
        }
    }

    proptest! {
        #[test]
        fn round_trips_for_any_number(number in any::<u64>()) {
            let number = RevisionNumber::new(number);
            let json = serde_json::to_string(&number).expect("a revision number serialises");
            prop_assert_eq!(serde_json::from_str::<RevisionNumber>(&json).unwrap(), number);
            prop_assert_eq!(number.to_string().parse::<RevisionNumber>().unwrap(), number);
        }
    }
}

/// The class the adversary's `adversary_id_text_form.rs` states for the UUID ids, over the one
/// member of the same class that is not a UUID.
///
/// `RevisionNumber` writes one text — `7` — and `u64::from_str` reads four more: `+7`, `007`,
/// and either with more leading zeros. A type whose `Display` documents one form must refuse
/// every other spelling of it, whatever the form is made of; the rule is not about UUIDs.
mod revision_number_text_form {
    use super::*;

    #[test]
    fn a_second_spelling_of_a_number_is_refused() {
        let number = RevisionNumber::new(7);
        assert_eq!(number.to_string(), "7");

        for spelling in ["+7", "007", "0000000000007", " 7", "7 ", "7\n", "", "-0"] {
            assert!(
                spelling.parse::<RevisionNumber>().is_err(),
                "parsing {spelling:?} must be refused: it is not the text `Display` writes"
            );
        }

        assert_eq!(
            "0".parse::<RevisionNumber>().expect("zero parses"),
            RevisionNumber::SEED
        );
        assert_eq!(
            "7".parse::<RevisionNumber>()
                .expect("the written form parses"),
            number
        );
    }

    proptest! {
        /// The only text a `RevisionNumber` reads is the text it writes.
        #[test]
        fn the_written_form_is_the_only_form(number in any::<u64>()) {
            let number = RevisionNumber::new(number);
            let text = number.to_string();
            prop_assert_eq!(text.parse::<RevisionNumber>().unwrap(), number);
            let plus = format!("+{text}");
            let padded = format!("0{text}");
            prop_assert!(plus.parse::<RevisionNumber>().is_err());
            prop_assert!(padded.parse::<RevisionNumber>().is_err());
        }
    }
}
