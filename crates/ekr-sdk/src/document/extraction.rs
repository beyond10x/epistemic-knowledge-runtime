//! The extraction document, `ekr.extraction-document/1`, as the SDK reads and writes it
//! (`story:extraction-verb-shares-the-sdk-path`).
//!
//! The engine's reader is `ekr_integrate::ExtractionDocument`, which the SDK may not link: it
//! sits on `ekr-graph`. This is the SDK's own mirror of the same document, as the SDK mirrors the
//! kernel's documents. Its ontology section is the SDK's [`OntologySpec`], read from and written
//! to the document's form: a value type is written as a value type is, `value_kind` and
//! `parameters` — `{variants: [...]}` for an `Enum`, `{allowed_types: [<node type name>, ...]}`
//! for a `NodeRef`, the element type for a `List` and the fields for a `Record`. A fact is a
//! YAML tag, `!Property` or `!Relation`, and an evidence item is the [`EvidenceAddition`] an
//! `!AddEvidence` carries.
//!
//! [`ExtractionDocument::from_yaml`] refuses what the engine's reader refuses while decoding, by the
//! reader's codes and through the same bounded loader (`ekr_core::decode::observe_yaml`): more than
//! [`EXTRACTION_INPUT_BYTES`] bytes, containers nested deeper than [`EXTRACTION_DEPTH`], a YAML alias,
//! an alias that is not a YAML string, anything but one document of the format. It then makes the
//! reader's checks of the document alone ([`ExtractionDocument::check`]): two evidence items under
//! one id, a payload that does not hash to its entry. What the document names against a store is
//! checked by `ekr apply-extraction` before it applies anything, and by
//! [`crate::extraction::apply`] before it writes anything.

use std::collections::BTreeMap;

use ekr_core::EvidenceId;
use serde::{Deserialize, Serialize};

use super::ontology::{EdgeTypeSpec, NodeTypeSpec, OntologySpec, PropertySpec, ValueSpec};
use super::transaction::EvidenceAddition;
use super::value::{Cardinality, Value};
use super::DocumentError;

/// The exact `format` of an extraction document.
pub const EXTRACTION_FORMAT: &str = "ekr.extraction-document/1";

/// The most bytes of an extraction document read: the engine reader's limit.
pub const EXTRACTION_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// The deepest container nesting an extraction document may have, its own mapping counting one:
/// the engine reader's limit.
pub const EXTRACTION_DEPTH: usize = 32;

/// The engine reader's codes (`ekr.integrate.ExtractionRefusalCode`) the SDK refuses with.
pub(crate) mod code {
    pub(crate) const TOO_LARGE: &str = "extraction-document-too-large";
    pub(crate) const TOO_DEEP: &str = "extraction-document-too-deep";
    pub(crate) const YAML_ALIAS: &str = "extraction-yaml-alias";
    pub(crate) const MALFORMED: &str = "extraction-document-malformed";
    pub(crate) const WITHOUT_IDENTITY: &str = "reference-without-identity";
    pub(crate) const HAS_SUBTYPES: &str = "reference-type-has-subtypes";
    pub(crate) const DUPLICATE_IDENTITY: &str = "duplicate-identity";
    pub(crate) const PAYLOAD_MISMATCH: &str = "evidence-payload-mismatch";
}

/// The one format the mirror reads.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtractionFormat {
    /// `ekr.extraction-document/1`.
    #[default]
    #[serde(rename = "ekr.extraction-document/1")]
    V1,
}

/// A named thing: its node type's name and the names it is known by.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractedReference {
    /// The name of the node type it is an instance of.
    pub node_type: String,
    /// The names it is known by, compared byte for byte; each a YAML string.
    #[serde(deserialize_with = "ekr_core::decode::strings")]
    pub aliases: Vec<String>,
}

impl ExtractedReference {
    /// A thing of the node type named `node_type`, known by `aliases`.
    #[must_use]
    pub fn new<S: Into<String>>(
        node_type: impl Into<String>,
        aliases: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            node_type: node_type.into(),
            aliases: aliases.into_iter().map(Into::into).collect(),
        }
    }
}

/// `!Property`: a property value read about a named thing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyFact {
    /// The thing.
    pub subject: ExtractedReference,
    /// The property's name, declared on the subject's type or an ancestor.
    pub property: String,
    /// The value.
    pub value: Value,
    /// The ids of the evidence items it rests on.
    #[serde(default)]
    pub evidence: Vec<EvidenceId>,
}

/// `!Relation`: a relation read between two named things, by edge type name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelationFact {
    /// Where the relation starts.
    pub subject: ExtractedReference,
    /// The edge type's name.
    pub relation: String,
    /// Where it ends.
    pub object: ExtractedReference,
    /// The ids of the evidence items it rests on.
    #[serde(default)]
    pub evidence: Vec<EvidenceId>,
}

/// A fact, written as a YAML tag.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExtractedFact {
    /// `!Property`.
    Property(PropertyFact),
    /// `!Relation`.
    Relation(RelationFact),
}

impl ExtractedFact {
    /// The ids it cites.
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceId] {
        match self {
            Self::Property(fact) => &fact.evidence,
            Self::Relation(fact) => &fact.evidence,
        }
    }
}

/// `ekr.extraction-document/1`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionDocument {
    /// Always [`ExtractionFormat::V1`].
    pub format: ExtractionFormat,
    /// The types it needs that the store may lack.
    #[serde(default, with = "wire")]
    pub ontology: OntologySpec,
    /// The named things it found, facts or none.
    #[serde(default)]
    pub entities: Vec<ExtractedReference>,
    /// What it read about them.
    pub facts: Vec<ExtractedFact>,
    /// What the facts rest on: each the entry and bytes an `!AddEvidence` carries.
    pub evidence: Vec<EvidenceAddition>,
}

/// Why an extraction document was refused, as the engine's reader refuses it
/// (`ekr.integrate.ExtractionRefusal`): the reader's code and what the refused part names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{code}: {name}")]
pub struct ExtractionRefusal {
    /// The code, as `ekr.integrate.ExtractionRefusalCode` spells it.
    pub code: String,
    /// The type, item or evidence id refused, or for a document refusal the reason.
    pub name: String,
}

impl ExtractionRefusal {
    pub(crate) fn new(code: &str, name: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            name: name.into(),
        }
    }
}

impl ExtractionDocument {
    /// A document with nothing in it.
    #[must_use]
    pub fn new() -> Self {
        Self {
            format: ExtractionFormat::V1,
            ontology: OntologySpec::default(),
            entities: Vec::new(),
            facts: Vec::new(),
            evidence: Vec::new(),
        }
    }

    /// Decodes one document, refusing what the engine's reader refuses while decoding, then
    /// [`Self::check`]s it.
    ///
    /// # Errors
    /// [`ExtractionRefusal`], with the reader's code.
    pub fn from_yaml(text: &str) -> Result<Self, ExtractionRefusal> {
        use ekr_core::decode::{observe_yaml, YamlRefusal};

        observe_yaml(text, EXTRACTION_INPUT_BYTES, EXTRACTION_DEPTH).map_err(|bound| {
            let code = match bound {
                YamlRefusal::TooLarge { .. } => code::TOO_LARGE,
                YamlRefusal::TooDeep { .. } => code::TOO_DEEP,
                YamlRefusal::Alias { .. } => code::YAML_ALIAS,
                YamlRefusal::Malformed(_) => code::MALFORMED,
            };
            ExtractionRefusal::new(code, bound.to_string())
        })?;
        let document: Self = serde_yaml_ng::from_str(text)
            .map_err(|error| ExtractionRefusal::new(code::MALFORMED, error.to_string()))?;
        document.check()?;
        Ok(document)
    }

    /// The engine reader's checks of the document alone, in its order: each evidence item's id
    /// held by no earlier item (`duplicate-identity`) and its payload hashing to its entry's
    /// `content_hash` (`evidence-payload-mismatch`).
    ///
    /// # Errors
    /// The first [`ExtractionRefusal`], naming the evidence id.
    pub fn check(&self) -> Result<(), ExtractionRefusal> {
        let mut ids = std::collections::BTreeSet::new();
        for item in &self.evidence {
            let id = item.evidence.id;
            if !ids.insert(id) {
                return Err(ExtractionRefusal::new(
                    code::DUPLICATE_IDENTITY,
                    id.to_string(),
                ));
            }
            if ekr_core::ContentHash::of_bytes(&item.payload) != item.evidence.content_hash {
                return Err(ExtractionRefusal::new(
                    code::PAYLOAD_MISMATCH,
                    id.to_string(),
                ));
            }
        }
        Ok(())
    }

    /// The document as YAML, facts as tags.
    ///
    /// # Errors
    /// [`DocumentError::Yaml`] when the writer refuses a value.
    pub fn to_yaml(&self) -> Result<String, DocumentError> {
        super::to_yaml(self)
    }
}

impl Default for ExtractionDocument {
    fn default() -> Self {
        Self::new()
    }
}

/// The ontology section in the document's form, read into and written from [`OntologySpec`].
mod wire {
    use super::{
        BTreeMap, Cardinality, Deserialize, EdgeTypeSpec, NodeTypeSpec, OntologySpec, PropertySpec,
        Serialize, ValueSpec,
    };

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "value_kind", content = "parameters", deny_unknown_fields)]
    enum Value {
        String,
        Boolean,
        Integer,
        Float,
        Decimal,
        Timestamp,
        Duration,
        NodeRef { allowed_types: Vec<String> },
        Enum { variants: Vec<String> },
        List(Box<Value>),
        Record(BTreeMap<String, Value>),
    }

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Property {
        name: String,
        value: Value,
        #[serde(default)]
        cardinality: Cardinality,
        #[serde(default)]
        required: bool,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct NodeType {
        name: String,
        #[serde(default)]
        parents: Vec<String>,
        #[serde(default)]
        abstract_type: bool,
        #[serde(default)]
        properties: Vec<Property>,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct EdgeType {
        name: String,
        source_types: Vec<String>,
        target_types: Vec<String>,
        #[serde(default)]
        cardinality: Cardinality,
        #[serde(default)]
        properties: Vec<Property>,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Ontology {
        #[serde(default)]
        node_types: Vec<NodeType>,
        #[serde(default)]
        edge_types: Vec<EdgeType>,
    }

    impl From<Value> for ValueSpec {
        fn from(value: Value) -> Self {
            match value {
                Value::String => Self::String,
                Value::Boolean => Self::Boolean,
                Value::Integer => Self::Integer,
                Value::Float => Self::Float,
                Value::Decimal => Self::Decimal,
                Value::Timestamp => Self::Timestamp,
                Value::Duration => Self::Duration,
                Value::NodeRef { allowed_types } => Self::NodeRef(allowed_types),
                Value::Enum { variants } => Self::Enum(variants),
                Value::List(element) => Self::List(Box::new((*element).into())),
                Value::Record(fields) => Self::Record(
                    fields
                        .into_iter()
                        .map(|(field, value)| (field, value.into()))
                        .collect(),
                ),
            }
        }
    }

    impl From<&ValueSpec> for Value {
        fn from(value: &ValueSpec) -> Self {
            match value {
                ValueSpec::String => Self::String,
                ValueSpec::Boolean => Self::Boolean,
                ValueSpec::Integer => Self::Integer,
                ValueSpec::Float => Self::Float,
                ValueSpec::Decimal => Self::Decimal,
                ValueSpec::Timestamp => Self::Timestamp,
                ValueSpec::Duration => Self::Duration,
                ValueSpec::NodeRef(allowed_types) => Self::NodeRef {
                    allowed_types: allowed_types.clone(),
                },
                ValueSpec::Enum(variants) => Self::Enum {
                    variants: variants.clone(),
                },
                ValueSpec::List(element) => Self::List(Box::new(element.as_ref().into())),
                ValueSpec::Record(fields) => Self::Record(
                    fields
                        .iter()
                        .map(|(field, value)| (field.clone(), value.into()))
                        .collect(),
                ),
            }
        }
    }

    impl From<Property> for PropertySpec {
        fn from(property: Property) -> Self {
            Self {
                name: property.name,
                value: property.value.into(),
                cardinality: property.cardinality,
                required: property.required,
            }
        }
    }

    impl From<&PropertySpec> for Property {
        fn from(property: &PropertySpec) -> Self {
            Self {
                name: property.name.clone(),
                value: (&property.value).into(),
                cardinality: property.cardinality,
                required: property.required,
            }
        }
    }

    pub(super) fn serialize<S: serde::Serializer>(
        spec: &OntologySpec,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Ontology {
            node_types: spec
                .node_types
                .iter()
                .map(|node| NodeType {
                    name: node.name.clone(),
                    parents: node.parents.clone(),
                    abstract_type: node.abstract_type,
                    properties: node.properties.iter().map(Property::from).collect(),
                })
                .collect(),
            edge_types: spec
                .edge_types
                .iter()
                .map(|edge| EdgeType {
                    name: edge.name.clone(),
                    source_types: edge.source_types.clone(),
                    target_types: edge.target_types.clone(),
                    cardinality: edge.cardinality,
                    properties: edge.properties.iter().map(Property::from).collect(),
                })
                .collect(),
        }
        .serialize(serializer)
    }

    pub(super) fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<OntologySpec, D::Error> {
        let ontology = Ontology::deserialize(deserializer)?;
        Ok(OntologySpec {
            node_types: ontology
                .node_types
                .into_iter()
                .map(|node| NodeTypeSpec {
                    name: node.name,
                    parents: node.parents,
                    abstract_type: node.abstract_type,
                    properties: node.properties.into_iter().map(Into::into).collect(),
                })
                .collect(),
            edge_types: ontology
                .edge_types
                .into_iter()
                .map(|edge| EdgeTypeSpec {
                    name: edge.name,
                    source_types: edge.source_types,
                    target_types: edge.target_types,
                    cardinality: edge.cardinality,
                    properties: edge.properties.into_iter().map(Into::into).collect(),
                })
                .collect(),
        })
    }
}
