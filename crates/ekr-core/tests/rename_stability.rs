//! The acceptance of `story:kernel-identity-and-hashing`: an id is not a function of a name.
//!
//! Design § 6.4 and § 10, and invariant 3 of `AGENTS.md`: a human-readable name is a property,
//! and identity is minted. The acceptance states the half of that which `ekr-core` can verify —
//! *"Two values that share a name never share an id: over arbitrary pairs of names, minted
//! `NodeId`s are always distinct."*
//!
//! It was restated on 2026-09-21, during wave p1-02. It previously read "a property test renames
//! a node's `canonical_name` a thousand times and its `NodeId` never changes", and the two cases
//! stating it could not fail: `Node` does not exist in this crate, so the fixture renamed its own
//! field and compared a `Copy` id to a copy of itself. The adversary measured it — with `mint()`
//! mutated to return a constant for every id, both rename cases still passed. They are gone, and
//! the rename invariant is stated where a real `Node` exists, in
//! `story:graph-model-and-assertions`.
//!
//! What is left is mutation-sensitive by construction: every case below goes red if `mint()`
//! stops minting.

use ekr_core::identity::*;
use proptest::prelude::*;

/// A thing with a stable id and a human-readable name — the two halves the invariant is about.
///
/// `ekr-graph`'s `Node` reduced to what the acceptance names. It is a fixture, not a draft of
/// that type.
#[derive(Debug, Clone)]
struct NamedThing {
    id: NodeId,
    canonical_name: String,
}

impl NamedThing {
    fn new(canonical_name: &str) -> Self {
        Self {
            id: NodeId::mint(),
            canonical_name: canonical_name.to_owned(),
        }
    }

    fn rename(&mut self, canonical_name: &str) {
        self.canonical_name = canonical_name.to_owned();
    }
}

proptest! {
    /// The acceptance. Whatever two names are chosen — equal names included, which is the case
    /// that matters — the ids minted for them differ.
    #[test]
    fn two_values_that_share_a_name_never_share_an_id(
        left_name in ".{0,32}",
        right_name in ".{0,32}",
    ) {
        let left = NamedThing::new(&left_name);
        let right = NamedThing::new(&right_name);

        prop_assert_ne!(left.id, right.id);
        prop_assert_eq!(&left.canonical_name, &left_name);
        prop_assert_eq!(&right.canonical_name, &right_name);
    }
}

#[test]
fn the_same_name_twice_is_two_identities() {
    let one = NamedThing::new("the same name");
    let other = NamedThing::new("the same name");

    assert_eq!(one.canonical_name, other.canonical_name);
    assert_ne!(
        one.id, other.id,
        "a minted id is the value's, not the name's"
    );
}

#[test]
fn renaming_two_things_to_one_name_does_not_merge_them() {
    // The rename half of the invariant, in the only form this crate can state it: a name is not
    // where identity lives, so making two names equal does not make two things one.
    let mut one = NamedThing::new("first");
    let mut other = NamedThing::new("second");

    one.rename("the same name");
    other.rename("the same name");

    assert_eq!(one.canonical_name, other.canonical_name);
    assert_ne!(one.id, other.id, "two things merged under one name");
}

#[test]
fn a_burst_of_mints_is_all_distinct() {
    // A thousand ids in one millisecond window: UUIDv7's timestamp does not separate them, so
    // this is the random half doing the work.
    let minted: std::collections::BTreeSet<NodeId> = (0..1000).map(|_| NodeId::mint()).collect();

    assert_eq!(minted.len(), 1000, "a mint repeated itself");
}

/// The class the acceptance is one member of: no id type mints a constant. Stated over every one
/// of the declared identity types, because "`NodeId` mints distinct values" is a property of
/// the macro they all come from.
#[test]
fn no_id_type_mints_a_constant() {
    macro_rules! assert_mints_distinct {
        ($($type:ident),+ $(,)?) => {
            $(
                assert_ne!(
                    $type::mint(),
                    $type::mint(),
                    concat!(stringify!($type), "::mint returned the same id twice")
                );
            )+
        };
    }

    assert_mints_distinct!(
        AgentId,
        TransactionId,
        RevisionId,
        IssueId,
        TypeId,
        PropertyId,
        SchemaVersionId,
        GraphRootId,
        NodeId,
        EdgeId,
        AssertionId,
        SupportId,
        EvidenceId,
        ObservationId,
        EventId,
    );
}
