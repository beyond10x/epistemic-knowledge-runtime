//! Generated document adoption exercises serialization, not kernel admission.

use ekr_sdk::contracts;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

fn roundtrip<T: DeserializeOwned + Serialize>(value: Value) {
    let typed: T =
        serde_json::from_value(value.clone()).expect("generated type reads the wire document");
    assert_eq!(serde_json::to_value(typed).unwrap(), value);
}

#[test]
fn generated_recursive_local_schema_and_receipts_cross_the_sdk_boundary() {
    roundtrip::<contracts::EkrIntegrateValueSpec>(json!({
        "value_kind":"List", "element":{"value_kind":"List", "element":{"value_kind":"String"}}
    }));
    roundtrip::<contracts::EkrIntegrateInterpretationCoordinate>(json!({
        "interpretation_id":"018fef55-1400-7000-8000-000000000001", "version":1
    }));
    for (outcome, retained) in [("Retained", false), ("AlreadyRetained", true)] {
        roundtrip::<contracts::EkrObserveObservationImportReceipt>(json!({
            "already_retained":retained, "content_hash":"a".repeat(64),
            "observation_id":"018fef55-1400-8000-8000-000000000002", "outcome":outcome
        }));
    }
}

#[test]
fn generated_semantic_enum_names_preserve_legacy_wire_labels() {
    roundtrip::<contracts::EkrIntegrateExtractionFormat>(json!("ekr.extraction-document/1"));
    roundtrip::<contracts::EkrKernelHumanDecisionFormat>(json!("ekr.human-decision/1"));
}

#[test]
fn generated_required_fields_and_absence_remain_distinct() {
    assert!(
        serde_json::from_value::<contracts::EkrIntegrateInterpretationCoordinate>(
            json!({"version":1})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<contracts::EkrIntegrateValueSpec>(
        json!({"value_kind":"String", "element":null})
    )
    .is_err());
    roundtrip::<contracts::EkrIntegrateValueSpec>(json!({"value_kind":"String"}));
}
