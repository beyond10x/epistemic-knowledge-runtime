use serde_yaml_ng::observation::{Documents, Event, ScalarKind};

#[test]
fn global_and_local_tags_are_observable_without_changing_scalar_resolution() {
    for (input, tag, local, expected) in [
        (
            "!<tag:example.test,2026:scalar> 0.0",
            "tag:example.test,2026:scalar",
            false,
            ScalarKind::String,
        ),
        ("!sample 0.0", "!sample", true, ScalarKind::Float),
        (
            "!!float 0.0",
            "tag:yaml.org,2002:float",
            false,
            ScalarKind::Float,
        ),
        (
            "!!str 0.0",
            "tag:yaml.org,2002:str",
            false,
            ScalarKind::String,
        ),
    ] {
        let mut documents = Documents::from_str(input).unwrap();
        let document = documents.next_document().unwrap();
        let Some(Event::Scalar(scalar)) = document.event(0).unwrap() else {
            panic!("scalar")
        };
        let observed = scalar.tag().unwrap().unwrap();
        assert_eq!(observed.decoded(), tag);
        assert_eq!(observed.enum_variant().is_some(), local);
        assert_eq!(scalar.text().unwrap(), "0.0");
        assert_eq!(scalar.kind(local).unwrap(), expected);
        assert_eq!(serde_yaml_ng::from_str::<String>(input).unwrap(), "0.0");
        document.check().unwrap();
        assert!(documents.next_document().is_none());
    }
}

#[test]
fn tag_directives_expose_complete_decoded_uris() {
    let input = "%TAG !e! tag:example.test,2026:\n---\n!e!scalar text\n";
    let mut documents = Documents::from_str(input).unwrap();
    let document = documents.next_document().unwrap();
    let Some(Event::Scalar(scalar)) = document.event(0).unwrap() else {
        panic!("scalar")
    };
    assert_eq!(
        scalar.tag().unwrap().unwrap().decoded(),
        "tag:example.test,2026:scalar"
    );
    assert_eq!(scalar.kind(false).unwrap(), ScalarKind::String);
    assert_eq!(serde_yaml_ng::from_str::<String>(input).unwrap(), "text");
}

#[test]
fn collection_tags_do_not_change_default_typed_decoding() {
    for tag in ["!items", "!<tag:example.test,2026:items>", "!!seq"] {
        let input = format!("{tag} [1, 2]");
        let mut documents = Documents::from_str(&input).unwrap();
        let document = documents.next_document().unwrap();
        assert!(matches!(
            document.event(0).unwrap(),
            Some(Event::SequenceStart(Some(_)))
        ));
        assert_eq!(serde_yaml_ng::from_str::<Vec<i64>>(&input).unwrap(), [1, 2]);
    }
    for tag in ["!fields", "!<tag:example.test,2026:fields>", "!!map"] {
        let input = format!("{tag} {{field: 1}}");
        let mut documents = Documents::from_str(&input).unwrap();
        let document = documents.next_document().unwrap();
        assert!(matches!(
            document.event(0).unwrap(),
            Some(Event::MappingStart(Some(_)))
        ));
        let direct: std::collections::BTreeMap<String, i64> =
            serde_yaml_ng::from_str(&input).unwrap();
        assert_eq!(direct["field"], 1);
    }
}

#[test]
fn aliases_are_safe_positions_without_eager_expansion() {
    let input = "[&item !sample text, *item, *item]";
    let mut documents = Documents::from_str(input).unwrap();
    let document = documents.next_document().unwrap();
    assert_eq!(document.event_count(), 5);
    for index in [2, 3] {
        let Some(Event::Alias { target }) = document.event(index).unwrap() else {
            panic!("alias")
        };
        assert_eq!(target, 1);
        let Some(Event::Scalar(value)) = document.event(target).unwrap() else {
            panic!("target")
        };
        assert_eq!(value.text().unwrap(), "text");
    }
    assert!(document.event(usize::MAX).unwrap().is_none());
    let mut documents = Documents::from_str("&cycle [*cycle]").unwrap();
    let document = documents.next_document().unwrap();
    assert_eq!(document.event_count(), 3);
    assert!(matches!(
        document.event(1).unwrap(),
        Some(Event::Alias { target: 0 })
    ));
}

#[test]
fn loader_errors_and_document_termination_remain_observable() {
    for input in ["*missing", "[unterminated"] {
        let mut documents = Documents::from_str(input).unwrap();
        assert!(documents.next_document().unwrap().check().is_err());
    }
    let mut documents = Documents::from_str("---\n1\n...\n---\n\n").unwrap();
    documents.next_document().unwrap().check().unwrap();
    documents.next_document().unwrap().check().unwrap();
    assert!(documents.next_document().is_none());
}

#[test]
fn scalar_classification_reuses_the_default_resolver() {
    for (input, expected) in [
        ("null", ScalarKind::Null),
        ("true", ScalarKind::Boolean),
        ("0x10", ScalarKind::Integer),
        (".NaN", ScalarKind::Float),
        ("'42'", ScalarKind::String),
        ("!!bool true", ScalarKind::Boolean),
        ("!sample 42", ScalarKind::Integer),
        ("!<tag:example.test,2026:scalar> 42", ScalarKind::String),
    ] {
        let mut documents = Documents::from_str(input).unwrap();
        let document = documents.next_document().unwrap();
        let Some(Event::Scalar(scalar)) = document.event(0).unwrap() else {
            panic!("scalar")
        };
        let local = scalar
            .tag()
            .unwrap()
            .is_some_and(|tag| tag.enum_variant().is_some());
        assert_eq!(scalar.kind(local).unwrap(), expected);
        let direct: serde_yaml_ng::Value = serde_yaml_ng::from_str(input).unwrap();
        let direct = match direct {
            serde_yaml_ng::Value::Tagged(value) => value.value,
            value => value,
        };
        assert!(match expected {
            ScalarKind::Null => direct.is_null(),
            ScalarKind::Boolean => direct.is_bool(),
            ScalarKind::Integer => direct.is_i64() || direct.is_u64(),
            ScalarKind::Float => direct.is_f64(),
            ScalarKind::String => direct.is_string(),
        });
    }
    let mut documents = Documents::from_str("!!bool wrong").unwrap();
    let document = documents.next_document().unwrap();
    let Some(Event::Scalar(scalar)) = document.event(0).unwrap() else {
        panic!("scalar")
    };
    assert!(scalar.kind(false).is_err());
    assert!(serde_yaml_ng::from_str::<serde_yaml_ng::Value>("!!bool wrong").is_err());
}
