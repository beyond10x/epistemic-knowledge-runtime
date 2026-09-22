//! The host envelope's exact format tag and the valid-time selector's typed refusal, through the
//! crate's public paths. All identities are synthetic.

use ekr::host::{CliHostConfigurationV1, CliHostFormatV1, ValidAtError};
use ekr_core::Timestamp;
use serde_json::{json, Value};

const OPERATOR: &str = "00000000-0000-0000-0000-000000000001";
const VALIDATOR: &str = "00000000-0000-0000-0000-000000000002";

fn host_document() -> Value {
    json!({
        "format": "ekr.cli-host/1", "tenant": "runtime",
        "context": {"operator": OPERATOR, "validator": VALIDATOR},
        "authority": {
            "format": "ekr.authority-state/1",
            "agents": {
                OPERATOR: {"id": OPERATOR, "name": "Operator", "capabilities": ["read"]},
                VALIDATOR: {"id": VALIDATOR, "name": "Validator", "capabilities": ["validate"]}
            },
            "validation_profile": {
                "format": "ekr.p1-validation-profile/1", "ruleset": "ekr.p1-deterministic/1",
                "checks": ["Structural", "Reference", "Type", "Cardinality",
                    "OntologyConstraint", "Provenance", "Authorization"],
                "validator": VALIDATOR, "proposer_separation": "distinct-authenticated-actor/1",
                "provenance": "retained-admissible-evidence/1", "application": "ekr.p1-apply/1"
            }
        }
    })
}

#[test]
fn the_host_format_tag_is_one_exact_json_string_in_both_directions() {
    assert_eq!(
        serde_json::to_value(CliHostFormatV1::V1).unwrap(),
        json!("ekr.cli-host/1")
    );
    let decoded: CliHostFormatV1 = serde_json::from_value(json!("ekr.cli-host/1")).unwrap();
    assert_eq!(decoded, CliHostFormatV1::V1);

    // The Rust variant name is not a spelling of the tag, and neither is any near miss.
    for spelling in [
        "V1",
        "ekr.cli-host/2",
        "EKR.cli-host/1",
        "ekr.cli-host/1 ",
        "",
    ] {
        let error = serde_json::from_value::<CliHostFormatV1>(json!(spelling))
            .expect_err(spelling)
            .to_string();
        assert!(
            error.contains("unknown variant") && error.contains("ekr.cli-host/1"),
            "{spelling:?}: {error}"
        );
    }
    for shape in [
        json!(null),
        json!(1),
        json!(["ekr.cli-host/1"]),
        json!({"ekr.cli-host/1": null}),
    ] {
        assert!(
            serde_json::from_value::<CliHostFormatV1>(shape.clone()).is_err(),
            "{shape}"
        );
    }
}

#[test]
fn a_host_document_carries_the_format_tag_through_decode_and_encode() {
    let host = CliHostConfigurationV1::from_json(host_document().to_string().as_bytes()).unwrap();
    assert_eq!(host.format, CliHostFormatV1::V1);
    let encoded = serde_json::to_value(&host).unwrap();
    assert_eq!(encoded["format"], json!("ekr.cli-host/1"));
    assert_eq!(
        CliHostConfigurationV1::from_json(encoded.to_string().as_bytes()).unwrap(),
        host
    );

    let mut substituted = host_document();
    substituted["format"] = json!("V1");
    let error = CliHostConfigurationV1::from_json(substituted.to_string().as_bytes())
        .expect_err("the variant name is not the envelope format");
    assert!(error.to_string().contains("unknown variant"), "{error}");
}

#[test]
fn a_refused_selector_is_a_valid_at_error_naming_the_input_and_the_grammar() {
    for input in [
        "2026-02-30",
        "2026-9-22",
        "2026-09-22T00:00:00Z",
        "1e3",
        "+1",
        "",
        "yesterday",
    ] {
        let error: ValidAtError = ekr::host::parse_valid_at(input).expect_err(input);
        assert_eq!(error.input(), input);
        assert_eq!(
            error.to_string(),
            format!(
                "{input:?} is not a valid-time selector: expected canonical decimal milliseconds \
                 or YYYY-MM-DD"
            )
        );
        let reported: Box<dyn std::error::Error> = Box::new(error.clone());
        assert!(reported.source().is_none());
        assert_eq!(reported.to_string(), error.to_string());
        assert_eq!(ekr::host::parse_valid_at(input).unwrap_err(), error);
    }
    assert_ne!(
        ekr::host::parse_valid_at("2026-02-30").unwrap_err(),
        ekr::host::parse_valid_at("2026-02-31").unwrap_err()
    );
}

#[test]
fn both_selector_grammars_name_the_same_instant() {
    // 2026-09-22T00:00:00Z, computed outside the crate: 1_790_035_200 seconds since the epoch.
    const MIDNIGHT: i64 = 1_790_035_200_000;
    const DAY: i64 = 86_400_000;
    let date = ekr::host::parse_valid_at("2026-09-22").unwrap();
    assert_eq!(date, Timestamp::from_millis(MIDNIGHT));
    assert_eq!(
        ekr::host::parse_valid_at(&MIDNIGHT.to_string()).unwrap(),
        date
    );
    assert_eq!(
        ekr::host::parse_valid_at("2026-09-23").unwrap().millis() - date.millis(),
        DAY
    );
    assert_eq!(
        date.millis() - ekr::host::parse_valid_at("2026-09-21").unwrap().millis(),
        DAY
    );
    assert_eq!(
        ekr::host::parse_valid_at(&(MIDNIGHT + 1).to_string())
            .unwrap()
            .millis(),
        MIDNIGHT + 1
    );
}
