//! `story:schema-transaction-cites-evidence`, the ontology's half: a schema version records the
//! evidence the transaction that made it cited, and a version that cites nothing encodes exactly
//! as every version did before the field existed (design § 109).
//!
//! The pinned roots below were computed by this file's first case at the wave's base, before
//! `SchemaVersion` had an `evidence` field. They are the ontology roots of a seed version and of a
//! version evolved from it; a store records such roots in every revision it commits, so a change
//! to either would stop every existing store from reopening.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EvidenceId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaChange, SchemaVersion};

fn id<T: std::str::FromStr>(n: u128) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

/// A seed version declaring one node type, `Note`, and the version that adds `Topic` to it.
fn seed_and_next() -> (Ontology, SchemaChange) {
    let seed = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(id::<SchemaVersionId>(1), Timestamp::EPOCH),
        node_types: vec![NodeType::new(id::<TypeId>(0x101), "Note")],
        edge_types: Vec::new(),
    })
    .unwrap();
    let change = SchemaChange::DefineNodeType(NodeType::new(id::<TypeId>(0x102), "Topic"));
    (seed, change)
}

/// The ontology root of a seed and of a version evolved without a citation, pinned at the base.
#[test]
fn a_version_that_cites_nothing_keeps_the_root_it_had_before_versions_could_cite() {
    let (seed, change) = seed_and_next();
    let next = seed
        .evolve(
            id::<SchemaVersionId>(2),
            Timestamp::from_millis(1_000),
            &[change],
        )
        .unwrap();
    assert_eq!(
        ContentHash::of(&seed).to_string(),
        "f9982e275c52cf83ff7ad5506abd6868b60f38bff3cdc83ec37f6db17fcab2d6",
        "the seed's ontology root"
    );
    assert_eq!(
        ContentHash::of(&next).to_string(),
        "904c85ae8c7554d75167bc276a5f2b9a3cb64860109fa8c2b9a490c369988342",
        "the evolved version's ontology root"
    );
}

/// `evolve_citing` records the cited evidence in the next version; the version's root differs
/// from the same version citing nothing; an empty citation is `evolve` exactly; and the version
/// survives its document, which omits `evidence` when it is empty.
#[test]
fn a_citing_version_records_its_evidence_and_has_a_root_of_its_own() {
    let (seed, change) = seed_and_next();
    let cited = BTreeSet::from([id::<EvidenceId>(0x401), id::<EvidenceId>(0x402)]);
    let at = Timestamp::from_millis(1_000);
    let citing = seed
        .evolve_citing(
            id::<SchemaVersionId>(2),
            at,
            std::slice::from_ref(&change),
            cited.clone(),
        )
        .unwrap();
    assert_eq!(citing.version().evidence, cited);
    assert_eq!(citing.version().number, 1);

    let plain = seed
        .evolve(id::<SchemaVersionId>(2), at, std::slice::from_ref(&change))
        .unwrap();
    assert!(plain.version().evidence.is_empty());
    assert_ne!(ContentHash::of(&citing), ContentHash::of(&plain));
    let empty = seed
        .evolve_citing(
            id::<SchemaVersionId>(2),
            at,
            std::slice::from_ref(&change),
            BTreeSet::new(),
        )
        .unwrap();
    assert_eq!(empty, plain);
    assert_eq!(ContentHash::of(&empty), ContentHash::of(&plain));

    let yaml = serde_yaml_ng::to_string(&citing.to_document()).unwrap();
    assert!(yaml.contains("evidence:"), "{yaml}");
    let back = Ontology::load(serde_yaml_ng::from_str(&yaml).unwrap()).unwrap();
    assert_eq!(back, citing);
    assert_eq!(ContentHash::of(&back), ContentHash::of(&citing));

    let yaml = serde_yaml_ng::to_string(&plain.to_document()).unwrap();
    assert!(!yaml.contains("evidence"), "{yaml}");
}
