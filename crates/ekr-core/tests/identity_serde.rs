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

id_cases! {
    agent_id => AgentId,
    transaction_id => TransactionId,
    revision_id => RevisionId,
    issue_id => IssueId,
    type_id => TypeId,
    property_id => PropertyId,
    schema_version_id => SchemaVersionId,
    graph_root_id => GraphRootId,
    node_id => NodeId,
    edge_id => EdgeId,
    assertion_id => AssertionId,
    support_id => SupportId,
    evidence_id => EvidenceId,
    observation_id => ObservationId,
    event_id => EventId,
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
fn ess_uuid_newtypes(domain_file: &str) -> Vec<String> {
    let path = workspace_root()
        .join("systems/ekr/domains")
        .join(domain_file);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));

    let mut found = Vec::new();
    let mut current: Option<String> = None;
    let mut is_newtype = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name: ") {
            current = name.rsplit('.').next().map(str::to_owned);
            is_newtype = false;
        } else if trimmed == "kind: newtype" {
            is_newtype = true;
        } else if trimmed == "of: Uuid" && is_newtype {
            if let Some(name) = current.take() {
                found.push(name);
            }
        }
    }
    found
}

#[test]
fn every_ess_id_type_exists_in_the_crate() {
    // Every domain of the runtime. `store.yaml` declares no id newtype since `ekr.store.SnapshotId`
    // was removed in wave p1-14; it is scanned so that an id declared there must be carried here too.
    let mut declared: Vec<String> = ["kernel.yaml", "ontology.yaml", "graph.yaml", "store.yaml"]
        .into_iter()
        .flat_map(ess_uuid_newtypes)
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

/// The class the adversary's `adversary_id_text_form.rs` states for the fourteen UUID ids, over
/// the fifteenth member of the same class.
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
