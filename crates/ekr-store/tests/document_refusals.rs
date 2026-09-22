//! Public structured document refusals remain usable without granting canonical authority.

use std::error::Error;

use ekr_core::{AssertionId, EdgeId, EvidenceId, NodeId};
use ekr_store::{Entity, MembraneError, StoreError};

#[test]
fn entity_display_retains_each_kind_and_stable_identity() {
    let node = NodeId::mint();
    let edge = EdgeId::mint();
    let assertion = AssertionId::mint();
    let evidence = EvidenceId::mint();
    for (entity, expected) in [
        (Entity::Node(node), format!("node {node}")),
        (Entity::Edge(edge), format!("edge {edge}")),
        (
            Entity::Assertion(assertion),
            format!("assertion {assertion}"),
        ),
        (Entity::Evidence(evidence), format!("evidence {evidence}")),
    ] {
        assert_eq!(entity.to_string(), expected);
    }
}

#[test]
fn structured_membrane_refusal_survives_store_error_conversion() {
    let key = Entity::Node(NodeId::mint());
    let found = Entity::Node(NodeId::mint());
    let refusal = MembraneError::Misfiled { key, found };
    let expected =
        format!("{key} is filed under a key that is not its own id: the record there is {found}");
    assert_eq!(refusal.to_string(), expected);
    let stored: StoreError = refusal.clone().into();
    assert!(matches!(&stored, StoreError::Membrane(actual) if actual == &refusal));
    assert_eq!(
        stored
            .source()
            .expect("the structured error remains a source")
            .to_string(),
        expected
    );
    assert!(stored.to_string().contains(&expected));
}
