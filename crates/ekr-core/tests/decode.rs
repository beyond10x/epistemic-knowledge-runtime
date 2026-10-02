//! Live decoding primitives keep typed key identity ahead of value decoding.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    decode::{unique_map, unique_set},
    PropertyId,
};
use serde::{Deserialize, Deserializer};

#[test]
fn shared_yaml_observation_bounds_loading_and_counts_aliases_across_documents() {
    use ekr_core::decode::yaml::{self, Expansion, Past, Tally};
    let limits = Expansion {
        depth: 3,
        nodes: 8,
        text_bytes: 3,
    };
    let mut documents = yaml::load("[&a [x], *a]", limits.depth).unwrap();
    let document = documents.next_document().unwrap();
    yaml::expand(&document, limits, &mut Tally::default()).unwrap();
    document.check().unwrap();
    assert!(matches!(
        yaml::expand(
            &document,
            Expansion { nodes: 4, ..limits },
            &mut Tally::default()
        ),
        Err(Past::Nodes(5))
    ));
    assert!(matches!(
        yaml::expand(
            &document,
            Expansion {
                text_bytes: 1,
                ..limits
            },
            &mut Tally::default()
        ),
        Err(Past::Text(2))
    ));

    let mut documents = yaml::load("[[[[[[]]]]]]", limits.depth).unwrap();
    let document = documents.next_document().unwrap();
    assert_eq!(document.event_count(), 4);
    assert!(matches!(
        yaml::expand(&document, limits, &mut Tally::default()),
        Err(Past::Depth)
    ));
    assert!(document.check().is_err());
    assert!(documents.next_document().is_none());

    let mut documents = yaml::load("one\n---\ntwo\n", limits.depth).unwrap();
    let mut tally = Tally::default();
    yaml::expand(&documents.next_document().unwrap(), limits, &mut tally).unwrap();
    assert!(matches!(
        yaml::expand(&documents.next_document().unwrap(), limits, &mut tally),
        Err(Past::Text(6))
    ));
}

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

/// The bounded YAML observation refuses, in order, a text over its byte limit, a container
/// nested past its depth, an alias and a malformed document; each says what it names.
#[test]
fn the_bounded_yaml_observation_refuses_each_bound_by_name() {
    use ekr_core::decode::{observe_yaml, YamlRefusal};

    assert_eq!(
        ekr_core::decode::observe_yaml("a: [1, {b: c}]\n", 64, 3),
        Ok(())
    );
    let large: YamlRefusal = observe_yaml("a: b\n", 4, 3).unwrap_err();
    assert_eq!(large, YamlRefusal::TooLarge { bytes: 5, limit: 4 });
    assert_eq!(large.to_string(), "5 bytes, at most 4");
    // Three containers deep is the bound; a fourth is refused before the rest is read.
    assert_eq!(observe_yaml("a: [[x]]\n", 64, 3), Ok(()));
    let deep = observe_yaml("a: [[[x]]]\n", 64, 3).unwrap_err();
    assert_eq!(deep, YamlRefusal::TooDeep { limit: 3 });
    assert_eq!(deep.to_string(), "nested more than 3 deep");
    let alias = observe_yaml("a: &n x\nb: *n\n", 64, 3).unwrap_err();
    assert!(matches!(alias, YamlRefusal::Alias { .. }), "{alias:?}");
    assert!(alias.to_string().starts_with("event "), "{alias}");
    let malformed = observe_yaml("a: [x\n", 64, 3).unwrap_err();
    assert!(
        matches!(malformed, YamlRefusal::Malformed(_)),
        "{malformed:?}"
    );
    // A second document is observed as well as the first.
    assert!(matches!(
        observe_yaml("a: x\n---\nb: &n y\nc: *n\n", 64, 3),
        Err(YamlRefusal::Alias { .. })
    ));
}

/// Aliases decode as strings only: a number, a boolean, a null or a null list is refused.
#[test]
fn strings_decode_only_strings() {
    let read =
        |json: &str| ekr_core::decode::strings(&mut serde_json::Deserializer::from_str(json));
    assert_eq!(read("[\"a\", \"b\"]").unwrap(), ["a", "b"]);
    assert_eq!(read("[]").unwrap(), Vec::<String>::new());
    for refused in ["[1]", "[true]", "[null]", "null", "\"a\""] {
        assert!(read(refused).is_err(), "{refused}");
    }
}
