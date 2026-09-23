//! Evidence (design § 16), observations (§ 15 and amendment 86) and the support that links an
//! assertion to one of them.
//!
//! `systems/ekr/domains/graph.yaml` declares `ekr.graph.Evidence` and `ekr.graph.Observation` as
//! flat records — a `kind`, a `locator`, a `section`, an `observation_id` — while the design gives
//! each a sum type whose variants carry different things. The crate holds the sum type, because a
//! `DatabaseRecord` with no table is not an evidence source, and projects the flat fields back out.
//! These cases hold the projection: every field the domain declares is answerable from the type
//! the crate holds.

use ekr_core::{
    AgentId, AssertionId, ContentHash, EvidenceId, ObservationId, SupportId, Timestamp,
};
use ekr_graph::{
    CanonicalRef, Confidence, Evidence, EvidenceKind, EvidenceSource, Observation,
    ObservationContent, ObservationKind, Support,
};

/// Every source kind, with the locator and section the domain would read off it.
#[test]
fn every_evidence_source_answers_the_flat_fields_the_domain_declares() {
    let assertion = AssertionId::mint();
    let observation = ObservationId::mint();

    let cases: Vec<(EvidenceSource, EvidenceKind, String, Option<&str>)> = vec![
        (
            EvidenceSource::Url("https://example.invalid/policy".to_owned()),
            EvidenceKind::Url,
            "https://example.invalid/policy".to_owned(),
            None,
        ),
        (
            EvidenceSource::Document {
                document_id: "handbook".to_owned(),
                section: Some("4.2".to_owned()),
            },
            EvidenceKind::Document,
            "handbook".to_owned(),
            Some("4.2"),
        ),
        (
            EvidenceSource::DatabaseRecord {
                database: "crm".to_owned(),
                table: "accounts".to_owned(),
                key: "17".to_owned(),
            },
            EvidenceKind::DatabaseRecord,
            "crm/accounts/17".to_owned(),
            None,
        ),
        (
            EvidenceSource::GraphAssertion(CanonicalRef::new(assertion)),
            EvidenceKind::GraphAssertion,
            assertion.to_string(),
            None,
        ),
        (
            EvidenceSource::Observation(observation),
            EvidenceKind::Observation,
            observation.to_string(),
            None,
        ),
        (
            EvidenceSource::HumanStatement {
                identity: Some("h.witness".to_owned()),
            },
            EvidenceKind::HumanStatement,
            "h.witness".to_owned(),
            None,
        ),
    ];

    assert_eq!(cases.len(), 6, "graph.yaml declares six evidence kinds");
    for (source, kind, locator, section) in cases {
        assert_eq!(source.kind(), kind);
        assert_eq!(source.locator(), locator, "{kind:?} has no locator");
        assert_eq!(source.section(), section);
        assert_eq!(
            source.observation(),
            (kind == EvidenceKind::Observation).then_some(observation),
            "only an Observation source names an observation — graph.yaml's optional link"
        );
    }
}

/// Confidence is basis points, and the domain's invariant is enforced by construction.
///
/// `graph.yaml` declares `confidence_bp >= 0` and `confidence_bp <= 10000` as invariants of
/// `ekr.graph.Evidence`. An invariant checked at commit time is a rule the type can still be built
/// in violation of; this one is checked where the value is made, so a validator never sees one.
#[test]
fn confidence_outside_its_declared_range_is_not_constructible() {
    assert_eq!(Confidence::CERTAIN.basis_points(), 10_000);
    assert_eq!(
        Confidence::from_basis_points(0).map(Confidence::basis_points),
        Some(0)
    );
    assert_eq!(
        Confidence::from_basis_points(10_000),
        Some(Confidence::CERTAIN)
    );
    assert_eq!(Confidence::from_basis_points(10_001), None);
    assert_eq!(Confidence::from_basis_points(u16::MAX), None);
}

#[test]
fn evidence_carries_its_provenance() {
    let extracted_by = AgentId::mint();
    let observed_at = Timestamp::from_millis(1_773_273_600_000);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::Url("https://example.invalid/announcement".to_owned()),
        content_hash: ContentHash::of_bytes(b"Bob became CEO on 2026-03-12"),
        extracted_by,
        observed_at,
        confidence: Confidence::CERTAIN,
    };

    assert_eq!(evidence.source.kind(), EvidenceKind::Url);
    assert_eq!(evidence.extracted_by, extracted_by);
    assert_eq!(evidence.observed_at, observed_at);
    assert_ne!(
        evidence.content_hash,
        ContentHash::of_bytes(b"something else")
    );
}

/// Every observation form the domain declares, including amendment 86's `Blob`.
#[test]
fn every_observation_form_answers_its_kind_and_its_content_hash() {
    let hash = ContentHash::of_bytes(b"what the runtime received");
    let cases: Vec<(ObservationContent, ObservationKind)> = vec![
        (
            ObservationContent::Document(hash),
            ObservationKind::Document,
        ),
        (
            ObservationContent::ApiResponse(hash),
            ObservationKind::ApiResponse,
        ),
        (
            ObservationContent::DatabaseRecord(hash),
            ObservationKind::DatabaseRecord,
        ),
        (
            ObservationContent::FeedItem(hash),
            ObservationKind::FeedItem,
        ),
        (
            ObservationContent::GraphFragment(hash),
            ObservationKind::GraphFragment,
        ),
        (
            ObservationContent::MessageBatch(hash),
            ObservationKind::MessageBatch,
        ),
        (ObservationContent::GitDiff(hash), ObservationKind::GitDiff),
        (
            ObservationContent::Blob {
                hash,
                media_type: "application/pdf".to_owned(),
                byte_len: 4096,
            },
            ObservationKind::Blob,
        ),
    ];

    assert_eq!(
        cases.len(),
        8,
        "graph.yaml declares eight observation kinds"
    );
    for (content, kind) in cases {
        assert_eq!(content.kind(), kind);
        assert_eq!(content.content_hash(), &hash);
    }
}

/// Amendment 86: a blob is bytes the runtime stores and does not parse, so the media type and the
/// length are part of what was observed rather than part of an interpretation of it.
#[test]
fn a_blob_observation_carries_its_media_type_and_length() {
    let hash = ContentHash::of_bytes(b"%PDF-1.7");
    let observation = Observation {
        id: ObservationId::mint(),
        source: "sharepoint".to_owned(),
        source_native_id: Some("01H9-board-pack".to_owned()),
        content: ObservationContent::Blob {
            hash,
            media_type: "application/pdf".to_owned(),
            byte_len: 8,
        },
        captured_at: Timestamp::EPOCH,
    };

    assert_eq!(observation.kind(), ObservationKind::Blob);
    assert_eq!(observation.content_hash(), &hash);
    assert_eq!(
        observation.source_native_id.as_deref(),
        Some("01H9-board-pack")
    );
    let ObservationContent::Blob {
        media_type,
        byte_len,
        ..
    } = &observation.content
    else {
        panic!("the fixture is a blob");
    };
    assert_eq!(media_type, "application/pdf");
    assert_eq!(*byte_len, 8);
}

/// Support is the link design § 13 writes as `evidence: BTreeSet<EvidenceId>` on the assertion and
/// `graph.yaml` declares as its own entity, so that one link can be addressed and retracted.
#[test]
fn support_links_one_assertion_to_one_piece_of_evidence() {
    let assertion_id = AssertionId::mint();
    let evidence_id = EvidenceId::mint();
    let support = Support {
        id: SupportId::mint(),
        assertion_id,
        evidence_id,
    };

    assert_eq!(support.assertion_id, assertion_id);
    assert_eq!(support.evidence_id, evidence_id);
    assert_ne!(
        Support {
            id: SupportId::mint(),
            assertion_id,
            evidence_id,
        }
        .id,
        support.id,
        "two links between the same pair are two links"
    );
}
