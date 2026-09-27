//! `ekr resolve`: one typed reference resolved by `ekr_integrate::resolve` against the canonical
//! snapshot of one verified read, and the `typed-reference` document's JSON Schema. Reads only:
//! a resolution proposes nothing and writes nothing (`systems/ekr/domains/integrate.yaml`).

use std::io::Read;
use std::path::Path;

use ekr_core::{RevisionNumber, TypeId};
use ekr_graph::GraphSnapshot;
use ekr_integrate::{ResolutionOutcome, ResolutionRefusal, ResolutionRefusalCode, TypedReference};
use ekr_kernel::Runtime;

use crate::exit::Failure;

/// The most bytes of a typed-reference document read.
const LIMIT: u64 = 1 << 20;

/// Reads and decodes the document at `document`, or stdin for `-`. A document that cannot be read
/// or is not a typed reference is a fault naming it, before any store is opened.
pub(super) fn read(document: &Path, stdin: &mut dyn Read) -> Result<TypedReference, Failure> {
    let fault = |error: &dyn std::fmt::Display| {
        Failure::fault(format!("typed reference {}: {error}", document.display()))
    };
    let mut text = String::new();
    super::input::open(document, stdin)?
        .take(LIMIT + 1)
        .read_to_string(&mut text)
        .map_err(|error| fault(&error))?;
    if u64::try_from(text.len()).unwrap_or(u64::MAX) > LIMIT {
        return Err(fault(&format_args!("over {LIMIT} bytes")));
    }
    strict(&text).map_err(|error| fault(&error))?;
    serde_yaml_ng::from_str(&text).map_err(|error| fault(&error))
}

/// Refuses, before anything decodes, what the typed decoder would expand or coerce and the
/// printed schema refuses: an alias (`*name`, which repeats its anchor's value on every use), a
/// tag anywhere, a `type_id` or an alias that is not a string scalar (a number, a boolean, a
/// null), and an `aliases` that is not a list. It walks the loader's event tape through the
/// vendored `serde_yaml_ng::observation` facade, which expands no alias. Everything else the
/// decoder refuses by itself, with its own message.
fn strict(text: &str) -> Result<(), String> {
    use serde_yaml_ng::observation::{Documents, Event, ScalarKind};

    let error = |e: serde_yaml_ng::Error| e.to_string();
    let mut documents = Documents::from_str(text).map_err(error)?;
    let Some(document) = documents.next_document() else {
        return Ok(());
    };
    document.check().map_err(error)?;
    let event = |at: usize| document.event(at).map_err(error);
    for at in 0..document.event_count() {
        match event(at)? {
            Some(Event::Alias { .. }) => {
                return Err("a YAML alias (`*name`) is not accepted: write each alias out".into())
            }
            Some(Event::Scalar(scalar)) if scalar.tag().map_err(error)?.is_some() => {
                return Err("a YAML tag is not accepted in a typed reference".into())
            }
            Some(Event::MappingStart(Some(_)) | Event::SequenceStart(Some(_))) => {
                return Err("a YAML tag is not accepted in a typed reference".into())
            }
            _ => {}
        }
    }
    let string = |at: usize, what: &str| match event(at)? {
        Some(Event::Scalar(scalar)) if scalar.kind(false).map_err(error)? == ScalarKind::String => {
            Ok(())
        }
        _ => Err(format!(
            "{what} is not a string; quote it (`\"123\"`) if it is one"
        )),
    };
    // The top-level mapping, key by key. A document of another shape is left to the decoder.
    if !matches!(event(0)?, Some(Event::MappingStart(None))) {
        return Ok(());
    }
    let mut at = 1;
    loop {
        let key = match event(at)? {
            Some(Event::Scalar(key)) => key.text().map_err(error)?.to_owned(),
            _ => return Ok(()),
        };
        at += 1;
        match key.as_str() {
            "type_id" => string(at, "type_id")?,
            "aliases" => {
                if !matches!(event(at)?, Some(Event::SequenceStart(None))) {
                    return Err("aliases is not a list; write `aliases: [<alias>, ...]`".into());
                }
                let mut item = at + 1;
                while !matches!(event(item)?, Some(Event::SequenceEnd) | None) {
                    string(item, "an alias")?;
                    item += 1;
                }
            }
            _ => {}
        }
        at = skip(&document, at).map_err(error)?;
    }
}

/// The position after the value at `at`: one scalar, or a whole container.
fn skip(
    document: &serde_yaml_ng::observation::Document<'_>,
    mut at: usize,
) -> Result<usize, serde_yaml_ng::Error> {
    use serde_yaml_ng::observation::Event;

    let mut depth = 0usize;
    loop {
        match document.event(at)? {
            Some(Event::MappingStart(_) | Event::SequenceStart(_)) => depth += 1,
            Some(Event::MappingEnd | Event::SequenceEnd) => depth = depth.saturating_sub(1),
            Some(_) => {}
            None => return Ok(at),
        }
        at += 1;
        if depth == 0 {
            return Ok(at);
        }
    }
}

/// Captures the requested (or newest) revision once and resolves `reference` against it. A
/// refused reference is the named refusal of its code (exit 2); every other outcome is printed.
pub(super) fn run(
    runtime: &Runtime,
    reference: &TypedReference,
    at: Option<u64>,
) -> Result<ResolutionOutcome, Failure> {
    let read = runtime.read(at.map(RevisionNumber::new))?;
    match ekr_integrate::resolve(GraphSnapshot::of(&read.graph), reference) {
        ResolutionOutcome::Refused(refusal) => Err(refused(&refusal, read.graph.revision)),
        outcome => Ok(outcome),
    }
}

/// The refusal named by its code, saying what to do instead.
fn refused(refusal: &ResolutionRefusal, revision: RevisionNumber) -> Failure {
    let ResolutionRefusal { code, reference } = refusal;
    let type_id = reference.type_id;
    let message = match code {
        ResolutionRefusalCode::ReferenceWithoutIdentity => {
            "the reference holds no alias but the empty string, so nothing identifies the node \
             it means; give at least one alias"
                .to_owned()
        }
        ResolutionRefusalCode::ReferenceTypeHasSubtypes => format!(
            "node type {type_id} is abstract or has a declared subtype at revision {revision}; \
             name the concrete type the node is an instance of (`ekr ontology`)"
        ),
        ResolutionRefusalCode::ReferenceTypeUndeclared => format!(
            "{type_id} is not a node type the ontology declares at revision {revision} \
             (`ekr ontology`)"
        ),
    };
    Failure::refused(code.code(), message)
}

/// The shape `TypedReference` decodes, for its schema: `ekr-integrate` derives no `JsonSchema`.
#[derive(schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
#[allow(dead_code)]
struct Shape {
    type_id: TypeId,
    aliases: Vec<String>,
}

/// Every field of `TypedReference` is a field of [`Shape`]: a new field does not compile here.
const _: fn(TypedReference) -> Shape =
    |TypedReference { type_id, aliases }| Shape { type_id, aliases };

/// The JSON Schema (draft 2020-12) of a `typed-reference` document: what `ekr schema
/// typed-reference` prints.
pub(super) fn schema() -> schemars::Schema {
    let mut schema = schemars::generate::SchemaSettings::draft2020_12()
        .with_transform(schemars::transform::RecursiveTransform(
            |schema: &mut schemars::Schema| {
                schema.remove("description");
            },
        ))
        .into_generator()
        .into_root_schema_for::<Shape>();
    for (field, description) in [
        (
            "type_id",
            "The node type the node is an instance of, exactly: a concrete type from \
             `ekr ontology`, never an abstract type or one with a subtype.",
        ),
        (
            "aliases",
            "The names the node is known by, each compared byte for byte with a node's aliases. \
             Order and repeats do not matter; an empty alias identifies nothing.",
        ),
    ] {
        schema
            .get_mut("properties")
            .and_then(|properties| properties.get_mut(field))
            .and_then(serde_json::Value::as_object_mut)
            .map(|property| property.insert("description".to_owned(), description.into()));
    }
    schema.insert("title".to_owned(), "typed-reference".into());
    schema.insert(
        "description".to_owned(),
        "A typed reference for `ekr resolve`: a node named by its type and the aliases it is \
         known by, never by its canonical name (`ekr example typed-reference`). The document is \
         YAML; this schema validates it read as YAML and written as JSON. Beyond the schema, the \
         reader also refuses a key written twice and a YAML alias (`*name`), which the JSON \
         projection has already expanded."
            .into(),
    );
    schema
}
