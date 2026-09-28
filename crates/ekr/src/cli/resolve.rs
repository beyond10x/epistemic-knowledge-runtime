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

/// The most nesting a typed-reference document may have: flow collections (`[`, `{`) and block
/// indentation levels, each counted by [`nesting`] before the YAML loader runs, whose scan is
/// quadratic in flow depth. A typed reference needs two.
const DEPTH: usize = 64;

/// What a typed reference is, for a refusal of its shape.
const SHAPE: &str = "a typed reference is one mapping with the keys `type_id` and `aliases`";

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
    nesting(&text).map_err(|error| fault(&error))?;
    decode(&text).map_err(|error| fault(&error))
}

/// One linear pass over the bytes, before any YAML is loaded, that refuses a document nested past
/// [`DEPTH`]. It counts every `[` and `{` in the document, quoted or not, rather than tracking
/// depth: telling a quoted `]` from a real one would need this pass to agree with the loader on
/// where every quoted scalar and comment starts, and a disagreement would hide nesting. So the
/// count bounds the flow depth from above and cannot be fooled. Block nesting is the number of
/// indentation levels open on a line, plus the compact `- ` and `? ` entries that start it; the
/// lines of a block scalar (`|`, `>`) are not counted.
fn nesting(text: &str) -> Result<(), String> {
    let too_deep = || format!("nested deeper than {DEPTH} levels");
    let flow = text
        .bytes()
        .filter(|byte| matches!(byte, b'[' | b'{'))
        .count();
    if flow > DEPTH {
        return Err(format!(
            "{} (it opens {flow} flow collections `[`, `{{`; at most {DEPTH})",
            too_deep()
        ));
    }
    let mut indents: Vec<usize> = Vec::new();
    let mut block_scalar: Option<usize> = None;
    for line in text.lines() {
        let content = line.trim_start_matches(' ');
        let indent = line.len() - content.len();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }
        if let Some(parent) = block_scalar {
            if indent > parent {
                continue;
            }
            block_scalar = None;
        }
        while indents.last().is_some_and(|open| *open > indent) {
            indents.pop();
        }
        if indents.last() != Some(&indent) {
            indents.push(indent);
        }
        let mut rest = content;
        let mut compact = 0;
        while let Some(after) = rest.strip_prefix("- ").or_else(|| rest.strip_prefix("? ")) {
            compact += 1;
            rest = after.trim_start_matches(' ');
        }
        if indents.len() + compact > DEPTH {
            return Err(too_deep());
        }
        let header = content.split(" #").next().unwrap_or(content).trim_end();
        if header
            .rsplit(' ')
            .next()
            .is_some_and(|last| last.starts_with('|') || last.starts_with('>'))
        {
            block_scalar = Some(indent);
        }
    }
    Ok(())
}

/// Decodes the typed reference from one load of the document, walking the loader's event tape
/// through the vendored `serde_yaml_ng::observation` facade, which expands no alias. Refused: an
/// alias (`*name`, which repeats its anchor's value on every use); a tag anywhere; anything but
/// one mapping holding exactly `type_id` and `aliases`, each once; a `type_id` that is not a
/// string scalar holding an id; an `aliases` that is not a list of string scalars (a number, a
/// boolean or a null is not one); and a second document. These are what the printed schema
/// refuses, and what the typed decoder of `TypedReference` refuses or would coerce.
fn decode(text: &str) -> Result<TypedReference, String> {
    use serde_yaml_ng::observation::{Documents, Event, ScalarKind};

    let error = |e: serde_yaml_ng::Error| e.to_string();
    let mut documents = Documents::from_str(text).map_err(error)?;
    let document = documents.next_document().ok_or(SHAPE)?;
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
            Ok(scalar.text().map_err(error)?.to_owned())
        }
        _ => Err(format!(
            "{what} is not a string; quote it (`\"123\"`) if it is one"
        )),
    };
    if !matches!(event(0)?, Some(Event::MappingStart(None))) {
        return Err(SHAPE.into());
    }
    let (mut type_id, mut aliases): (Option<TypeId>, Option<Vec<String>>) = (None, None);
    let mut at = 1;
    loop {
        if matches!(event(at)?, Some(Event::MappingEnd)) {
            at += 1;
            break;
        }
        let key = string(at, "a key")?;
        at += 1;
        match key.as_str() {
            "type_id" if type_id.is_none() => {
                type_id = Some(self::type_id(&string(at, "type_id")?)?);
                at += 1;
            }
            "aliases" if aliases.is_none() => {
                if !matches!(event(at)?, Some(Event::SequenceStart(None))) {
                    return Err("aliases is not a list; write `aliases: [<alias>, ...]`".into());
                }
                at += 1;
                let mut found = Vec::new();
                while !matches!(event(at)?, Some(Event::SequenceEnd)) {
                    found.push(string(at, "an alias")?);
                    at += 1;
                }
                aliases = Some(found);
                at += 1;
            }
            "type_id" | "aliases" => return Err(format!("`{key}` is written twice")),
            _ => return Err(format!("unknown key `{key}`; {SHAPE}")),
        }
    }
    if at != document.event_count() || documents.next_document().is_some() {
        return Err("more than one document; a typed reference is one".into());
    }
    reference(type_id, aliases)
}

/// The typed reference of a document's two fields, once its reader has taken them out of the
/// document's syntax: what the YAML reader here and the `resolve` tool of `ekr mcp` both build,
/// naming a missing field alike.
pub(super) fn reference(
    type_id: Option<TypeId>,
    aliases: Option<Vec<String>>,
) -> Result<TypedReference, String> {
    match (type_id, aliases) {
        (Some(type_id), Some(aliases)) => Ok(TypedReference { type_id, aliases }),
        (None, _) => Err(format!("missing `type_id`; {SHAPE}")),
        (_, None) => Err(format!("missing `aliases`; {SHAPE}")),
    }
}

/// A `type_id` field's text as the id it names, or why it names none.
pub(super) fn type_id(text: &str) -> Result<TypeId, String> {
    text.parse()
        .map_err(|e| format!("type_id {text:?} is not an id: {e}"))
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
