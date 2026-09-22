//! Independent public host-input boundary cases; all identities are synthetic.

use ekr::host::{parse_valid_at, CliHostConfigurationV1};
use serde_json::{json, Value};

const OPERATOR: &str = "00000000-0000-0000-0000-000000000001";
const VALIDATOR: &str = "00000000-0000-0000-0000-000000000002";
const AGENT: &str = "/authority/agents/00000000-0000-0000-0000-000000000001";

fn fixture() -> Value {
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

fn refused_by_both(input: &str, reason: &str) {
    for result in [
        CliHostConfigurationV1::from_json(input.as_bytes()),
        serde_json::from_reader::<_, CliHostConfigurationV1>(input.as_bytes()),
    ] {
        let error = result.expect_err(input).to_string();
        assert!(error.contains(reason), "expected {reason:?}, got {error}");
    }
}

fn substitute_object(input: &Value, path: &str, replacement: &str) -> String {
    let original = input.pointer(path).unwrap().to_string();
    let text = input.to_string();
    assert_eq!(text.matches(&original).count(), 1);
    text.replacen(&original, replacement, 1)
}

#[test]
fn all_named_carrier_fields_are_strict_through_both_public_routes() {
    let original = fixture();
    CliHostConfigurationV1::from_json(original.to_string().as_bytes()).unwrap();
    for path in [
        "",
        "/context",
        "/authority",
        AGENT,
        "/authority/validation_profile",
    ] {
        let object = original.pointer(path).unwrap().as_object().unwrap();
        for field in object.keys() {
            let mut missing = original.clone();
            missing
                .pointer_mut(path)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field);
            refused_by_both(&missing.to_string(), &format!("missing field `{field}`"));

            let text = original.pointer(path).unwrap().to_string();
            let escaped = format!("\\u{:04x}{}", field.as_bytes()[0], &field[1..]);
            // The repeated value is intentionally malformed: duplicate detection must win.
            let duplicate = format!("{},\"{escaped}\":[", &text[..text.len() - 1]);
            refused_by_both(
                &substitute_object(&original, path, &duplicate),
                &format!("duplicate field `{field}`"),
            );
        }
        let text = original.pointer(path).unwrap().to_string();
        let unknown = format!("{{\"un\\u006bnown\":null,{}", &text[1..]);
        refused_by_both(
            &substitute_object(&original, path, &unknown),
            "unknown field `unknown`",
        );
    }
}

#[test]
fn named_object_shapes_survive_value_and_streaming_deserializer_boundaries() {
    let original = fixture();
    for (path, fields) in [
        ("", vec!["format", "tenant", "context", "authority"]),
        ("/context", vec!["operator", "validator"]),
        ("/authority", vec!["format", "agents", "validation_profile"]),
        (AGENT, vec!["id", "name", "capabilities"]),
        (
            "/authority/validation_profile",
            vec![
                "format",
                "ruleset",
                "checks",
                "validator",
                "proposer_separation",
                "provenance",
                "application",
            ],
        ),
    ] {
        let positional = Value::Array(
            fields
                .iter()
                .map(|field| original.pointer(path).unwrap()[field].clone())
                .collect(),
        );
        for replacement in [positional, json!([]), Value::Null, json!(0), json!(true)] {
            let mut input = original.clone();
            *input.pointer_mut(path).unwrap() = replacement;
            refused_by_both(&input.to_string(), "invalid type");
            assert!(
                serde_json::from_value::<CliHostConfigurationV1>(input).is_err(),
                "{path}"
            );
        }
    }
}

#[test]
fn decoded_collisions_refuse_without_unicode_normalization() {
    let original = fixture();
    let path = "/authority/agents";
    let map = original.pointer(path).unwrap().to_string();
    let escaped_id = OPERATOR
        .chars()
        .map(|character| format!("\\u{:04x}", u32::from(character)))
        .collect::<String>();
    let duplicate = format!("{},\"{escaped_id}\":[", &map[..map.len() - 1]);
    refused_by_both(
        &substitute_object(&original, path, &duplicate),
        "duplicate decoded map key",
    );

    for members in [r#"["𝄞","\uD834\uDD1E"]"#, r#"["","\u0000","\u0000"]"#] {
        let input = original.to_string().replacen(r#"["read"]"#, members, 1);
        refused_by_both(&input, "duplicate decoded set member");
    }
    let mut distinct = original;
    distinct.pointer_mut(AGENT).unwrap()["capabilities"] = json!(["", "é", "e\u{301}", "𝄞", "\0"]);
    let host = CliHostConfigurationV1::from_json(distinct.to_string().as_bytes()).unwrap();
    assert_eq!(
        host.authority.agents[&host.context.operator]
            .capabilities
            .len(),
        5
    );
}

#[test]
fn legitimate_strings_and_lists_retain_values_without_kernel_policy_checks() {
    let mut input = fixture();
    input["tenant"] = json!("9007199254740993\0𝄞");
    input.pointer_mut(AGENT).unwrap()["name"] = json!("-9223372036854775808");
    input.pointer_mut(AGENT).unwrap()["capabilities"] = json!(["-0", "1", "1.0", "1e0"]);
    input.pointer_mut(AGENT).unwrap()["id"] = json!(VALIDATOR);
    input["authority"]["format"] = json!("unsupported-authority");
    input["authority"]["validation_profile"]["checks"] = json!(["Type", "Structural", "Type"]);
    let host = CliHostConfigurationV1::from_json(input.to_string().as_bytes()).unwrap();
    assert_eq!(host.tenant, "9007199254740993\0𝄞");
    assert_eq!(
        host.authority.agents[&host.context.operator].id,
        host.context.validator
    );
    assert_eq!(host.authority.validation_profile.checks.len(), 3);
    assert_eq!(serde_json::to_value(&host).unwrap(), input);
    assert_eq!(
        serde_json::from_value::<CliHostConfigurationV1>(input.clone()).unwrap(),
        host
    );
    for path in [
        "/tenant",
        "/authority/agents/00000000-0000-0000-0000-000000000001/name",
        "/authority/agents/00000000-0000-0000-0000-000000000001/capabilities/0",
    ] {
        let mut numeric = input.clone();
        *numeric.pointer_mut(path).unwrap() = json!(9_007_199_254_740_993_u64);
        refused_by_both(&numeric.to_string(), "invalid type");
    }
}

#[test]
fn valid_dates_follow_independent_gregorian_ordinal_across_centuries() {
    fn leap(year: i64) -> bool {
        year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
    }
    fn preceding_days(year: i64) -> i64 {
        365 * year + (year + 3) / 4 - (year + 99) / 100 + (year + 399) / 400
    }
    for year in [
        0_i64, 1, 4, 100, 400, 1600, 1700, 1900, 1969, 1970, 2000, 2100, 2400, 9999,
    ] {
        let mut ordinal = preceding_days(year) - preceding_days(1970);
        for (month, days) in [
            31,
            if leap(year) { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ]
        .into_iter()
        .enumerate()
        {
            for day in 1..=days {
                let input = format!("{year:04}-{:02}-{day:02}", month + 1);
                let expected = ordinal * 86_400_000;
                assert_eq!(
                    parse_valid_at(&input).unwrap().millis(),
                    expected,
                    "{input}"
                );
                assert_eq!(
                    parse_valid_at(&expected.to_string()).unwrap().millis(),
                    expected
                );
                ordinal += 1;
            }
            for day in [0, days + 1, 99] {
                let input = format!("{year:04}-{:02}-{day:02}", month + 1);
                assert_eq!(parse_valid_at(&input).unwrap_err().input(), input);
            }
        }
    }
}

#[test]
fn decimal_extremes_and_byte_level_noncanonical_forms_stay_distinct() {
    for value in [
        i64::MIN,
        i64::MIN + 1,
        -253_402_214_400_000,
        -1,
        0,
        1,
        9_007_199_254_740_993,
        i64::MAX - 1,
        i64::MAX,
    ] {
        let input = value.to_string();
        assert_eq!(parse_valid_at(&input).unwrap().millis(), value);
        for malformed in [
            format!("+{input}"),
            format!("0{input}"),
            format!("{input}.0"),
            format!("{input}e0"),
            format!("{input}\0"),
            format!("{input}\r"),
        ] {
            let error = parse_valid_at(&malformed).unwrap_err();
            assert_eq!(error.input(), malformed);
            assert!(error.to_string().contains("valid-time selector"));
        }
    }
    for input in [
        "9223372036854775808",
        "-9223372036854775809",
        "18446744073709551615",
        "-0",
        "2026-01-01\0",
        "2026-01-01\r",
        "2026-01-01\u{a0}",
        "2026-01-01T00:00:00+00:00",
        "2026-0é-1",
        "\0\0\0\0-01-01",
        "2026−01-01",
    ] {
        assert_eq!(parse_valid_at(input).unwrap_err().input(), input);
    }
}
