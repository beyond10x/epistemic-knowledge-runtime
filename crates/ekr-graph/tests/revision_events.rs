//! `RevisionEvent`: the vocabulary the kernel publishes and the store persists, and the first sum
//! type this runtime content-addresses.
//!
//! The type lives here rather than in `ekr-kernel` because both the publisher and the persister
//! sit above this crate and neither depends on the other, so one of them would otherwise own a
//! definition the other could only copy.
//!
//! Its encoding is the reason `Encoder::variant` exists. `crates/ekr-core/src/canonical.rs` rule 5:
//! a newtype is structural, but "two variants carrying the same payload shape would collide, and
//! nothing about their position distinguishes them, so a sum type carries a variant tag". The
//! index that tag carries is part of the contract — reordering the variants moves every address
//! that contains one — so the mapping is pinned below rather than left to a reviewer.

use ekr_core::canonical::Canonical;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, TransactionId};
use ekr_graph::{RevisionEvent, RevisionPayload};

/// One value of each variant, built over *the same* ids, numbers and hashes.
///
/// Sharing the payloads is the point: anything that distinguishes these six is the encoding's
/// doing and not the fixture's.
fn every_variant() -> Vec<RevisionEvent> {
    let transaction_id = TransactionId::mint();
    let revision_id = RevisionId::mint();
    let agent = AgentId::mint();
    let hash = ContentHash::of_bytes(b"one payload for all of them");
    let number = RevisionNumber::new(3);
    let event_id = EventId::mint();

    vec![
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: hash,
        },
        RevisionPayload::TransactionProposed {
            transaction_id,
            proposer: agent,
            operations_hash: Some(hash),
        },
        RevisionPayload::TransactionValidated {
            transaction_id,
            against: number,
            validation_hash: hash,
        },
        RevisionPayload::TransactionRejected {
            transaction_id,
            issues: 3,
        },
        RevisionPayload::TransactionStale {
            transaction_id,
            validated_against: number,
            current: number,
        },
        RevisionPayload::RevisionCommitted {
            transaction_id,
            revision_id,
            number,
            knowledge_root: hash,
        },
    ]
    .into_iter()
    .map(|payload| RevisionEvent {
        format: RevisionEvent::FORMAT.into(),
        event_id,
        record_hash: hash,
        payload,
    })
    .collect()
}

/// The six `ekr.kernel` event names of `systems/ekr/domains/kernel.yaml`, in the order the story's
/// scope lists them — which is the order the variant indices count in.
const NAMES: [&str; 6] = [
    "ekr.kernel.Seeded",
    "ekr.kernel.TransactionProposed",
    "ekr.kernel.TransactionValidated",
    "ekr.kernel.TransactionRejected",
    "ekr.kernel.TransactionStale",
    "ekr.kernel.RevisionCommitted",
];

#[test]
fn every_variant_carries_its_declared_index_and_domain_name() {
    let events = every_variant();
    assert_eq!(events.len(), NAMES.len(), "a variant has no fixture");

    for (position, event) in events.iter().enumerate() {
        let index = u32::try_from(position).expect("six variants fit in a u32");
        assert_eq!(
            event.variant_index(),
            index,
            "{} is not variant {position}; renumbering moves every address that contains one",
            event.name()
        );
        assert_eq!(event.name(), NAMES[position]);

        // The payload still opens with the frozen kind index. The current event envelope
        // precedes it with format, occurrence identity and retained-record address.
        let mut expected = vec![0x0fu8];
        expected.extend_from_slice(&index.to_be_bytes());
        assert_eq!(
            &event.payload.canonical_bytes()[..5],
            &expected[..],
            "{} payload does not open with its variant marker",
            event.name()
        );
    }
}

#[test]
fn no_two_variants_share_an_encoding() {
    let mut encodings: Vec<Vec<u8>> = every_variant()
        .iter()
        .map(Canonical::canonical_bytes)
        .collect();
    let count = encodings.len();
    encodings.sort();
    encodings.dedup();
    assert_eq!(
        encodings.len(),
        count,
        "two revision events share a content address"
    );
}

/// The marker, not the payload shape, is what separates two variants.
///
/// No two of the six variants happen to carry the same payload shape today — the field lists in
/// `systems/ekr/domains/kernel.yaml` differ. That is an accident of this version of the domain and
/// not a guarantee, and it is exactly the accident rule 5 refuses to rest on: `RevisionId` and
/// `TransactionId` are newtypes over the same UUID shape, so they already produce identical bytes,
/// and one field added or removed makes two variants collide.
///
/// `crates/ekr-core/tests/canonical_encoding.rs` holds the byte-identical pair directly, over a
/// two-variant type built for it. This case holds the half of it that lives in the real type: the
/// same id under two variants reaches the same seventeen payload bytes, and the encodings still
/// differ, because of the five bytes in front of them.
#[test]
fn the_variant_marker_and_not_the_payload_is_what_separates_two_events() {
    let uuid = "01a0c3a0-7889-7395-b687-b771f5ae3aa7";
    let seeded = RevisionPayload::Seeded {
        revision_id: uuid.parse().expect("the one text form of an id"),
        seed_hash: ContentHash::of_bytes(b"the same payload twice"),
    };
    let rejected = RevisionPayload::TransactionRejected {
        transaction_id: uuid.parse().expect("the one text form of an id"),
        issues: 0,
    };

    let seeded_bytes = seeded.canonical_bytes();
    let rejected_bytes = rejected.canonical_bytes();

    // The structural half of rule 5, visible: two different id types over one UUID are one
    // seventeen-byte run, tag included.
    assert_eq!(
        &seeded_bytes[5..22],
        &rejected_bytes[5..22],
        "two id newtypes over the same UUID must encode identically — rule 5"
    );
    // The tagged half: the encodings differ, and they differ in the marker.
    assert_ne!(seeded_bytes, rejected_bytes);
    assert_ne!(&seeded_bytes[..5], &rejected_bytes[..5]);

    // Remove the markers and the two shortest events are told apart by nothing but a length of
    // trailing bytes — which is the position rule 5 calls a collision.
    assert_eq!(seeded_bytes[..5].len(), rejected_bytes[..5].len());
}

/// Equal events encode equally: the property a content address rests on.
#[test]
fn an_event_encodes_as_a_function_of_its_value() {
    for event in every_variant() {
        assert_eq!(event.canonical_bytes(), event.clone().canonical_bytes());
    }

    let hash = ContentHash::of_bytes(b"seed");
    let revision_id = RevisionId::mint();
    assert_eq!(
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: hash
        }
        .canonical_bytes(),
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: hash
        }
        .canonical_bytes()
    );
    assert_ne!(
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: hash
        }
        .canonical_bytes(),
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: hash
        }
        .canonical_bytes()
    );
}

/// The current envelope has its own fixed bytes; the original family remains frozen in legacy.
#[test]
fn the_current_envelope_binds_format_occurrence_record_and_payload() {
    let event = RevisionEvent {
        format: "ekr.revision-event/2".into(),
        event_id: "00000000-0000-7000-8000-000000000001".parse().unwrap(),
        record_hash: ContentHash::from_bytes([0x21; 32]),
        payload: RevisionPayload::Seeded {
            revision_id: "00000000-0000-7000-8000-000000000002".parse().unwrap(),
            seed_hash: ContentHash::from_bytes([0x43; 32]),
        },
    };
    // Transcribed from the canonical format: String tag/length/UTF-8, UUID tag/bytes,
    // hash tag/bytes, then Seeded's variant tag/index and its UUID/hash payload.
    // No production encoder builds the expected value.
    let vector = concat!(
        "060000000000000014",
        "656b722e7265766973696f6e2d6576656e742f32",
        "0d00000000000070008000000000000001",
        "0e2121212121212121212121212121212121212121212121212121212121212121",
        "0f00000000",
        "0d00000000000070008000000000000002",
        "0e4343434343434343434343434343434343434343434343434343434343434343",
    );
    let expected: Vec<u8> = vector
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect();
    assert_eq!(event.canonical_bytes(), expected);
    assert_eq!(RevisionEvent::FORMAT, "ekr.revision-event/2");
    let address = ContentHash::of(&event);
    let mut changed = event.clone();
    changed.format = "ekr.revision-event/3".into();
    assert_ne!(ContentHash::of(&changed), address);
    changed = event.clone();
    changed.event_id = "00000000-0000-7000-8000-000000000003".parse().unwrap();
    assert_ne!(ContentHash::of(&changed), address);
    changed = event.clone();
    changed.record_hash = ContentHash::from_bytes([0x22; 32]);
    assert_ne!(ContentHash::of(&changed), address);
    changed = event;
    let RevisionPayload::Seeded { seed_hash, .. } = &mut changed.payload else {
        unreachable!("the fixed fixture is Seeded")
    };
    *seed_hash = ContentHash::from_bytes([0x44; 32]);
    assert_ne!(ContentHash::of(&changed), address);
}

#[test]
fn proposal_payload_distinguishes_absent_and_present_operations_hashes() {
    let mut event = every_variant().remove(1);
    let original = event.canonical_bytes();
    let RevisionPayload::TransactionProposed {
        operations_hash, ..
    } = &mut event.payload
    else {
        unreachable!("the second fixture is Proposed")
    };
    *operations_hash = None;
    assert_ne!(event.canonical_bytes(), original);
    assert_eq!(event.variant_index(), 1);
    assert_eq!(event.name(), "ekr.kernel.TransactionProposed");
}

/// Declaration order and `variant_index()` still agree, whatever form the variants are written in.
///
/// `every_variant_carries_its_declared_index_and_domain_name` pins the six numbers by
/// transcription, which catches a **renumbering** — the change that moves every content address
/// containing an event. It cannot see a **reorder**, because `variant_index()` is a hand-written
/// match on variant *names* and nothing derives it from a position. So the enum could grow a
/// seventh variant in the middle whose arm lands at the end, and the two halves of the type would
/// describe different orderings with the whole suite green.
///
/// Neither half is derivable from the other in Rust without a macro this crate has no dependency
/// for, so the agreement is checked instead: `src/events.rs` is read as text, the way
/// `tests/domain_projection.rs` reads the ESS domain, and three lists are compared — the variants
/// as declared, the arms of `variant_index`, and the fixtures `every_variant` builds.
///
/// # Three forms, not one
///
/// The first version of this scan stripped `" {"`, so it saw **struct variants only**, and guarded
/// its own vacuity against the constant `6`. A seventh variant written as `Explained(AssertionId)`
/// or as a bare `Sealed` was invisible to it *and* to the transcribed case, because both were
/// counted against the same constant. All three forms are read now, and the vacuity guard is the
/// agreement of the three derived lists rather than a number written here: if the scan stops
/// finding variants, the lists stop matching the fixtures.
#[test]
fn the_declaration_order_of_the_variants_equals_their_numbering() {
    let source = std::fs::read_to_string(
        std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR")
                .expect("Cargo supplies the runtime manifest directory"),
        )
        .join("src/events.rs"),
    )
    .expect("the crate's own source");

    let declaration = source
        .split_once("pub enum RevisionPayload {")
        .expect("the crate declares the six-kind RevisionPayload")
        .1;

    // A variant head is a line at exactly one level of indentation inside the enum. Three forms:
    // `Name {` (struct), `Name(` (tuple) and `Name,` (unit). `rustfmt` runs in the gate, so the
    // indentation and the one-head-per-line shape are not assumptions about style.
    let declared: Vec<String> = declaration
        .lines()
        .take_while(|line| *line != "}")
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .filter_map(|line| {
            let head = line.trim();
            let name = head
                .strip_suffix(" {")
                .or_else(|| head.strip_suffix('('))
                .or_else(|| head.strip_suffix(','))?;
            (!name.is_empty()
                && name.starts_with(char::is_uppercase)
                && name.chars().all(char::is_alphanumeric))
            .then(|| name.to_owned())
        })
        .collect();

    // The arms of `variant_index`, which is the other half that can move independently.
    let index_body = source
        .split_once("impl RevisionPayload {")
        .expect("the payload owns its variant numbering")
        .1
        .split_once("pub const fn variant_index(&self) -> u32 {")
        .expect("the crate declares variant_index")
        .1
        .split_once("\n    }")
        .expect("the function closes")
        .0;
    let mut arms: Vec<(u32, String)> = index_body
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Self::"))
        .filter_map(|arm| {
            let (head, number) = arm.split_once("=> ")?;
            let name: String = head.chars().take_while(|c| c.is_alphanumeric()).collect();
            let number: u32 = number.trim().trim_end_matches(',').parse().ok()?;
            Some((number, name))
        })
        .collect();
    arms.sort();

    assert_eq!(
        declared.len(),
        arms.len(),
        "the enum declares {declared:?} and variant_index answers for {arms:?}: a variant has no \
         arm, or an arm has no variant. One of the two forms the scan reads may also have been \
         missed."
    );
    assert_eq!(
        declared,
        arms.iter()
            .map(|(_, name)| name.clone())
            .collect::<Vec<_>>(),
        "the order RevisionPayload's variants are written in is not the order variant_index() \
         counts in. Nothing derives one from the other: variant_index() is a hand-written match on \
         names. Whichever is wrong, changing a *number* moves every content address that contains \
         that variant — changing the declaration order moves nothing."
    );
    assert_eq!(
        arms.iter().map(|(number, _)| *number).collect::<Vec<_>>(),
        (0..u32::try_from(arms.len()).expect("a small enum")).collect::<Vec<_>>(),
        "the variant numbers are not 0..n without gaps or repeats: {arms:?}"
    );

    // And the fixtures cover every declared variant, so a new one cannot arrive with the two
    // scans agreeing and nothing exercising it.
    let mut fixtures: Vec<String> = every_variant()
        .iter()
        .map(|event| {
            event
                .name()
                .rsplit('.')
                .next()
                .expect("a qualified event name has a last segment")
                .to_owned()
        })
        .collect();
    fixtures.sort();
    let mut expected = declared.clone();
    expected.sort();
    assert_eq!(
        fixtures, expected,
        "every_variant() does not build one of each declared variant, so the transcribed case \
         above is measuring a subset of the type"
    );
    assert_eq!(
        declared.len(),
        NAMES.len(),
        "NAMES is the transcription the other case reads; it and the type have come apart"
    );
}
