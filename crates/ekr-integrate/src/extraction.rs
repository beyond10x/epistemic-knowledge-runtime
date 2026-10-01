//! The extraction document, `ekr.extraction-document/1`: what an extracting agent hands the
//! engine, and its reader.
//!
//! Implements `ekr.integrate.ExtractionDocument` and the types around it
//! (`systems/ekr/domains/integrate.yaml`, design § 24 and § 63). A consumer's extractor, run by the
//! consumer in its own sandbox, reads a source and writes one document: the ontology it needs, by
//! name ([`OntologySpec`]); the named things it found ([`ExtractedReference`]); the facts it read
//! about them ([`ExtractedFact`]); and the evidence each fact cites ([`ExtractionEvidence`], the
//! entry and bytes an `AddEvidence` carries). The engine starts no agent.
//!
//! A name is not an identity (`AGENTS.md` invariant 3). Types, properties and relations are named
//! here because an extractor knows them by name; applying the document maps each name to the id
//! the store holds, or mints one for what the document declares and the store lacks, and resolves
//! each named thing as an [`crate::TypedReference`] of that type before any `CreateNode`.
//!
//! # The reader
//!
//! [`read_extraction`] decodes a document ([`ExtractionDocument::from_yaml`]) and checks it against
//! the ontology of the store it is for ([`ExtractionDocument::check`]). Decoding refuses, as
//! [`ExtractionError::Document`]: more than [`EXTRACTION_INPUT_BYTES`] bytes; a YAML alias
//! (`*name`), which repeats its anchor's value on every use; any format but
//! [`EXTRACTION_FORMAT`]; an unknown or missing field; a fact written other than as a `!Property`
//! or `!Relation` tag; and a second document. Checking refuses, as a named
//! [`ExtractionRefusal`] and in document order, the first of:
//!
//! * `extraction-type-undeclared` — a node type or edge type that neither the document's ontology
//!   nor the store declares, named by a parent, an edge type's end, a `NodeRef`, a named thing, a
//!   fact's subject or object, or a relation;
//! * `extraction-property-undeclared` — a property fact's property that neither declares on the
//!   subject's type or any of its ancestors;
//! * `fact-without-evidence` — a fact citing no evidence item;
//! * `fact-evidence-unlisted` — a fact citing an id no evidence item of the document carries.
//!
//! Nothing here writes: AGENTS.md invariant 1.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ekr_core::EvidenceId;
use ekr_graph::Evidence;
use ekr_ontology::{Cardinality, Ontology, Value};
use serde::{Deserialize, Serialize};

/// The exact `format` of an extraction document.
pub const EXTRACTION_FORMAT: &str = "ekr.extraction-document/1";

/// The most bytes of an extraction document read: the input limit of an
/// `ekr.transaction-document/2`.
pub const EXTRACTION_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// `ekr.integrate.ExtractionFormat`: the one format this reader reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExtractionFormat {
    /// `ekr.extraction-document/1`.
    #[serde(rename = "ekr.extraction-document/1")]
    V1,
}

/// `ekr.integrate.ValueSpec`: a value type with node types named rather than identified, written
/// as a value type is (`value_kind`, and `parameters` for the kinds that take one).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "value_kind", content = "parameters", deny_unknown_fields)]
pub enum ValueSpec {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A whole number.
    Integer,
    /// An approximate number: declarable, never committed as a value.
    Float,
    /// An exact number.
    Decimal,
    /// A point in time.
    Timestamp,
    /// A length of time.
    Duration,
    /// A reference to a node of one of the node types of these names.
    NodeRef(Vec<String>),
    /// One of these variants.
    Enum(Vec<String>),
    /// A list whose every element has this type.
    List(Box<ValueSpec>),
    /// These named fields, each of its own type.
    Record(BTreeMap<String, ValueSpec>),
}

/// `ekr.integrate.PropertySpec`: a property by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PropertySpec {
    /// Its name, unique on the type that declares it.
    pub name: String,
    /// Its value type.
    pub value: ValueSpec,
    /// How many values a node may carry; `One` when absent.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// Whether a node must carry one; `false` when absent.
    #[serde(default)]
    pub required: bool,
}

/// `ekr.integrate.NodeTypeSpec`: a node type by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct NodeTypeSpec {
    /// Its name, unique among node types.
    pub name: String,
    /// The node types it specialises, by name.
    #[serde(default)]
    pub parents: Vec<String>,
    /// Whether it is abstract.
    #[serde(default)]
    pub abstract_type: bool,
    /// The properties it declares itself.
    #[serde(default)]
    pub properties: Vec<PropertySpec>,
}

/// `ekr.integrate.EdgeTypeSpec`: an edge type by name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct EdgeTypeSpec {
    /// Its name, unique among edge types.
    pub name: String,
    /// The node types an edge may start at, by name.
    pub source_types: Vec<String>,
    /// The node types an edge may end at, by name.
    pub target_types: Vec<String>,
    /// How many edges may leave one node; `One` when absent.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// The properties an edge carries.
    #[serde(default)]
    pub properties: Vec<PropertySpec>,
}

/// `ekr.integrate.OntologySpec`: the ontology a document needs, by name.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct OntologySpec {
    /// Its node types.
    #[serde(default)]
    pub node_types: Vec<NodeTypeSpec>,
    /// Its edge types.
    #[serde(default)]
    pub edge_types: Vec<EdgeTypeSpec>,
}

/// `ekr.integrate.ExtractedReference`: a named thing, by its type's name and its aliases.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExtractedReference {
    /// The name of the node type it is an instance of.
    pub node_type: String,
    /// The names it is known by, each compared byte for byte.
    pub aliases: Vec<String>,
}

/// `ekr.integrate.PropertyFact`: a property value read about a named thing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct PropertyFact {
    /// The thing.
    pub subject: ExtractedReference,
    /// The property's name, declared on the subject's type or an ancestor.
    pub property: String,
    /// The value.
    pub value: Value,
    /// The ids of the evidence items it rests on; at least one.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub evidence: Vec<EvidenceId>,
}

/// `ekr.integrate.RelationFact`: a relation read between two named things.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct RelationFact {
    /// Where the relation starts.
    pub subject: ExtractedReference,
    /// The edge type's name.
    pub relation: String,
    /// Where it ends.
    pub object: ExtractedReference,
    /// The ids of the evidence items it rests on; at least one.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
    pub evidence: Vec<EvidenceId>,
}

/// `ekr.integrate.ExtractedFact`, written as a YAML tag: `!Property` or `!Relation`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExtractedFact {
    /// `!Property`.
    #[cfg_attr(feature = "schema", schemars(rename = "!Property"))]
    Property(PropertyFact),
    /// `!Relation`.
    #[cfg_attr(feature = "schema", schemars(rename = "!Relation"))]
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

/// One evidence item: the entry and the exact bytes its `content_hash` addresses, as an
/// `AddEvidence` carries them (`ekr.kernel.EvidenceAdditionProjection`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExtractionEvidence {
    /// The entry, under its own id, which facts cite.
    pub evidence: Evidence,
    /// Its bytes, as a list of byte values.
    pub payload: Vec<u8>,
}

/// `ekr.integrate.ExtractionDocument`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct ExtractionDocument {
    /// Exactly [`EXTRACTION_FORMAT`].
    pub format: ExtractionFormat,
    /// The types it needs that the store may lack; empty when absent.
    #[serde(default)]
    pub ontology: OntologySpec,
    /// The named things it found, facts or none.
    #[serde(default)]
    pub entities: Vec<ExtractedReference>,
    /// What it read about them.
    pub facts: Vec<ExtractedFact>,
    /// What the facts rest on.
    pub evidence: Vec<ExtractionEvidence>,
}

/// `ekr.integrate.ExtractionRefusalCode`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExtractionRefusalCode {
    /// A node or edge type no declaration of the document or the store gives.
    #[serde(rename = "extraction-type-undeclared")]
    ExtractionTypeUndeclared,
    /// A property the subject's type and its ancestors do not declare.
    #[serde(rename = "extraction-property-undeclared")]
    ExtractionPropertyUndeclared,
    /// A fact citing no evidence item.
    #[serde(rename = "fact-without-evidence")]
    FactWithoutEvidence,
    /// A fact citing an id no evidence item of the document carries.
    #[serde(rename = "fact-evidence-unlisted")]
    FactEvidenceUnlisted,
}

impl ExtractionRefusalCode {
    /// The code as the domain spells it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExtractionTypeUndeclared => "extraction-type-undeclared",
            Self::ExtractionPropertyUndeclared => "extraction-property-undeclared",
            Self::FactWithoutEvidence => "fact-without-evidence",
            Self::FactEvidenceUnlisted => "fact-evidence-unlisted",
        }
    }
}

/// `ekr.integrate.ExtractionRefusal`: a code, and what the refused part names — the type,
/// `Type.property`, or `facts[<index>]` and, for an unlisted id, `: <id>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractionRefusal {
    /// Why.
    pub code: ExtractionRefusalCode,
    /// What.
    pub name: String,
}

impl fmt::Display for ExtractionRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let what = match self.code {
            ExtractionRefusalCode::ExtractionTypeUndeclared => {
                "no node type or edge type of the document's ontology or the store's is named"
            }
            ExtractionRefusalCode::ExtractionPropertyUndeclared => {
                "the subject's type and its ancestors declare no property"
            }
            ExtractionRefusalCode::FactWithoutEvidence => "cites no evidence item:",
            ExtractionRefusalCode::FactEvidenceUnlisted => {
                "cites an evidence id the document's evidence does not hold:"
            }
        };
        write!(f, "{}: {what} {}", self.code.code(), self.name)
    }
}

/// Why a document was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractionError {
    /// It is not an extraction document: the text says why.
    Document(String),
    /// It is one, and names something the store cannot take.
    Refused(ExtractionRefusal),
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Document(error) => write!(f, "not an {EXTRACTION_FORMAT} document: {error}"),
            Self::Refused(refusal) => refusal.fmt(f),
        }
    }
}

impl std::error::Error for ExtractionError {}

/// Decodes `text` and checks it against `store`, the ontology of the store it is for.
///
/// # Errors
/// See the [module](self) documentation.
pub fn read_extraction(
    text: &str,
    store: &Ontology,
) -> Result<ExtractionDocument, ExtractionError> {
    let document = ExtractionDocument::from_yaml(text)?;
    document.check(store).map_err(ExtractionError::Refused)?;
    Ok(document)
}

impl ExtractionDocument {
    /// Decodes one document. Refuses what the [module](self) documentation lists under decoding.
    ///
    /// # Errors
    /// [`ExtractionError::Document`].
    pub fn from_yaml(text: &str) -> Result<Self, ExtractionError> {
        use serde_yaml_ng::observation::{Documents, Event};

        let fault = |error: &dyn fmt::Display| ExtractionError::Document(error.to_string());
        if text.len() > EXTRACTION_INPUT_BYTES {
            return Err(fault(&format_args!("over {EXTRACTION_INPUT_BYTES} bytes")));
        }
        let mut documents = Documents::from_str(text).map_err(|e| fault(&e))?;
        while let Some(document) = documents.next_document() {
            document.check().map_err(|e| fault(&e))?;
            for at in 0..document.event_count() {
                if let Some(Event::Alias { .. }) = document.event(at).map_err(|e| fault(&e))? {
                    return Err(fault(
                        &"a YAML alias (`*name`) is not accepted: write each value out",
                    ));
                }
            }
        }
        serde_yaml_ng::from_str(text).map_err(|e| fault(&e))
    }

    /// Checks every name against this document's ontology and `store`'s, and every fact's
    /// evidence against this document's, in document order.
    ///
    /// # Errors
    /// The first [`ExtractionRefusal`] the [module](self) documentation lists under checking.
    pub fn check(&self, store: &Ontology) -> Result<(), ExtractionRefusal> {
        let names = Names::of(&self.ontology, store);
        let refuse = |code, name: String| Err(ExtractionRefusal { code, name });
        let node_type = |name: &str| {
            if names.nodes.contains_key(name) {
                Ok(())
            } else {
                refuse(
                    ExtractionRefusalCode::ExtractionTypeUndeclared,
                    name.to_owned(),
                )
            }
        };

        for declared in &self.ontology.node_types {
            declared
                .parents
                .iter()
                .try_for_each(|parent| node_type(parent))?;
            for property in &declared.properties {
                value_node_types(&property.value, &node_type)?;
            }
        }
        for declared in &self.ontology.edge_types {
            declared
                .source_types
                .iter()
                .chain(&declared.target_types)
                .try_for_each(|end| node_type(end))?;
            for property in &declared.properties {
                value_node_types(&property.value, &node_type)?;
            }
        }
        for entity in &self.entities {
            node_type(&entity.node_type)?;
        }
        let listed: BTreeSet<EvidenceId> =
            self.evidence.iter().map(|item| item.evidence.id).collect();
        for (at, fact) in self.facts.iter().enumerate() {
            match fact {
                ExtractedFact::Property(fact) => {
                    node_type(&fact.subject.node_type)?;
                    if !names.declares(&fact.subject.node_type, &fact.property) {
                        return refuse(
                            ExtractionRefusalCode::ExtractionPropertyUndeclared,
                            format!("{}.{}", fact.subject.node_type, fact.property),
                        );
                    }
                }
                ExtractedFact::Relation(fact) => {
                    node_type(&fact.subject.node_type)?;
                    if !names.edges.contains(&fact.relation) {
                        return refuse(
                            ExtractionRefusalCode::ExtractionTypeUndeclared,
                            fact.relation.clone(),
                        );
                    }
                    node_type(&fact.object.node_type)?;
                }
            }
            let cited = fact.evidence();
            if cited.is_empty() {
                return refuse(
                    ExtractionRefusalCode::FactWithoutEvidence,
                    format!("facts[{at}]"),
                );
            }
            if let Some(id) = cited.iter().find(|id| !listed.contains(id)) {
                return refuse(
                    ExtractionRefusalCode::FactEvidenceUnlisted,
                    format!("facts[{at}]: {id}"),
                );
            }
        }
        Ok(())
    }
}

/// Every node type a value type names, checked by `node_type`.
fn value_node_types(
    value: &ValueSpec,
    node_type: &impl Fn(&str) -> Result<(), ExtractionRefusal>,
) -> Result<(), ExtractionRefusal> {
    match value {
        ValueSpec::NodeRef(names) => names.iter().try_for_each(|name| node_type(name)),
        ValueSpec::List(element) => value_node_types(element, node_type),
        ValueSpec::Record(fields) => fields
            .values()
            .try_for_each(|field| value_node_types(field, node_type)),
        _ => Ok(()),
    }
}

/// What a document and a store declare between them, by name.
struct Names {
    /// Each node type's parents and the properties it declares itself, both sides merged.
    nodes: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)>,
    /// Every edge type.
    edges: BTreeSet<String>,
}

impl Names {
    fn of(spec: &OntologySpec, store: &Ontology) -> Self {
        let held = store.to_document();
        let named: BTreeMap<_, _> = held
            .node_types
            .iter()
            .map(|declared| (declared.id, declared.name.clone()))
            .collect();
        let mut nodes: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>)> = BTreeMap::new();
        for declared in &held.node_types {
            let entry = nodes.entry(declared.name.clone()).or_default();
            entry.0.extend(
                declared
                    .parents
                    .iter()
                    .filter_map(|id| named.get(id).cloned()),
            );
            entry
                .1
                .extend(declared.properties.values().map(|p| p.name.clone()));
        }
        for declared in &spec.node_types {
            let entry = nodes.entry(declared.name.clone()).or_default();
            entry.0.extend(declared.parents.iter().cloned());
            entry
                .1
                .extend(declared.properties.iter().map(|p| p.name.clone()));
        }
        let edges = held
            .edge_types
            .iter()
            .map(|declared| declared.name.clone())
            .chain(spec.edge_types.iter().map(|declared| declared.name.clone()))
            .collect();
        Self { nodes, edges }
    }

    /// Whether `node_type` or one of its ancestors declares `property`.
    fn declares(&self, node_type: &str, property: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut open = vec![node_type];
        while let Some(name) = open.pop() {
            if !seen.insert(name) {
                continue;
            }
            let Some((parents, properties)) = self.nodes.get(name) else {
                continue;
            };
            if properties.contains(property) {
                return true;
            }
            open.extend(parents.iter().map(String::as_str));
        }
        false
    }
}
