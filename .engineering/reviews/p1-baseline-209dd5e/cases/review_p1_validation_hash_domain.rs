//! Independent review of the P1 core: which address space `validation_hash` lives in.
//!
//! `crates/ekr-core/src/hash.rs` publishes two domains and says why they never share an address:
//! a *value* is addressed by `ContentHash::of` over its canonical encoding, a *payload* from
//! outside the runtime by `ContentHash::of_bytes`, and "`of(&v)` is never
//! `of_bytes(&v.canonical_bytes())`; the two live in different domains, and that is the point."
//!
//! `ValidatedTransaction::seal` (`crates/ekr-kernel/src/transaction.rs`) encodes the transaction
//! and the revision canonically and then takes `ContentHash::of_bytes` over those bytes — the exact
//! composition the core crate forbids. The consequence is the collision the two domains exist to
//! prevent: a payload whose bytes happen to be that encoding takes the validation hash's address,
//! through the public `ObjectStore::put`. The hash is recorded in every `TransactionValidated`
//! event for the lifetime of the lineage, so this is cheap now and a migration later.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, ContentHash, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Space};
use ekr_kernel::{GraphOperation, GraphTransaction, NodeDraft, Pipeline};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};

#[test]
fn the_validation_hash_is_a_value_address_and_not_a_payload_address() {
    let (root_id, schema, decision) =
        (GraphRootId::mint(), SchemaVersionId::mint(), TypeId::mint());
    let graph = CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: schema,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(7),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![NodeType::new(decision, "Decision")],
            edge_types: Vec::new(),
        })
        .expect("one node type coheres"),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    let proposal = GraphTransaction {
        id: TransactionId::mint(),
        proposer: AgentId::mint(),
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id,
            type_id: decision,
            canonical_name: "hash canonical state only".to_owned(),
            properties: BTreeMap::new(),
        })],
        evidence: BTreeSet::new(),
    };
    let validated = Pipeline::deterministic(AgentId::mint())
        .validate(&GraphSnapshot::of(&graph), &proposal)
        .expect("nothing is wrong with this proposal");

    // The bytes `seal` encodes, rebuilt from the public accessors and the public encoder.
    let mut encoder = Encoder::new();
    validated.transaction().encode(&mut encoder);
    validated.validated_against().encode(&mut encoder);
    let as_a_payload = ContentHash::of_bytes(encoder.as_bytes());

    assert_ne!(
        validated.validation_hash(),
        as_a_payload,
        "ekr-core's contract is that a value's canonical bytes hashed as a payload never share an \
         address with the value; the validation hash is exactly that composition, so bytes handed \
         to ObjectStore::put by anyone take the address of a validation the kernel performed"
    );
}
