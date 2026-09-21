//! Which values canonical state admits: `architecture-decision-record:0005-float-is-not-canonical`.
//!
//! `crates/ekr-core/src/canonical.rs` rule 4 admits no float, and `Value::Float` exists — so the
//! question "may this value be content-addressed" is a question about the *whole* value, not about
//! its outermost kind. A `List` of one `Record` of one `Float` is not admissible, and a refusal
//! that says only "no" leaves the caller to find which of a hundred fields it meant.
//!
//! So the answer this crate gives is the **path** to the first value canonical state refuses, and
//! these cases are about that path being right at depth, not merely present.

use std::collections::BTreeMap;

use ekr_core::{NodeId, Timestamp};
use ekr_ontology::{Value, ValuePath};

/// The path a refusal names, or `None` when the value is admissible.
fn refusal(value: &Value) -> Option<String> {
    value
        .inadmissible_in_canonical_state()
        .map(|path| path.to_string())
}

/// One value of every kind but `Float`, `List` and `Record`.
fn admissible_scalars() -> Vec<Value> {
    vec![
        Value::String("acme".to_owned()),
        Value::Boolean(true),
        Value::Integer(-7),
        Value::Decimal("0.1".to_owned()),
        Value::Timestamp(Timestamp::from_millis(1_704_067_200_000)),
        Value::Duration(90_000),
        Value::NodeRef(NodeId::mint()),
        Value::Enum("decided".to_owned()),
    ]
}

/// A record, built from pairs, so a case reads as the value it is about.
fn record(fields: &[(&str, Value)]) -> Value {
    Value::Record(
        fields
            .iter()
            .map(|(name, value)| ((*name).to_owned(), value.clone()))
            .collect::<BTreeMap<String, Value>>(),
    )
}

/// A path is built from the offending value upwards, and reads from the whole value downwards.
///
/// Stated over the builder directly, and not only through a refusal: the two steps are what a
/// caller above this crate composes when it reports a value inside a structure of its own — the
/// kernel naming the operation a refused value arrived in, for one — so the rendering is a
/// contract rather than a detail of how the walk below happens to unwind.
#[test]
fn a_path_reads_from_the_whole_value_down_to_the_part() {
    assert_eq!(ValuePath::root().to_string(), "value");
    assert_eq!(ValuePath::root().inside_element(2).to_string(), "value[2]");
    assert_eq!(
        ValuePath::root().inside_field("reading").to_string(),
        "value.reading"
    );

    // Each step is taken by the level *above*, so the deepest one is added first.
    assert_eq!(
        ValuePath::root()
            .inside_field("mean")
            .inside_element(0)
            .inside_field("samples")
            .to_string(),
        "value.samples[0].mean"
    );

    // A field name that would otherwise read as two steps, or as none.
    assert_eq!(
        ValuePath::root().inside_field("mean reading").to_string(),
        "value.\"mean reading\""
    );
    assert_eq!(
        ValuePath::root().inside_field("a.b").to_string(),
        "value.\"a.b\""
    );
    assert_eq!(ValuePath::root().inside_field("").to_string(), "value.\"\"");
}

#[test]
fn every_kind_but_a_float_is_admissible_on_its_own() {
    for value in admissible_scalars() {
        assert_eq!(
            refusal(&value),
            None,
            "{:?} is not a float and carries none",
            value.kind()
        );
    }

    assert_eq!(
        refusal(&Value::List(admissible_scalars())),
        None,
        "a list of admissible values is admissible"
    );
    assert_eq!(
        refusal(&Value::List(Vec::new())),
        None,
        "an empty list carries no float"
    );
    assert_eq!(
        refusal(&Value::Record(BTreeMap::new())),
        None,
        "an empty record carries no float"
    );
}

#[test]
fn a_bare_float_is_refused_and_the_path_is_the_value_itself() {
    assert_eq!(refusal(&Value::Float(1.5)), Some("value".to_owned()));
    assert_eq!(refusal(&Value::Float(f64::NAN)), Some("value".to_owned()));
    assert_eq!(refusal(&Value::Float(-0.0)), Some("value".to_owned()));
}

#[test]
fn a_float_inside_a_compound_is_refused_at_the_path_it_sits_at() {
    let cases = [
        (
            Value::List(vec![Value::Integer(1), Value::Float(2.5)]),
            "value[1]",
        ),
        (record(&[("reading", Value::Float(2.5))]), "value.reading"),
        (
            record(&[(
                "samples",
                Value::List(vec![record(&[("mean", Value::Float(0.5))])]),
            )]),
            "value.samples[0].mean",
        ),
        (
            Value::List(vec![Value::List(vec![
                Value::String("a".to_owned()),
                Value::Float(0.0),
            ])]),
            "value[0][1]",
        ),
        (
            // A field name that is not an identifier is quoted, so the path stays unambiguous.
            record(&[("mean reading", Value::Float(0.5))]),
            "value.\"mean reading\"",
        ),
    ];

    for (value, path) in cases {
        assert_eq!(
            refusal(&value),
            Some(path.to_owned()),
            "{value:?} is refused at {path}"
        );
    }
}

#[test]
fn the_first_refused_value_in_the_values_own_order_is_the_one_named() {
    // Two floats: the path names the earlier, and "earlier" is the value's own order — key order
    // in a record, position in a list — never the order the caller inserted them in.
    let built_backwards = Value::Record(
        [
            ("second".to_owned(), Value::Float(2.0)),
            ("first".to_owned(), Value::Float(1.0)),
        ]
        .into_iter()
        .collect::<BTreeMap<String, Value>>(),
    );
    assert_eq!(refusal(&built_backwards), Some("value.first".to_owned()));

    assert_eq!(
        refusal(&Value::List(vec![
            Value::Integer(0),
            Value::Float(1.0),
            Value::Float(2.0),
        ])),
        Some("value[1]".to_owned())
    );
}

#[test]
fn nesting_does_not_hide_a_float_at_any_depth() {
    // Ten layers, alternating list and record, with the path the wrapping built kept beside the
    // value — so the expectation is the construction rather than a string written by hand.
    let mut value = Value::Float(0.5);
    let mut path = "value".to_owned();

    for depth in 0..10 {
        if depth % 2 == 0 {
            value = Value::List(vec![Value::Integer(0), value]);
            path = format!("value[1]{}", &path["value".len()..]);
        } else {
            value = record(&[("held", value), ("other", Value::Integer(0))]);
            path = format!("value.held{}", &path["value".len()..]);
        }

        assert_eq!(
            refusal(&value),
            Some(path.clone()),
            "at depth {depth} the float is still refused, at its own path"
        );
    }

    // And the same shape with the float replaced by an integer is admissible all the way down: the
    // depth is not what the refusal is about.
    let mut admissible = Value::Integer(9);
    for depth in 0..10 {
        admissible = if depth % 2 == 0 {
            Value::List(vec![Value::Integer(0), admissible])
        } else {
            record(&[("held", admissible), ("other", Value::Integer(0))])
        };
        assert_eq!(refusal(&admissible), None, "depth {depth} refuses nothing");
    }
}
