//! Real Ed25519 signatures over generated contracts; keys are deterministic test-only seeds.
use ekr_core::contracts::{integrate, kernel as m, primitives::Uuid};
use ekr_kernel::human_review::{self as review, Reviewer, VerifiedDecision};
use ring::signature::{Ed25519KeyPair, KeyPair};

fn hash(bytes: &[u8]) -> m::ContentHash {
    m::ContentHash(review::digest(bytes).to_string())
}

#[test]
fn generated_json_review_documents_preserve_all_signed_targets_and_reject_bad_scalars() {
    use ekr_core::contract_data::{EkrKernelReviewerTrustPolicy, EkrKernelSignedHumanDecision};
    let mut f = Fixture::new();
    f.policy.keys[0].scopes = vec![
        m::HumanDecisionScope::AnswerAttention,
        m::HumanDecisionScope::ApproveSchemaProposal,
        m::HumanDecisionScope::RejectSchemaProposal,
        m::HumanDecisionScope::UpgradeAuthority,
    ];
    let audience = serde_json::json!({"tenant": f.intent.audience.tenant, "seed_anchor": f.intent.audience.seed_anchor.0});
    let mut policy_json = serde_json::json!({
        "format": "ekr.reviewer-trust/1", "audience": audience,
        "keys": [{"key_digest": f.policy.keys[0].key_digest.0, "algorithm": "Ed25519",
            "public_key": ekr_core::bytes::encode(f.key.public_key().as_ref()),
            "operator": {"actor": f.policy.keys[0].operator.actor.0.0, "authentication_subject": "fixture-human"},
            "scopes": ["AnswerAttention", "ApproveSchemaProposal", "RejectSchemaProposal", "UpgradeAuthority"]}]
    });
    let wire: EkrKernelReviewerTrustPolicy = serde_json::from_value(policy_json.clone()).unwrap();
    assert_eq!(review::policy_from_document(&wire).unwrap(), f.policy);
    policy_json["keys"][0]["public_key"] = "invalid-base64".into();
    let wire = serde_json::from_value(policy_json).unwrap();
    assert!(review::policy_from_document(&wire).is_err());
    let basis = m::ReviewBasis {
        observed_revision: m::RevisionNumber(17),
        evidence_digest: hash(b"evidence"),
        options_digest: hash(b"options"),
        effects_digest: hash(b"effects"),
    };
    let basis_json = serde_json::json!({"observed_revision": 17, "evidence_digest": basis.evidence_digest.0,
        "options_digest": basis.options_digest.0, "effects_digest": basis.effects_digest.0});
    let schema = m::SchemaReviewTarget {
        proposal_id: integrate::SchemaProposalId(id()),
        proposal_digest: hash(b"proposal"),
        basis: basis.clone(),
    };
    let schema_json = serde_json::json!({"proposal_id": id().0, "proposal_digest": schema.proposal_digest.0, "basis": basis_json});
    let cases = [
        (
            m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
                dispute_id: m::DisputeId(id()),
                basis,
                corrections_digest: hash(b"corrections"),
            }),
            serde_json::json!({"kind": "AnswerAttention", "value": {"dispute_id": id().0, "basis": basis_json,
                "corrections_digest": hash(b"corrections").0}}),
        ),
        (
            m::HumanDecisionTarget::ApproveSchemaProposal(schema.clone()),
            serde_json::json!({"kind": "ApproveSchemaProposal", "value": schema_json}),
        ),
        (
            m::HumanDecisionTarget::RejectSchemaProposal(schema),
            serde_json::json!({"kind": "RejectSchemaProposal", "value": schema_json}),
        ),
        (
            f.intent.target.clone(),
            serde_json::json!({"kind": "UpgradeAuthority", "value": {
            "preview_digest": hash(b"preview").0, "reviewer_policy_digest": f.intent.reviewer_policy_digest.0}}),
        ),
    ];
    for (target, target_json) in cases {
        f.intent.target = target;
        f.intent.expected_previous_decision = Some(hash(b"previous"));
        let expected = f.proof();
        let mut json = serde_json::json!({"algorithm": "Ed25519", "signature": ekr_core::bytes::encode(&expected.signature),
            "intent": {"format": "ekr.human-decision/1", "decision_id": id().0, "audience": audience,
                "reviewer_policy_digest": f.intent.reviewer_policy_digest.0, "signer_key_digest": f.intent.signer_key_digest.0,
                "target": target_json, "statement_digest": f.intent.statement_digest.0,
                "expected_previous_decision": hash(b"previous").0}});
        let wire: EkrKernelSignedHumanDecision = serde_json::from_value(json.clone()).unwrap();
        let actual = review::proof_from_document(&wire).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            review::proof_bytes(&actual).unwrap(),
            review::proof_bytes(&expected).unwrap()
        );
        json["intent"]["decision_id"] = "not-a-uuid".into();
        let wire = serde_json::from_value(json).unwrap();
        assert!(review::proof_from_document(&wire).is_err());
    }
}
fn id() -> Uuid {
    Uuid("00000000-0000-0000-0000-000000000001".into())
}
struct Fixture {
    key: Ed25519KeyPair,
    policy: m::ReviewerTrustPolicy,
    binding: m::TrustedReviewHostBinding,
    intent: m::HumanDecisionIntent,
}
impl Fixture {
    fn new() -> Self {
        let key = Ed25519KeyPair::from_seed_unchecked(&[7; 32]).unwrap();
        let audience = m::HumanDecisionAudience {
            tenant: "fixture".into(),
            seed_anchor: hash(b"seed"),
        };
        let key_digest = hash(key.public_key().as_ref());
        let policy = m::ReviewerTrustPolicy {
            format: m::ReviewerTrustFormat::ReviewerTrust1,
            audience: audience.clone(),
            keys: vec![m::ReviewerVerificationKey {
                key_digest: key_digest.clone(),
                algorithm: m::ReviewSignatureAlgorithm::Ed25519,
                public_key: key.public_key().as_ref().to_vec(),
                operator: m::TrustedOperatorIdentity {
                    actor: m::AgentId(id()),
                    authentication_subject: "fixture-human".into(),
                },
                scopes: vec![m::HumanDecisionScope::UpgradeAuthority],
            }],
        };
        let policy_digest = hash(&review::policy_bytes(&policy).unwrap());
        let binding = m::TrustedReviewHostBinding {
            audience: audience.clone(),
            reviewer_policy_digest: policy_digest.clone(),
        };
        let intent = m::HumanDecisionIntent {
            format: m::HumanDecisionFormat::HumanDecision1,
            decision_id: id(),
            audience,
            reviewer_policy_digest: policy_digest.clone(),
            signer_key_digest: key_digest,
            target: m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
                preview_digest: hash(b"preview"),
                reviewer_policy_digest: policy_digest,
            }),
            statement_digest: hash(b"reviewed"),
            expected_previous_decision: None,
        };
        Self {
            key,
            policy,
            binding,
            intent,
        }
    }
    fn proof(&self) -> m::SignedHumanDecision {
        m::SignedHumanDecision {
            intent: self.intent.clone(),
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            signature: self
                .key
                .sign(&review::signing_bytes(&self.intent).unwrap())
                .as_ref()
                .to_vec(),
        }
    }
    fn reviewer(&self) -> Reviewer {
        Reviewer::from_host(&self.binding, self.policy.clone()).unwrap()
    }
}

#[test]
fn verified_operator_comes_from_policy_and_statement_and_target_are_bound() {
    let f = Fixture::new();
    let proof = f.proof();
    let reviewer = f.reviewer();
    assert_eq!(
        reviewer
            .verify(&proof, &f.intent.target, b"reviewed", None)
            .unwrap()
            .operator(),
        &f.policy.keys[0].operator
    );
    assert!(reviewer
        .verify(&proof, &f.intent.target, b"changed", None)
        .is_err());
    let mut target = f.intent.target.clone();
    let m::HumanDecisionTarget::UpgradeAuthority(ref mut upgrade) = target else {
        panic!()
    };
    upgrade.preview_digest = hash(b"changed");
    assert!(reviewer.verify(&proof, &target, b"reviewed", None).is_err());
}

#[test]
fn independent_binding_refuses_substituted_policy_and_store() {
    let f = Fixture::new();
    let mut policy = f.policy.clone();
    policy.keys[0].operator.authentication_subject = "agent-asserted-human".into();
    assert!(Reviewer::from_host(&f.binding, policy).is_err());
    let mut binding = f.binding.clone();
    binding.audience.tenant = "other-store".into();
    assert!(Reviewer::from_host(&binding, f.policy.clone()).is_err());
    binding = f.binding.clone();
    binding.audience.seed_anchor = hash(b"other-seed");
    assert!(Reviewer::from_host(&binding, f.policy.clone()).is_err());
}

#[test]
fn forged_truncated_and_altered_signatures_refuse() {
    let f = Fixture::new();
    let reviewer = f.reviewer();
    let proof = f.proof();
    let mut candidates = vec![proof.clone(); 4];
    candidates[0].signature[0] ^= 1;
    candidates[1].signature.pop();
    candidates[2].signature.push(0);
    let other = Ed25519KeyPair::from_seed_unchecked(&[8; 32]).unwrap();
    candidates[3].signature = other
        .sign(&review::signing_bytes(&f.intent).unwrap())
        .as_ref()
        .to_vec();
    for candidate in candidates {
        assert!(reviewer
            .verify(&candidate, &f.intent.target, b"reviewed", None)
            .is_err());
    }
}

#[test]
fn even_valid_signatures_cannot_cross_audience_key_policy_or_predecessor() {
    let mut f = Fixture::new();
    let reviewer = f.reviewer();
    let original = f.intent.clone();
    for mutation in 0..6 {
        f.intent = original.clone();
        match mutation {
            0 => f.intent.audience.tenant = "other-store".into(),
            1 => f.intent.audience.seed_anchor = hash(b"other-seed"),
            2 => f.intent.signer_key_digest = hash(b"unknown-key"),
            3 => f.intent.reviewer_policy_digest = hash(b"other-policy"),
            4 => f.intent.expected_previous_decision = Some(hash(b"old-decision")),
            _ => {
                let m::HumanDecisionTarget::UpgradeAuthority(ref mut target) = f.intent.target
                else {
                    panic!()
                };
                target.reviewer_policy_digest = hash(b"other-policy");
            }
        }
        assert!(
            reviewer
                .verify(&f.proof(), &f.intent.target, b"reviewed", None)
                .is_err(),
            "mutation {mutation}"
        );
    }
    f.intent = original;
    assert!(reviewer
        .verify(
            &f.proof(),
            &f.intent.target,
            b"reviewed",
            Some(review::digest(b"latest"))
        )
        .is_err());
    f.intent.expected_previous_decision = Some(hash(b"latest"));
    assert!(reviewer
        .verify(
            &f.proof(),
            &f.intent.target,
            b"reviewed",
            Some(review::digest(b"latest"))
        )
        .is_ok());
}

#[test]
fn policy_refuses_duplicate_keys_scopes_wrong_key_digest_and_noncanonical_order() {
    let f = Fixture::new();
    for mutation in 0..5 {
        let mut p = f.policy.clone();
        match mutation {
            0 => p.keys.push(p.keys[0].clone()),
            1 => p.keys[0]
                .scopes
                .push(m::HumanDecisionScope::UpgradeAuthority),
            2 => p.keys[0].key_digest = hash(b"wrong"),
            3 => p.keys[0].public_key.pop().map(|_| ()).unwrap(),
            _ => p.keys[0]
                .scopes
                .push(m::HumanDecisionScope::AnswerAttention),
        }
        assert!(review::policy_bytes(&p).is_err(), "mutation {mutation}");
    }
}

#[test]
fn signing_codec_has_explicit_domain_and_ess_field_order_and_refuses_invalid_scalars() {
    let f = Fixture::new();
    let message = review::signing_bytes(&f.intent).unwrap();
    let mut prefix = b"ekr.human-decision/1\0".to_vec();
    prefix.extend_from_slice(&20u64.to_be_bytes());
    prefix.extend_from_slice(b"ekr.human-decision/1");
    prefix.extend_from_slice(&1u128.to_be_bytes());
    assert!(message.starts_with(&prefix));
    let mut intent = f.intent;
    intent.decision_id = Uuid("not-a-uuid".into());
    assert!(review::signing_bytes(&intent).is_err());
    intent.decision_id = id();
    intent.statement_digest = m::ContentHash("AB".repeat(32));
    assert!(review::signing_bytes(&intent).is_err());
}

#[test]
fn all_four_targets_verify_only_in_their_own_enrolled_scope() {
    let mut f = Fixture::new();
    let basis = m::ReviewBasis {
        observed_revision: m::RevisionNumber(7),
        evidence_digest: hash(b"evidence"),
        options_digest: hash(b"options"),
        effects_digest: hash(b"effects"),
    };
    let schema = m::SchemaReviewTarget {
        proposal_id: integrate::SchemaProposalId(id()),
        proposal_digest: hash(b"proposal"),
        basis: basis.clone(),
    };
    let targets = [
        (
            m::HumanDecisionScope::AnswerAttention,
            m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
                dispute_id: m::DisputeId(id()),
                basis,
                corrections_digest: hash(b"corrections"),
            }),
        ),
        (
            m::HumanDecisionScope::ApproveSchemaProposal,
            m::HumanDecisionTarget::ApproveSchemaProposal(schema.clone()),
        ),
        (
            m::HumanDecisionScope::RejectSchemaProposal,
            m::HumanDecisionTarget::RejectSchemaProposal(schema),
        ),
        (
            m::HumanDecisionScope::UpgradeAuthority,
            f.intent.target.clone(),
        ),
    ];
    for (scope, target) in &targets {
        f.policy.keys[0].scopes = vec![*scope];
        let policy_digest = hash(&review::policy_bytes(&f.policy).unwrap());
        f.binding.reviewer_policy_digest = policy_digest.clone();
        f.intent.reviewer_policy_digest = policy_digest.clone();
        f.intent.target = target.clone();
        if let m::HumanDecisionTarget::UpgradeAuthority(target) = &mut f.intent.target {
            target.reviewer_policy_digest = policy_digest.clone();
        }
        let reviewer = f.reviewer();
        let proof = f.proof();
        assert_eq!(
            review::read_proof(&review::proof_bytes(&proof).unwrap()).unwrap(),
            proof
        );
        assert!(reviewer
            .verify(&proof, &f.intent.target, b"reviewed", None)
            .is_ok());
        for (other_scope, other_target) in &targets {
            if scope == other_scope {
                continue;
            }
            let mut other_intent = f.intent.clone();
            other_intent.target = other_target.clone();
            if let m::HumanDecisionTarget::UpgradeAuthority(target) = &mut other_intent.target {
                target.reviewer_policy_digest = policy_digest.clone();
            }
            // Both cross-operation replay and a fresh signature with an unauthorized scope fail.
            assert!(reviewer
                .verify(&proof, &other_intent.target, b"reviewed", None)
                .is_err());
            let other_proof = m::SignedHumanDecision {
                signature: f
                    .key
                    .sign(&review::signing_bytes(&other_intent).unwrap())
                    .as_ref()
                    .to_vec(),
                intent: other_intent,
                algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            };
            let error = reviewer
                .verify(&other_proof, &other_proof.intent.target, b"reviewed", None)
                .err()
                .unwrap();
            assert_eq!(error.code, "review-scope");
        }
    }
}

#[test]
fn retained_proof_and_every_review_basis_field_are_bound() {
    let mut f = Fixture::new();
    f.policy.keys[0].scopes = vec![m::HumanDecisionScope::AnswerAttention];
    let policy_digest = hash(&review::policy_bytes(&f.policy).unwrap());
    f.binding.reviewer_policy_digest = policy_digest.clone();
    f.intent.reviewer_policy_digest = policy_digest;
    f.intent.target = m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
        dispute_id: m::DisputeId(id()),
        corrections_digest: hash(b"corrections"),
        basis: m::ReviewBasis {
            observed_revision: m::RevisionNumber(7),
            evidence_digest: hash(b"evidence"),
            options_digest: hash(b"options"),
            effects_digest: hash(b"effects"),
        },
    });
    let reviewer = f.reviewer();
    let proof = f.proof();
    let verified: VerifiedDecision = reviewer
        .verify(&proof, &proof.intent.target, b"reviewed", None)
        .unwrap();
    assert_eq!(verified.intent(), &proof.intent);
    assert_eq!(
        verified.proof_digest(),
        review::digest(verified.canonical_proof())
    );
    for mutation in 0..6 {
        let mut altered = proof.clone();
        let m::HumanDecisionTarget::AnswerAttention(target) = &mut altered.intent.target else {
            panic!()
        };
        match mutation {
            0 => target.basis.observed_revision.0 += 1,
            1 => target.basis.evidence_digest = hash(b"changed"),
            2 => target.basis.options_digest = hash(b"changed"),
            3 => target.basis.effects_digest = hash(b"changed"),
            4 => target.corrections_digest = hash(b"changed"),
            _ => target.dispute_id.0 = Uuid("00000000-0000-0000-0000-000000000002".into()),
        }
        assert!(reviewer
            .verify(&altered, &altered.intent.target, b"reviewed", None)
            .is_err());
    }
    let mut invalid = f.intent;
    let m::HumanDecisionTarget::AnswerAttention(target) = &mut invalid.target else {
        panic!()
    };
    target.basis.observed_revision.0 = -1;
    assert!(review::signing_bytes(&invalid).is_err());
}

#[test]
fn policy_key_order_is_checked_and_an_empty_policy_never_authorizes() {
    let f = Fixture::new();
    let mut policy = f.policy.clone();
    let other = Ed25519KeyPair::from_seed_unchecked(&[8; 32]).unwrap();
    let mut key = policy.keys[0].clone();
    key.public_key = other.public_key().as_ref().to_vec();
    key.key_digest = hash(&key.public_key);
    policy.keys.push(key);
    policy
        .keys
        .sort_by(|a, b| a.key_digest.0.cmp(&b.key_digest.0));
    assert!(review::policy_bytes(&policy).is_ok());
    policy.keys.reverse();
    assert!(review::policy_bytes(&policy).is_err());
    policy.keys.clear();
    let mut binding = f.binding.clone();
    binding.reviewer_policy_digest = hash(&review::policy_bytes(&policy).unwrap());
    let reviewer = Reviewer::from_host(&binding, policy).unwrap();
    let mut intent = f.intent.clone();
    intent.reviewer_policy_digest = binding.reviewer_policy_digest.clone();
    let m::HumanDecisionTarget::UpgradeAuthority(target) = &mut intent.target else {
        panic!()
    };
    target.reviewer_policy_digest = binding.reviewer_policy_digest;
    let proof = m::SignedHumanDecision {
        signature: f
            .key
            .sign(&review::signing_bytes(&intent).unwrap())
            .as_ref()
            .to_vec(),
        intent,
        algorithm: m::ReviewSignatureAlgorithm::Ed25519,
    };
    let error = reviewer
        .verify(&proof, &proof.intent.target, b"reviewed", None)
        .err()
        .unwrap();
    assert_eq!(error.code, "review-untrusted-key");
}

#[test]
fn protocol_digests_and_object_addresses_resolve_the_same_bytes_after_both_provider_reopens() {
    use ekr_core::contracts::primitives;
    use ekr_core::{ContentHash, SchemaVersionId, Timestamp};
    use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
    use ekr_store::{FileStore, ObjectStore, SqliteStore, StorageClass};
    let f = Fixture::new();
    let proof = f.proof();
    let verified = f
        .reviewer()
        .verify(&proof, &proof.intent.target, b"reviewed", None)
        .unwrap();
    let record = verified
        .record(primitives::Timestamp("1970-01-01T00:00:00Z".into()))
        .unwrap();
    assert!(verified
        .record(primitives::Timestamp("not-a-time".into()))
        .is_err());
    assert!(verified
        .record(primitives::Timestamp("1970-01-01T00:00:00.000001Z".into()))
        .is_err());
    let objects = [
        (
            &record.proof_object_hash,
            &record.proof_digest,
            verified.canonical_proof(),
        ),
        (
            &record.policy_object_hash,
            &record.policy_digest,
            verified.canonical_policy(),
        ),
        (
            &record.statement_object_hash,
            &record.statement_digest,
            verified.statement(),
        ),
    ];
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let open = || -> Box<dyn ObjectStore> {
            let ontology = Ontology::load(OntologyDocument {
                version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
                node_types: vec![],
                edge_types: vec![],
            })
            .unwrap();
            if sqlite {
                Box::new(
                    SqliteStore::sqlite(
                        &directory.path().join("state.db"),
                        "review-fixture",
                        ontology,
                    )
                    .unwrap(),
                )
            } else {
                Box::new(FileStore::file(directory.path(), "review-fixture", ontology).unwrap())
            }
        };
        {
            let store = open();
            for (address, protocol, bytes) in objects {
                assert_ne!(
                    address, protocol,
                    "storage and review protocols occupy different hash domains"
                );
                let stored = store
                    .put(StorageClass::Provenance, bytes, Timestamp::EPOCH)
                    .unwrap();
                assert_eq!(stored.content_hash.to_string(), address.0);
            }
        }
        let store = open();
        for (address, protocol, bytes) in objects {
            let retained = store
                .get(&address.0.parse::<ContentHash>().unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(retained, bytes);
            assert_eq!(review::digest(&retained).to_string(), protocol.0);
        }
        // A fresh verifier uses only retained public bytes plus the independent host binding.
        let read = |hash: &m::ContentHash| {
            store
                .get(&hash.0.parse::<ContentHash>().unwrap())
                .unwrap()
                .unwrap()
        };
        let retained_policy = review::read_policy(&read(&record.policy_object_hash)).unwrap();
        let retained_proof = review::read_proof(&read(&record.proof_object_hash)).unwrap();
        let replayed = Reviewer::from_host(&f.binding, retained_policy)
            .unwrap()
            .verify(
                &retained_proof,
                &f.intent.target,
                &read(&record.statement_object_hash),
                None,
            )
            .unwrap();
        assert_eq!(replayed.proof_digest(), verified.proof_digest());
        assert_eq!(replayed.operator(), verified.operator());
    }
}

#[test]
fn signing_message_matches_the_fixed_binary_layout_vector() {
    let h = |byte: u8| m::ContentHash(format!("{byte:02x}").repeat(32));
    let intent = m::HumanDecisionIntent {
        format: m::HumanDecisionFormat::HumanDecision1,
        decision_id: id(),
        audience: m::HumanDecisionAudience {
            tenant: "fixture".into(),
            seed_anchor: h(1),
        },
        reviewer_policy_digest: h(2),
        signer_key_digest: h(3),
        target: m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
            preview_digest: h(4),
            reviewer_policy_digest: h(2),
        }),
        statement_digest: h(5),
        expected_previous_decision: Some(h(6)),
    };
    // Hand-specified §105.6 vector: domain, format, UUID, audience, policy, key,
    // discriminant + upgrade payload, statement, present predecessor. No serde or codec helper.
    let expected: &[&[u8]] = &[
        b"ekr.human-decision/1\0",
        b"\0\0\0\0\0\0\0\x14",
        b"ekr.human-decision/1",
        b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x01",
        b"\0\0\0\0\0\0\0\x07",
        b"fixture",
        &[1; 32],
        &[2; 32],
        &[3; 32],
        b"\0\0\0\0\0\0\0\x10",
        b"UpgradeAuthority",
        &[4; 32],
        &[2; 32],
        &[5; 32],
        b"\x01",
        &[6; 32],
    ];
    assert_eq!(review::signing_bytes(&intent).unwrap(), expected.concat());
}

#[test]
fn canonical_readers_refuse_every_truncation_and_trailing_byte() {
    let f = Fixture::new();
    let proof = review::proof_bytes(&f.proof()).unwrap();
    let policy = review::policy_bytes(&f.policy).unwrap();
    let binding = review::host_binding_bytes(&f.binding).unwrap();
    assert_eq!(review::read_policy(&policy).unwrap(), f.policy);
    assert_eq!(review::read_host_binding(&binding).unwrap(), f.binding);
    for length in 0..proof.len() {
        assert!(
            review::read_proof(&proof[..length]).is_err(),
            "proof prefix {length}"
        );
    }
    for length in 0..policy.len() {
        assert!(
            review::read_policy(&policy[..length]).is_err(),
            "policy prefix {length}"
        );
    }
    for length in 0..binding.len() {
        assert!(
            review::read_host_binding(&binding[..length]).is_err(),
            "binding prefix {length}"
        );
    }
    let extra = |mut bytes: Vec<u8>| {
        bytes.push(0);
        bytes
    };
    assert!(review::read_proof(&extra(proof)).is_err());
    assert!(review::read_policy(&extra(policy)).is_err());
    assert!(review::read_host_binding(&extra(binding)).is_err());
    assert!(review::read_proof(&[255; 8]).is_err());
    assert!(review::read_policy(&[255; 8]).is_err());
    assert!(review::read_host_binding(&[255; 8]).is_err());
}

#[test]
fn no_single_byte_change_in_retained_proof_can_replay_as_approved() {
    let f = Fixture::new();
    let bytes = review::proof_bytes(&f.proof()).unwrap();
    let reviewer = f.reviewer();
    for index in 0..bytes.len() {
        let mut altered = bytes.clone();
        altered[index] ^= 1;
        if let Ok(proof) = review::read_proof(&altered) {
            assert_eq!(
                review::proof_bytes(&proof).unwrap(),
                altered,
                "canonical reader preserves byte {index}"
            );
            assert!(
                reviewer
                    .verify(&proof, &proof.intent.target, b"reviewed", None)
                    .is_err(),
                "changed byte {index}"
            );
        }
    }
}

fn answer_fixture() -> (Fixture, m::AttentionItem, Vec<m::ClaimCorrection>) {
    let mut f = Fixture::new();
    f.policy.keys[0].scopes = vec![m::HumanDecisionScope::AnswerAttention];
    let policy_digest = hash(&review::policy_bytes(&f.policy).unwrap());
    f.binding.reviewer_policy_digest = policy_digest.clone();
    f.intent.reviewer_policy_digest = policy_digest;
    let basis = m::ReviewBasis {
        observed_revision: m::RevisionNumber(17),
        evidence_digest: hash(b"original retained evidence"),
        options_digest: hash(b"original competing claims"),
        effects_digest: hash(b"original correction effects"),
    };
    let corrections = vec![m::ClaimCorrection {
        kind: m::ClaimCorrectionKind::Choose,
        assertion_id: ekr_core::contracts::graph::AssertionId(id()),
        valid_from: None,
        valid_to: None,
        reason: "supported by retained evidence".into(),
    }];
    f.intent.target = m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
        dispute_id: m::DisputeId(id()),
        basis: basis.clone(),
        corrections_digest: hash(&review::corrections_bytes(&corrections).unwrap()),
    });
    let question = m::AttentionItem {
        subject: m::AttentionSubject {
            kind: m::AttentionKind::Dispute,
            dispute_id: Some(m::DisputeId(id())),
            blocker_id: None,
            proposal_id: None,
        },
        question: "Which claim is supported?".into(),
        basis,
        claims: vec![corrections[0].assertion_id.clone()],
        evidence: vec![],
        observations: vec![],
    };
    (f, question, corrections)
}

#[test]
fn exact_human_answer_survives_only_unrelated_basis_advancement() {
    let (f, question, corrections) = answer_fixture();
    let proof = f.proof();
    let reviewer = f.reviewer();
    assert!(reviewer
        .verify_attention(&proof, &question, &corrections, b"reviewed", None)
        .is_ok());
    let mut advanced = question.clone();
    advanced.basis.observed_revision.0 += 10;
    // Presentation changes are not changes in retained claims, evidence or intended effects.
    advanced.question = "A reformatted question".into();
    let verified = reviewer
        .verify_attention(&proof, &advanced, &corrections, b"reviewed", None)
        .unwrap();
    assert_eq!(
        verified.canonical_proof(),
        review::proof_bytes(&proof).unwrap()
    );
    assert_eq!(verified.operator(), &f.policy.keys[0].operator);
    for changed in 0..4 {
        let mut current = advanced.clone();
        match changed {
            0 => current.basis.evidence_digest = hash(b"new evidence"),
            1 => current.basis.options_digest = hash(b"new claim"),
            2 => current.basis.effects_digest = hash(b"new declaration"),
            3 => current.basis.observed_revision = m::RevisionNumber(16),
            _ => unreachable!(),
        }
        assert_eq!(
            reviewer
                .verify_attention(&proof, &current, &corrections, b"reviewed", None)
                .err()
                .unwrap()
                .code,
            "answer-review-required"
        );
    }
}

#[test]
fn human_answer_cannot_change_its_corrections_subject_scope_statement_or_predecessor() {
    let (mut f, question, corrections) = answer_fixture();
    let reviewer = f.reviewer();
    let proof = f.proof();
    let mut changed = corrections.clone();
    changed[0].kind = m::ClaimCorrectionKind::Retract;
    assert!(reviewer
        .verify_attention(&proof, &question, &changed, b"reviewed", None)
        .is_err());
    assert!(reviewer
        .verify_attention(
            &proof,
            &question,
            &corrections,
            b"different statement",
            None
        )
        .is_err());
    assert!(reviewer
        .verify_attention(
            &proof,
            &question,
            &corrections,
            b"reviewed",
            Some(review::digest(b"prior"))
        )
        .is_err());
    for mode in 0..4 {
        let mut other = question.clone();
        match mode {
            0 => {
                other.subject.dispute_id = Some(m::DisputeId(Uuid(
                    "00000000-0000-0000-0000-000000000002".into(),
                )))
            }
            1 => other.subject.kind = m::AttentionKind::BlockedIntegration,
            2 => other.subject.blocker_id = Some(integrate::IntegrationBlockerId(id())),
            3 => other.subject.proposal_id = Some(integrate::SchemaProposalId(id())),
            _ => unreachable!(),
        }
        assert!(reviewer
            .verify_attention(&proof, &other, &corrections, b"reviewed", None)
            .is_err());
    }
    let mut forged = proof.clone();
    forged.signature[0] ^= 1;
    assert_eq!(
        reviewer
            .verify_attention(&forged, &question, &corrections, b"reviewed", None)
            .err()
            .unwrap()
            .code,
        "review-signature"
    );
    f.intent.target = m::HumanDecisionTarget::ApproveSchemaProposal(m::SchemaReviewTarget {
        proposal_id: integrate::SchemaProposalId(id()),
        proposal_digest: hash(b"proposal"),
        basis: question.basis.clone(),
    });
    assert!(reviewer
        .verify_attention(&f.proof(), &question, &corrections, b"reviewed", None)
        .is_err());
}

#[test]
fn correction_codec_preserves_order_every_field_and_millisecond_instants() {
    use ekr_core::contracts::primitives::Timestamp;
    let input = m::ClaimCorrection {
        kind: m::ClaimCorrectionKind::CorrectTime,
        assertion_id: ekr_core::contracts::graph::AssertionId(id()),
        valid_from: Some(Timestamp("1969-12-31T23:59:59.999Z".into())),
        valid_to: Some(Timestamp("1970-01-01T00:00:00.001Z".into())),
        reason: "corrected".into(),
    };
    let bytes = review::corrections_bytes(std::slice::from_ref(&input)).unwrap();
    assert_eq!(bytes.iter().fold(String::new(), |mut text, byte| {
        use std::fmt::Write;
        write!(text, "{byte:02x}").unwrap();
        text
    }), "0000000000000001000000000000000b436f727265637454696d650000000000000000000000000000000101ffffffffffffffff0100000000000000010000000000000009636f72726563746564");
    assert_eq!(review::corrections_bytes(&[]).unwrap(), 0_u64.to_be_bytes());
    for field in 0..5 {
        let mut changed = input.clone();
        match field {
            0 => changed.kind = m::ClaimCorrectionKind::Choose,
            1 => changed.assertion_id.0 .0 = "00000000-0000-0000-0000-000000000002".into(),
            2 => changed.valid_from = None,
            3 => changed.valid_to = None,
            4 => changed.reason.push('!'),
            _ => unreachable!(),
        }
        assert_ne!(
            review::corrections_bytes(&[changed.clone()]).unwrap(),
            bytes
        );
        assert_ne!(
            review::corrections_bytes(&[input.clone(), changed.clone()]).unwrap(),
            review::corrections_bytes(&[changed, input.clone()]).unwrap()
        );
    }
    for bad in ["not-time", "1970-01-01T00:00:00.0001Z"] {
        let mut changed = input.clone();
        changed.valid_to = Some(Timestamp(bad.into()));
        assert!(review::corrections_bytes(&[changed]).is_err());
    }
    let mut offset = input.clone();
    offset.valid_to = Some(Timestamp("1970-01-01T01:00:00.001+01:00".into()));
    assert_eq!(review::corrections_bytes(&[offset]).unwrap(), bytes);
    let mut invalid_id = input;
    invalid_id.assertion_id.0 .0 = "invalid".into();
    assert!(review::corrections_bytes(&[invalid_id]).is_err());
}
