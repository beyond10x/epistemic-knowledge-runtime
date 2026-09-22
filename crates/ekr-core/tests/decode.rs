//! Live decoding primitives keep typed key identity ahead of value decoding.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    decode::{unique_map, unique_set},
    PropertyId,
};
use serde::{Deserialize, Deserializer};

const ID: &str = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";

#[test]
fn distinct_typed_keys_and_empty_maps_decode_normally() {
    let mut input = serde_json::Deserializer::from_str("{\"left\": 1, \"right\": 2}");
    let values: BTreeMap<String, i64> = unique_map(&mut input).unwrap();
    input.end().unwrap();
    assert_eq!(
        values,
        BTreeMap::from([("left".into(), 1), ("right".into(), 2)])
    );
    let mut input = serde_json::Deserializer::from_str("{}");
    assert!(
        ekr_core::decode::unique_map::<_, PropertyId, i64>(&mut input)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_duplicate_id_is_refused_before_its_invalid_second_value_is_decoded() {
    let text = format!("{{\"{ID}\":1,\"{ID}\":\"not an integer\"}}");
    let mut input = serde_json::Deserializer::from_str(&text);
    let error = unique_map::<_, PropertyId, i64>(&mut input).unwrap_err();
    assert!(error.to_string().contains("duplicate decoded map key"));
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Folded(String);
impl<'de> Deserialize<'de> for Folded {
    fn deserialize<D: Deserializer<'de>>(input: D) -> Result<Self, D::Error> {
        Ok(Self(String::deserialize(input)?.to_ascii_lowercase()))
    }
}
#[test]
fn uniqueness_uses_the_decoded_key_type_not_its_input_spelling() {
    let mut input =
        serde_json::Deserializer::from_str("{\"field\":1,\"FIELD\":\"not an integer\"}");
    let error = unique_map::<_, Folded, i64>(&mut input).unwrap_err();
    assert!(error.to_string().contains("duplicate decoded map key"));
}

#[test]
fn distinct_set_members_and_the_empty_set_decode_without_losing_values() {
    let mut input = serde_json::Deserializer::from_str("[\"right\",\"left\"]");
    let values: BTreeSet<String> = ekr_core::decode::unique_set(&mut input).unwrap();
    input.end().unwrap();
    assert_eq!(values, BTreeSet::from(["left".into(), "right".into()]));
    let mut input = serde_json::Deserializer::from_str("[]");
    assert!(unique_set::<_, PropertyId>(&mut input).unwrap().is_empty());
    input.end().unwrap();
}

#[test]
fn duplicate_set_identity_is_refused_instead_of_discarded() {
    let text = format!("[\"{ID}\",\"{ID}\"]");
    let mut input = serde_json::Deserializer::from_str(&text);
    let error = unique_set::<_, PropertyId>(&mut input).unwrap_err();
    assert!(error.to_string().contains("duplicate decoded set member"));
}

#[test]
fn set_uniqueness_uses_decoded_values_instead_of_input_spelling() {
    let mut input = serde_json::Deserializer::from_str("[\"field\",\"FIELD\"]");
    let error = unique_set::<_, Folded>(&mut input).unwrap_err();
    assert!(error.to_string().contains("duplicate decoded set member"));
}
