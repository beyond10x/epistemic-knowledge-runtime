//! Adversary pass 2, wave 2026-10-21b, unit U1: the SDK's `TransactionDocument::from_yaml(text,
//! profile)`, new in this unit, whose contract reads "Reads `text` as `ekr propose` reads it in a
//! store under `profile`" (`crates/ekr-sdk/src/document/transaction.rs:30`, `docs/sdk.md:170`).

use ekr_sdk::document::{
    AgentId, Assertion, EvidenceId, GraphRootId, NodeId, NodeSupersession, Object, Predicate,
    PropertyId, Subject, TemporalRange, Timestamp, TransactionBuilder, TransactionDocument,
    ValidationProfile, Value,
};

/// Each SDK profile beside the kernel profile of the same ruleset.
fn profiles() -> [(ValidationProfile, ekr_kernel::ValidationProfileV1); 4] {
    let validator = AgentId::mint();
    [
        (
            ValidationProfile::V1,
            ekr_kernel::ValidationProfileV1::deterministic(validator),
        ),
        (
            ValidationProfile::V2,
            ekr_kernel::ValidationProfileV1::schema_evolving(validator),
        ),
        (
            ValidationProfile::V3,
            ekr_kernel::ValidationProfileV1::identity_keeping(validator),
        ),
        (
            ValidationProfile::V4,
            ekr_kernel::ValidationProfileV1::supersession_exact(validator),
        ),
    ]
}

/// `yaml` with the first line listing `evidence` as a block item doubled.
fn doubled(yaml: &str, evidence: EvidenceId) -> String {
    let mut lines: Vec<&str> = yaml.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == format!("- {evidence}"))
        .expect("the operation lists its evidence before the transaction does");
    lines.insert(at, lines[at]);
    lines.join("\n") + "\n"
}

/// `yaml` with the first block list of `evidence` (its key line and its one item) replaced by
/// the flow sequence `flow`.
fn flowed(yaml: &str, evidence: EvidenceId, flow: &str) -> String {
    let mut lines: Vec<String> = yaml.lines().map(str::to_owned).collect();
    let at = lines
        .iter()
        .position(|line| line.trim() == format!("- {evidence}"))
        .expect("the supersession lists its evidence");
    let key = lines[at - 1].clone();
    assert_eq!(key.trim(), "evidence:", "{yaml}");
    lines[at - 1] = format!("{key} {flow}");
    lines.remove(at);
    lines.join("\n") + "\n"
}

/// Whether the kernel's propose reader and the SDK reader under the same profile agree on
/// `text`: both accept it or both refuse it.
fn disagreements(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for (sdk, kernel) in profiles() {
        let by_kernel = ekr_kernel::TransactionDocument::parse_under(text.as_bytes(), &kernel);
        let by_sdk = TransactionDocument::from_yaml(text, sdk);
        if by_kernel.is_ok() != by_sdk.is_ok() {
            found.push(format!(
                "{sdk:?}: kernel {:?}, sdk {:?}",
                by_kernel.map(|_| "accepts").map_err(|e| e.to_string()),
                by_sdk.map(|_| "accepts").map_err(|e| e.to_string())
            ));
        }
    }
    found
}

/// The contract of `from_yaml` against the propose reader, on an `!AddAssertion` citing one
/// evidence id twice: `ekr propose` refuses it under every profile (the kernel's assertion
/// evidence is a `unique_set`, `crates/ekr-graph/src/assertion.rs:640`); `from_yaml` collapses it
/// under every profile, v4 included, because the SDK's `Assertion.evidence` is a plain
/// `BTreeSet` (`crates/ekr-sdk/src/document/graph.rs:239`).
#[test]
#[ignore = "SDK assertion evidence decoding is profile-independent and out of story:supersession-verdict-fixes-replay-safe; kept for story:merge-entity-review-follow-ups"]
fn from_yaml_refuses_an_assertion_citing_one_evidence_id_twice_as_propose_does() {
    let operator = AgentId::mint();
    let evidence = EvidenceId::mint();
    let document = TransactionBuilder::new(operator)
        .push(
            Assertion::new(
                GraphRootId::mint(),
                Subject::Node(NodeId::mint()),
                Predicate::Property(PropertyId::mint()),
                Object::Value(Value::String("an invented value".into())),
                operator,
            )
            .citing(evidence)
            .with_valid_time(TemporalRange::since(Timestamp::from_millis(
                1_554_076_800_000,
            )))
            .into(),
        )
        .build()
        .unwrap();
    let twice = doubled(&document.to_yaml().unwrap(), evidence);
    let found = disagreements(&twice);
    assert!(
        found.is_empty(),
        "from_yaml and ekr propose read one document two ways:\n{}",
        found.join("\n")
    );
}

/// Q7 for the readers in the shapes the kernel test does not write: a flow sequence listing one
/// id twice is refused by both under v4 and read once by both under v1 to v3; an empty flow list
/// and one id are read by both under every profile.
#[test]
fn q7_flow_empty_and_single_evidence_lists_read_alike_in_both_readers() {
    let operator = AgentId::mint();
    let evidence = EvidenceId::mint();
    let document = TransactionBuilder::new(operator)
        .push(
            NodeSupersession::new(NodeId::mint(), NodeId::mint(), "recorded twice", [evidence])
                .into(),
        )
        .build()
        .unwrap();
    let once = document.to_yaml().unwrap();
    let mut wrong = Vec::new();
    for (what, text, v4_refuses) in [
        ("block, once", once.clone(), false),
        (
            "flow, twice",
            flowed(&once, evidence, &format!("[{evidence}, {evidence}]")),
            true,
        ),
        ("flow, empty", flowed(&once, evidence, "[]"), false),
        (
            "flow, once",
            flowed(&once, evidence, &format!("[{evidence}]")),
            false,
        ),
    ] {
        for (sdk, kernel) in profiles() {
            let by_kernel = ekr_kernel::TransactionDocument::parse_under(text.as_bytes(), &kernel);
            let by_sdk = TransactionDocument::from_yaml(&text, sdk);
            let refuses = v4_refuses && sdk == ValidationProfile::V4;
            if by_kernel.is_err() != refuses || by_sdk.is_err() != refuses {
                wrong.push(format!(
                    "{what} {sdk:?}: kernel {:?}, sdk {:?}\n{text}",
                    by_kernel.err().map(|e| e.to_string()),
                    by_sdk.err().map(|e| e.to_string())
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
