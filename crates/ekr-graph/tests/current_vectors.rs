//! Fixed current-format vectors: amendment 88's assertion (assessment then lifecycle), amendment
//! 89's `ekr.revision-event/2` envelope with each of its six payload kinds, and the § 34 revision
//! root. The frozen originals live in `tests/legacy_status.rs` and the kernel's `tests/legacy.rs`;
//! these are their current-format counterparts, captured from this tree and pinned as literals so
//! that any encoder change — a dropped field, a reordered field, a renumbered tag — moves a
//! number a reader can see.
//!
//! Two of the pins are not only captured: `a_rejected_event_matches_its_hand_derived_bytes` and
//! `the_seed_root_matches_its_hand_derived_bytes` rebuild the expected bytes from the tag table in
//! `ekr_core::canonical`, independently of any encoder, so a captured literal that merely echoed a
//! defect would disagree with them.

use std::collections::BTreeSet;

use ekr_core::canonical::Canonical;
use ekr_core::{
    AgentId, AssertionId, ContentHash, EventId, EvidenceId, IssueId, NodeId, PropertyId,
    RevisionId, RevisionNumber, Timestamp, TransactionId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, CanonicalValue, Object, Predicate,
    RetractionReason, RevisionEvent, RevisionPayload, Root, Subject, TemporalRange,
    TransactionTime,
};

/// A fixed identity: the same UUID text form the kernel fixtures use, with `n` in the last group.
fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every mismatch at once, with its actual value, so one red run is enough to read them all.
#[derive(Default)]
struct Pins(Vec<String>);
impl Pins {
    fn check(&mut self, name: &str, actual: String, expected: &str) {
        if actual != expected {
            self.0.push(format!("{name}: actual {actual}"));
        }
    }
    fn finish(self) {
        assert!(self.0.is_empty(), "\n{}", self.0.join("\n"));
    }
}

fn base_assertion() -> Assertion {
    Assertion {
        id: id(0x10),
        root_id: id(0x11),
        subject: Subject::Node(CanonicalRef::new(id::<NodeId>(0x12))),
        predicate: Predicate::Property(id::<PropertyId>(0x13)),
        object: Object::Value(CanonicalValue::String("fixed".into())),
        evidence: BTreeSet::from([id::<EvidenceId>(0x14)]),
        proposed_by: id(0x15),
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::new(
            Some(Timestamp::from_millis(1_000)),
            Some(Timestamp::from_millis(2_000)),
        )
        .unwrap(),
        transaction_time: TransactionTime::since(Timestamp::from_millis(500)),
    }
}

/// One assertion per assessment kind and one per lifecycle kind, all over the same base.
fn assertions() -> Vec<(&'static str, Assertion)> {
    let with = |assessment: Assessment, lifecycle: AssertionLifecycle| Assertion {
        assessment,
        lifecycle,
        ..base_assertion()
    };
    vec![
        ("proposed-active", base_assertion()),
        (
            "validating",
            with(
                Assessment::Validating {
                    completed: 2,
                    required: 7,
                },
                AssertionLifecycle::Active,
            ),
        ),
        (
            "accepted",
            with(
                Assessment::Accepted {
                    validators: BTreeSet::from([id::<AgentId>(0x20), id::<AgentId>(0x21)]),
                },
                AssertionLifecycle::Active,
            ),
        ),
        (
            "rejected",
            with(
                Assessment::Rejected {
                    issues: vec![id::<IssueId>(0x23), id::<IssueId>(0x22)],
                },
                AssertionLifecycle::Active,
            ),
        ),
        (
            "disputed",
            with(
                Assessment::Disputed {
                    competing_assertions: vec![id::<AssertionId>(0x24)],
                },
                AssertionLifecycle::Active,
            ),
        ),
        (
            "accepted-retracted",
            with(
                Assessment::Accepted {
                    validators: BTreeSet::from([id::<AgentId>(0x20)]),
                },
                AssertionLifecycle::Retracted {
                    at_revision: RevisionNumber::new(4),
                    reason: RetractionReason::new("withdrawn by its source"),
                },
            ),
        ),
        (
            "accepted-superseded",
            with(
                Assessment::Accepted {
                    validators: BTreeSet::from([id::<AgentId>(0x20)]),
                },
                AssertionLifecycle::Superseded {
                    by: id(0x25),
                    at_revision: RevisionNumber::new(5),
                    effective_from: Timestamp::from_millis(1_500),
                },
            ),
        ),
    ]
}

#[test]
fn every_current_assessment_and_lifecycle_kind_has_a_fixed_assertion_address() {
    let expected = [
        (
            "proposed-active",
            "8671d08638a4ec9e1ed5ec114a016ac57485a6c6ecea62fd29ea29ee5edd7656",
        ),
        (
            "validating",
            "4d336d04b93315e959138d8373851524baf34e0cecd9ff6a56ce3bde31b798ce",
        ),
        (
            "accepted",
            "373bd03c42f94015b6d80e3c76476cbef16296d24b64ca457ef40c255730c934",
        ),
        (
            "rejected",
            "80738b4ea8bb17c52988af420fae4227da27e27e06def0d715d529b447070037",
        ),
        (
            "disputed",
            "60ee3853563629fd878c73bd8be0c28efea93edab0ea187ad3f96f2ca52a93f3",
        ),
        (
            "accepted-retracted",
            "a8514b0529310453389e7b1807c5dbead7b828e2c4395eff821d55c3df0966d7",
        ),
        (
            "accepted-superseded",
            "23338823814fdf0ed9ae5e9893cd34458a567826100c622e4ce7c931a7a90cde",
        ),
    ];
    let actual = assertions();
    assert_eq!(actual.len(), expected.len());
    let mut pins = Pins::default();
    for ((name, assertion), (pinned_name, pinned)) in actual.iter().zip(expected) {
        assert_eq!(*name, pinned_name);
        pins.check(name, ContentHash::of(assertion).to_hex(), pinned);
    }
    pins.finish();
}

#[test]
fn the_base_assertion_has_fixed_canonical_bytes() {
    let mut pins = Pins::default();
    pins.check(
        "proposed-active bytes",
        hex(&base_assertion().canonical_bytes()),
        "0d000000000000400080000000000000100d000000000000400080000000000000110f000000000d000000000000400080000000000000120f000000000d000000000000400080000000000000130f000000000f0000000006000000000000000566697865640900000000000000010d000000000000400080000000000000140d000000000000400080000000000000150f000000000f000000000c05000000000000000000000000000003e80c05000000000000000000000000000007d005000000000000000000000000000001f40b",
    );
    pins.finish();
}

/// One occurrence of each payload kind over the same identities, numbers and hashes.
fn events() -> Vec<RevisionEvent> {
    let transaction_id: TransactionId = id(0x30);
    let revision_id: RevisionId = id(0x31);
    let payloads = vec![
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: hash(0xa1),
        },
        RevisionPayload::TransactionProposed {
            transaction_id,
            proposer: id(0x32),
            operations_hash: Some(hash(0xa2)),
        },
        RevisionPayload::TransactionValidated {
            transaction_id,
            against: RevisionNumber::new(3),
            validation_hash: hash(0xa3),
        },
        RevisionPayload::TransactionRejected {
            transaction_id,
            issues: 3,
        },
        RevisionPayload::TransactionStale {
            transaction_id,
            validated_against: RevisionNumber::new(3),
            current: RevisionNumber::new(4),
        },
        RevisionPayload::RevisionCommitted {
            transaction_id,
            revision_id,
            number: RevisionNumber::new(4),
            knowledge_root: hash(0xa4),
        },
    ];
    payloads
        .into_iter()
        .map(|payload| RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(0x40),
            record_hash: hash(0xb0),
            payload,
        })
        .collect()
}

#[test]
fn every_current_event_payload_kind_has_fixed_envelope_bytes() {
    let expected: [&str; 6] = [
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000000d000000000000400080000000000000310ea1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1",
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000010d000000000000400080000000000000300d000000000000400080000000000000320c0ea2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2a2",
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000020d0000000000004000800000000000003004000000000000000000000000000000030ea3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3a3",
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000030d000000000000400080000000000000300400000000000000000000000000000003",
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000040d0000000000004000800000000000003004000000000000000000000000000000030400000000000000000000000000000004",
        "060000000000000014656b722e7265766973696f6e2d6576656e742f320d000000000000400080000000000000400eb0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b00f000000050d000000000000400080000000000000300d0000000000004000800000000000003104000000000000000000000000000000040ea4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4a4",
    ];
    let events = events();
    assert_eq!(events.len(), expected.len());
    let mut pins = Pins::default();
    for (index, (event, pinned)) in events.iter().zip(expected).enumerate() {
        assert_eq!(event.variant_index(), index as u32);
        pins.check(event.name(), hex(&event.canonical_bytes()), pinned);
    }
    pins.finish();
}

#[test]
fn every_current_event_payload_kind_has_a_fixed_envelope_address() {
    let expected: [&str; 6] = [
        "bf073820275a257e2898aee3c7abf035b2650c91deb790b5f2e146a2abc183a5",
        "a6f058be52f03be0dbfa2b2c95e196106fdfb566d31b8750c7ac743cde940fbd",
        "88a2f27eb10e66b2e0b585221ce59fb9728e0f8262c983a107d3ec09e0ba329c",
        "70c86117c9da0be8aa82d4940cf108f85b7960f581d7d1839957d705ae5ba23b",
        "a8532a964bc161373d3e8d08a51f277b498fd456ea9cbd51f368d34d3a5f9e39",
        "2854988d5648f937ce03b4c87ee4c3ff0079de4d7267edd05f387b35c2fff48a",
    ];
    let mut pins = Pins::default();
    for (event, pinned) in events().iter().zip(expected) {
        pins.check(event.name(), ContentHash::of(event).to_hex(), pinned);
    }
    pins.finish();
}

/// Amendment 89: "format, event_id, record_hash and payload in that order", written out from the
/// tag table rather than from any encoder.
#[test]
fn a_rejected_event_matches_its_hand_derived_bytes() {
    let format = RevisionEvent::FORMAT;
    let expected = [
        "06".to_owned(),
        format!("{:016x}", format.len()),
        hex(format.as_bytes()),
        format!("0d{:032x}", (0x4000_u128 << 64) | 0x8000_0000_0000_0040),
        format!("0e{}", "b0".repeat(32)),
        "0f00000003".to_owned(),
        format!("0d{:032x}", (0x4000_u128 << 64) | 0x8000_0000_0000_0030),
        format!("04{:032x}", 3_u128),
    ]
    .concat();
    let event = &events()[3];
    assert!(matches!(
        event.payload,
        RevisionPayload::TransactionRejected { .. }
    ));
    assert_eq!(hex(&event.canonical_bytes()), expected);
}

fn roots() -> [Root; 2] {
    let seed = Root {
        revision: RevisionNumber::SEED,
        parent: None,
        ontology_root: hash(0xc1),
        knowledge_root: hash(0xc2),
        evidence_root: hash(0xc3),
        agent_root: hash(0xc4),
        transaction: hash(0xc5),
    };
    let next = Root {
        revision: RevisionNumber::new(1),
        parent: Some(ContentHash::of(&seed)),
        transaction: hash(0xc6),
        ..seed
    };
    [seed, next]
}

#[test]
fn the_seed_root_matches_its_hand_derived_bytes() {
    let expected = [
        format!("04{:032x}", 0_u128),
        "0b".to_owned(),
        format!("0e{}", "c1".repeat(32)),
        format!("0e{}", "c2".repeat(32)),
        format!("0e{}", "c3".repeat(32)),
        format!("0e{}", "c4".repeat(32)),
        format!("0e{}", "c5".repeat(32)),
    ]
    .concat();
    assert_eq!(hex(&roots()[0].canonical_bytes()), expected);
}

#[test]
fn seed_and_successor_roots_have_fixed_addresses() {
    let [seed, next] = roots();
    let mut pins = Pins::default();
    pins.check(
        "seed root",
        ContentHash::of(&seed).to_hex(),
        "136015d8a151e3c605746f54fff48beb55f4cea37eb7ce24dc3fa7566a338fff",
    );
    pins.check(
        "successor root",
        ContentHash::of(&next).to_hex(),
        "ff02aaea06dfa597e5be3314f3ae7f1833506a2958e6371ed56603b9b0947c91",
    );
    pins.check("successor bytes", hex(&next.canonical_bytes()), "04000000000000000000000000000000010c0e136015d8a151e3c605746f54fff48beb55f4cea37eb7ce24dc3fa7566a338fff0ec1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c1c10ec2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c2c20ec3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c3c30ec4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c4c40ec6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6c6");
    pins.finish();
}
