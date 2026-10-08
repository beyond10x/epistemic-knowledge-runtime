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
//! the ontology of the store it is for ([`ExtractionDocument::check`]). Every refusal is an
//! [`ExtractionRefusal`] with a code. Decoding refuses, as [`ExtractionError::Document`]:
//!
//! * `extraction-document-too-large` — more than [`EXTRACTION_INPUT_BYTES`] bytes;
//! * `extraction-document-too-deep` — containers nested deeper than [`EXTRACTION_DEPTH`], the
//!   transaction reader's limit. The YAML loader stops at the first container past it, so a deep
//!   nesting costs what the limit allows, not what the input holds;
//! * `extraction-yaml-alias` — a YAML alias (`*name`), which repeats its anchor's value;
//! * `extraction-document-malformed` — anything but one document of the format: another format,
//!   an unknown or missing field, a mapping key written twice, a fact written other than as a
//!   `!Property` or `!Relation` tag, an alias that is not a YAML string.
//!
//! Checking refuses, as [`ExtractionError::Refused`] and in document order, the first of:
//!
//! * `extraction-name-duplicate` — a node type, edge type, property of one type, `Enum` variant
//!   or `NodeRef` type the document declares twice;
//! * `extraction-type-conflict` — a type the store holds, redeclared with other parents or another
//!   abstractness (a node type) or another cardinality (an edge type), which no schema operation
//!   changes;
//! * `extraction-type-undeclared` — a node type or edge type that neither the document's ontology
//!   nor the store declares, named by a parent, an edge type's end, a `NodeRef`, a named thing, a
//!   fact's subject or object, or a relation;
//! * `extraction-property-conflict` — a property a node type of the document declares that one of
//!   its ancestors already declares with another value type, cardinality, requiredness or
//!   constraints: the property is the ancestor's, and no schema operation lets a subtype change
//!   it, so declare it alike or change it on the ancestor;
//! * `extraction-value-type-empty` — an `Enum` with no variant or a `NodeRef` to no node type;
//! * `reference-without-identity` — a named thing, or a fact's subject or object, with no alias
//!   but the empty string;
//! * `reference-type-has-subtypes` — a named thing, or a fact's subject or object, whose node type
//!   is abstract or has a subtype once the document's ontology is applied: a typed reference to it
//!   would match no one type of node, so `ekr resolve` refuses it, and the whole document is
//!   refused before anything is written;
//! * `extraction-property-undeclared` — a property fact's property that neither declares on the
//!   subject's type or any of its ancestors;
//! * `extraction-value-mismatch` — a property fact's value its property's type does not hold (any
//!   `Float`, which is never committed);
//! * `extraction-relation-ends` — a relation whose subject or object is not of a node type, or a
//!   subtype of one, at that end of its edge type;
//! * `fact-without-evidence` — a fact citing no evidence item;
//! * `fact-evidence-unlisted` — a fact citing an id no evidence item of the document carries;
//! * `duplicate-identity` — two evidence items under one id;
//! * `extraction-evidence-kind-unsupported` — an evidence item whose source is not a
//!   `!HumanStatement`, the one source the kernel admits as evidence today;
//! * `evidence-payload-mismatch` — an evidence item whose payload does not hash to its entry.
//!
//! [`ExtractionDocument::check_facts`] makes the same checks a fact at a time
//! (`story:extraction-partial-apply`): a code from `reference-without-identity` to
//! `fact-evidence-unlisted` met in `facts[<index>]` refuses that fact alone, and the rest of the
//! document is checked on; one met in the ontology, in `entities` or in an evidence item refuses
//! the document whole. `ekr apply-extraction` reads a document so unless it is run `--strict`.
//!
//! Nothing here writes: AGENTS.md invariant 1.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ekr_core::{ContentHash, EvidenceId, TypeId};
use ekr_graph::{Evidence, EvidenceSource};
use ekr_ontology::{Cardinality, Ontology, Value, ValueType};
use serde::{Deserialize, Serialize};

/// The exact `format` of an extraction document.
pub const EXTRACTION_FORMAT: &str = "ekr.extraction-document/1";

/// The most bytes of an extraction document read: the input limit of an
/// `ekr.transaction-document/2`.
pub const EXTRACTION_INPUT_BYTES: usize = 8 * 1024 * 1024;

/// The deepest container nesting an extraction document may have: the depth limit of an
/// `ekr.transaction-document/2`, the document's own mapping counting one.
pub const EXTRACTION_DEPTH: usize = 32;

/// `ekr.integrate.ExtractionFormat`: the one format this reader reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum ExtractionFormat {
    /// `ekr.extraction-document/1`.
    #[serde(rename = "ekr.extraction-document/1")]
    V1,
}

/// `ekr.integrate.ValueSpec`: a value type with node types named rather than identified, written
/// as a value type is: `value_kind`, and `parameters` for the kinds that take one —
/// `{allowed_types: [<node type name>, ...]}` for a `NodeRef`, `{variants: [...]}` for an `Enum`,
/// the element type for a `List` and the fields for a `Record`.
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
    /// A reference to a node of one of these node types, by name.
    NodeRef {
        /// The node types, by name; at least one, each once.
        #[cfg_attr(
            feature = "schema",
            schemars(length(min = 1), extend("uniqueItems" = true))
        )]
        allowed_types: Vec<String>,
    },
    /// One of these variants.
    Enum {
        /// The variants; at least one, each once.
        #[cfg_attr(
            feature = "schema",
            schemars(length(min = 1), extend("uniqueItems" = true))
        )]
        variants: Vec<String>,
    },
    /// A list whose every element has this type.
    List(Box<ValueSpec>),
    /// These named fields, each of its own type; a field is written once.
    Record(#[serde(deserialize_with = "ekr_core::decode::unique_map")] BTreeMap<String, ValueSpec>),
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
    /// The names it is known by, each a YAML string compared byte for byte, as a
    /// [`crate::TypedReference`]'s are: a null, a boolean or a number is refused.
    #[serde(deserialize_with = "crate::strings")]
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
    /// Whether the value replaces the active value of the same subject and property: applying the
    /// fact supersedes that assertion (`story:extraction-supersession`). `false` when absent, and
    /// then not written.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub replaces: bool,
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

/// `ekr.integrate.ExtractionRefusalCode`: why the reader refused a document.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ExtractionRefusalCode {
    /// More than [`EXTRACTION_INPUT_BYTES`] bytes.
    #[serde(rename = "extraction-document-too-large")]
    ExtractionDocumentTooLarge,
    /// Containers nested deeper than [`EXTRACTION_DEPTH`].
    #[serde(rename = "extraction-document-too-deep")]
    ExtractionDocumentTooDeep,
    /// A YAML alias.
    #[serde(rename = "extraction-yaml-alias")]
    ExtractionYamlAlias,
    /// Not one document of the format.
    #[serde(rename = "extraction-document-malformed")]
    ExtractionDocumentMalformed,
    /// A name the document declares twice.
    #[serde(rename = "extraction-name-duplicate")]
    ExtractionNameDuplicate,
    /// A store type redeclared in a way no schema operation makes.
    #[serde(rename = "extraction-type-conflict")]
    ExtractionTypeConflict,
    /// A node or edge type no declaration of the document or the store gives.
    #[serde(rename = "extraction-type-undeclared")]
    ExtractionTypeUndeclared,
    /// A property a subtype declares that an ancestor declares otherwise.
    #[serde(rename = "extraction-property-conflict")]
    ExtractionPropertyConflict,
    /// An `Enum` with no variant or a `NodeRef` to no node type.
    #[serde(rename = "extraction-value-type-empty")]
    ExtractionValueTypeEmpty,
    /// A reference with no alias but the empty string.
    #[serde(rename = "reference-without-identity")]
    ReferenceWithoutIdentity,
    /// A reference to a node type that is abstract or has a subtype.
    #[serde(rename = "reference-type-has-subtypes")]
    ReferenceTypeHasSubtypes,
    /// A property the subject's type and its ancestors do not declare.
    #[serde(rename = "extraction-property-undeclared")]
    ExtractionPropertyUndeclared,
    /// A property value its property's type does not hold.
    #[serde(rename = "extraction-value-mismatch")]
    ExtractionValueMismatch,
    /// A relation between node types its edge type does not connect.
    #[serde(rename = "extraction-relation-ends")]
    ExtractionRelationEnds,
    /// A fact citing no evidence item.
    #[serde(rename = "fact-without-evidence")]
    FactWithoutEvidence,
    /// A fact citing an id no evidence item of the document carries.
    #[serde(rename = "fact-evidence-unlisted")]
    FactEvidenceUnlisted,
    /// Two evidence items under one id.
    #[serde(rename = "duplicate-identity")]
    DuplicateIdentity,
    /// An evidence item from a source the kernel does not admit.
    #[serde(rename = "extraction-evidence-kind-unsupported")]
    ExtractionEvidenceKindUnsupported,
    /// An evidence payload that does not hash to its entry's `content_hash`.
    #[serde(rename = "evidence-payload-mismatch")]
    EvidencePayloadMismatch,
}

impl ExtractionRefusalCode {
    /// Every code, in the order the domain declares them.
    pub const ALL: [Self; 19] = [
        Self::ExtractionDocumentTooLarge,
        Self::ExtractionDocumentTooDeep,
        Self::ExtractionYamlAlias,
        Self::ExtractionDocumentMalformed,
        Self::ExtractionNameDuplicate,
        Self::ExtractionTypeConflict,
        Self::ExtractionTypeUndeclared,
        Self::ExtractionPropertyConflict,
        Self::ExtractionValueTypeEmpty,
        Self::ReferenceWithoutIdentity,
        Self::ReferenceTypeHasSubtypes,
        Self::ExtractionPropertyUndeclared,
        Self::ExtractionValueMismatch,
        Self::ExtractionRelationEnds,
        Self::FactWithoutEvidence,
        Self::FactEvidenceUnlisted,
        Self::DuplicateIdentity,
        Self::ExtractionEvidenceKindUnsupported,
        Self::EvidencePayloadMismatch,
    ];

    /// The code as the domain spells it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ExtractionDocumentTooLarge => "extraction-document-too-large",
            Self::ExtractionDocumentTooDeep => "extraction-document-too-deep",
            Self::ExtractionYamlAlias => "extraction-yaml-alias",
            Self::ExtractionDocumentMalformed => "extraction-document-malformed",
            Self::ExtractionNameDuplicate => "extraction-name-duplicate",
            Self::ExtractionTypeConflict => "extraction-type-conflict",
            Self::ExtractionTypeUndeclared => "extraction-type-undeclared",
            Self::ExtractionPropertyConflict => "extraction-property-conflict",
            Self::ExtractionValueTypeEmpty => "extraction-value-type-empty",
            Self::ReferenceWithoutIdentity => "reference-without-identity",
            Self::ReferenceTypeHasSubtypes => "reference-type-has-subtypes",
            Self::ExtractionPropertyUndeclared => "extraction-property-undeclared",
            Self::ExtractionValueMismatch => "extraction-value-mismatch",
            Self::ExtractionRelationEnds => "extraction-relation-ends",
            Self::FactWithoutEvidence => "fact-without-evidence",
            Self::FactEvidenceUnlisted => "fact-evidence-unlisted",
            Self::DuplicateIdentity => "duplicate-identity",
            Self::ExtractionEvidenceKindUnsupported => "extraction-evidence-kind-unsupported",
            Self::EvidencePayloadMismatch => "evidence-payload-mismatch",
        }
    }

    /// What the code means, for a reader of the refusal.
    const fn meaning(self) -> &'static str {
        match self {
            Self::ExtractionDocumentTooLarge => "the document is larger than the reader reads:",
            Self::ExtractionDocumentTooDeep => "the document nests deeper than the reader reads:",
            Self::ExtractionYamlAlias => {
                "a YAML alias (`*name`) is not accepted; write each value out:"
            }
            Self::ExtractionDocumentMalformed => "not an ekr.extraction-document/1 document:",
            Self::ExtractionNameDuplicate => "the document declares this name twice:",
            Self::ExtractionTypeConflict => {
                "the store declares this type with other parents, abstractness or cardinality, \
                 which no schema operation changes:"
            }
            Self::ExtractionTypeUndeclared => {
                "no node type or edge type of the document's ontology or the store's is named"
            }
            Self::ExtractionPropertyConflict => {
                "an ancestor declares this property otherwise; declare it alike or change it there:"
            }
            Self::ExtractionValueTypeEmpty => "an Enum with no variant or a NodeRef to no type:",
            Self::ReferenceWithoutIdentity => {
                "the reference holds no alias but the empty string, so nothing identifies it:"
            }
            Self::ReferenceTypeHasSubtypes => {
                "the type is abstract or has a subtype, so a reference to it is not resolved; name \
                 the concrete type:"
            }
            Self::ExtractionPropertyUndeclared => {
                "the subject's type and its ancestors declare no property"
            }
            Self::ExtractionValueMismatch => "the property's type does not hold the value:",
            Self::ExtractionRelationEnds => {
                "the edge type does not connect the subject's and object's node types:"
            }
            Self::FactWithoutEvidence => "cites no evidence item:",
            Self::FactEvidenceUnlisted => {
                "cites an evidence id the document's evidence does not hold:"
            }
            Self::DuplicateIdentity => "two evidence items carry the id",
            Self::ExtractionEvidenceKindUnsupported => {
                "the evidence item's source is not a !HumanStatement, the one source the kernel \
                 admits:"
            }
            Self::EvidencePayloadMismatch => {
                "the payload's bytes do not hash to the content_hash of the evidence item"
            }
        }
    }
}

/// `ekr.integrate.ExtractionRefusal`: a code, and what the refused part names — the type,
/// `Type.property`, `facts[<index>]` or `entities[<index>]` with what it carries, an evidence id,
/// or for a document refusal the reason.
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
        write!(
            f,
            "{}: {} {}",
            self.code.code(),
            self.code.meaning(),
            self.name
        )
    }
}

/// Why a document was not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtractionError {
    /// It is not an extraction document the reader reads; the code says why.
    Document(ExtractionRefusal),
    /// It is one, and names something the store cannot take.
    Refused(ExtractionRefusal),
}

impl ExtractionError {
    /// The refusal, whichever stage made it.
    #[must_use]
    pub const fn refusal(&self) -> &ExtractionRefusal {
        match self {
            Self::Document(refusal) | Self::Refused(refusal) => refusal,
        }
    }
}

impl fmt::Display for ExtractionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.refusal().fmt(f)
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

/// A refusal of `code` naming `name`.
fn refusal(code: ExtractionRefusalCode, name: impl Into<String>) -> ExtractionRefusal {
    ExtractionRefusal {
        code,
        name: name.into(),
    }
}

impl ExtractionDocument {
    /// Decodes one document. Refuses what the [module](self) documentation lists under decoding.
    ///
    /// # Errors
    /// [`ExtractionError::Document`].
    pub fn from_yaml(text: &str) -> Result<Self, ExtractionError> {
        let refused = |code, name: String| ExtractionError::Document(refusal(code, name));
        ekr_core::decode::observe_yaml(text, EXTRACTION_INPUT_BYTES, EXTRACTION_DEPTH)
            .map_err(|bound| refused(observed(&bound), bound.to_string()))?;
        serde_yaml_ng::from_str(text).map_err(|e| {
            refused(
                ExtractionRefusalCode::ExtractionDocumentMalformed,
                e.to_string(),
            )
        })
    }

    /// Checks the document against itself and `store`'s ontology, in document order.
    ///
    /// # Errors
    /// The first [`ExtractionRefusal`] the [module](self) documentation lists under checking.
    pub fn check(&self, store: &Ontology) -> Result<(), ExtractionRefusal> {
        self.checked(store, true).map(drop)
    }

    /// [`Self::check`], a fact at a time (`story:extraction-partial-apply`). What refuses one
    /// fact — a reference of its subject or object, its property, its value, its relation's ends
    /// or the evidence it cites (`reference-without-identity`, `extraction-type-undeclared`,
    /// `reference-type-has-subtypes`, `extraction-property-undeclared`,
    /// `extraction-value-mismatch`, `extraction-relation-ends`, `fact-without-evidence`,
    /// `fact-evidence-unlisted`) — is that fact's refusal, under its index, and the rest of the
    /// document is checked on. What refuses the document — its ontology, a named thing of
    /// `entities` or an evidence item — refuses it whole, as [`Self::check`] does.
    ///
    /// # Errors
    /// The first refusal of the document as a whole, in document order.
    pub fn check_facts(
        &self,
        store: &Ontology,
    ) -> Result<BTreeMap<usize, ExtractionRefusal>, ExtractionRefusal> {
        self.checked(store, false)
    }

    /// The checks of [`Self::check`]; a fact's refusal ends them when `strict`, and is collected
    /// under its index otherwise.
    fn checked(
        &self,
        store: &Ontology,
        strict: bool,
    ) -> Result<BTreeMap<usize, ExtractionRefusal>, ExtractionRefusal> {
        use ExtractionRefusalCode as Code;

        let held = Names::of_store(store);
        let names = held.with(&self.ontology);
        let node_type = |name: &str| {
            if names.nodes.contains_key(name) {
                Ok(())
            } else {
                Err(refusal(Code::ExtractionTypeUndeclared, name))
            }
        };

        let mut declared = BTreeSet::new();
        for spec in &self.ontology.node_types {
            if !declared.insert(spec.name.as_str()) {
                return Err(refusal(Code::ExtractionNameDuplicate, spec.name.as_str()));
            }
            unique_properties(&spec.name, &spec.properties)?;
            if let Some(store_type) = held.nodes.get(&spec.name) {
                let parents: BTreeSet<String> = spec.parents.iter().cloned().collect();
                if parents != store_type.parents || spec.abstract_type != store_type.abstract_type {
                    return Err(refusal(Code::ExtractionTypeConflict, spec.name.as_str()));
                }
            }
            for property in &spec.properties {
                let own = held
                    .nodes
                    .get(&spec.name)
                    .is_some_and(|entry| entry.shapes.contains_key(&property.name));
                let inherited = names
                    .lineage(&spec.name)
                    .into_iter()
                    .skip(1)
                    .find_map(|ancestor| names.nodes.get(ancestor)?.shapes.get(&property.name));
                if !own && inherited.is_some_and(|shape| *shape != Shape::of_spec(property)) {
                    return Err(refusal(
                        Code::ExtractionPropertyConflict,
                        format!("{}.{}", spec.name, property.name),
                    ));
                }
            }
            spec.parents
                .iter()
                .try_for_each(|parent| node_type(parent))?;
            for property in &spec.properties {
                value_spec(
                    &format!("{}.{}", spec.name, property.name),
                    &property.value,
                    &node_type,
                )?;
            }
        }
        let mut declared = BTreeSet::new();
        for spec in &self.ontology.edge_types {
            if !declared.insert(spec.name.as_str()) {
                return Err(refusal(Code::ExtractionNameDuplicate, spec.name.as_str()));
            }
            unique_properties(&spec.name, &spec.properties)?;
            if held
                .edges
                .get(&spec.name)
                .is_some_and(|store_type| store_type.cardinality != spec.cardinality)
            {
                return Err(refusal(Code::ExtractionTypeConflict, spec.name.as_str()));
            }
            spec.source_types
                .iter()
                .chain(&spec.target_types)
                .try_for_each(|end| node_type(end))?;
            for property in &spec.properties {
                value_spec(
                    &format!("{}.{}", spec.name, property.name),
                    &property.value,
                    &node_type,
                )?;
            }
        }

        let reference = |at: String, reference: &ExtractedReference| {
            if reference.aliases.iter().all(String::is_empty) {
                return Err(refusal(Code::ReferenceWithoutIdentity, at));
            }
            node_type(&reference.node_type)?;
            if names.has_subtypes(&reference.node_type) {
                return Err(refusal(
                    Code::ReferenceTypeHasSubtypes,
                    format!("{at}: {}", reference.node_type),
                ));
            }
            Ok(())
        };
        for (at, entity) in self.entities.iter().enumerate() {
            reference(format!("entities[{at}]"), entity)?;
        }

        let listed: BTreeSet<EvidenceId> =
            self.evidence.iter().map(|item| item.evidence.id).collect();
        let check_fact = |at: usize, fact: &ExtractedFact| -> Result<(), ExtractionRefusal> {
            match fact {
                ExtractedFact::Property(fact) => {
                    reference(format!("facts[{at}].subject"), &fact.subject)?;
                    let named = format!("{}.{}", fact.subject.node_type, fact.property);
                    let Some(value_type) = names.property(&fact.subject.node_type, &fact.property)
                    else {
                        return Err(refusal(Code::ExtractionPropertyUndeclared, named));
                    };
                    if !holds(value_type, &fact.value) {
                        return Err(refusal(
                            Code::ExtractionValueMismatch,
                            format!("facts[{at}]: {named}"),
                        ));
                    }
                }
                ExtractedFact::Relation(fact) => {
                    reference(format!("facts[{at}].subject"), &fact.subject)?;
                    let Some(edge) = names.edges.get(&fact.relation) else {
                        return Err(refusal(
                            Code::ExtractionTypeUndeclared,
                            fact.relation.as_str(),
                        ));
                    };
                    reference(format!("facts[{at}].object"), &fact.object)?;
                    let connects = |node: &str, ends: &BTreeSet<String>| {
                        ends.iter().any(|end| names.conforms(node, end))
                    };
                    if !connects(&fact.subject.node_type, &edge.sources)
                        || !connects(&fact.object.node_type, &edge.targets)
                    {
                        return Err(refusal(
                            Code::ExtractionRelationEnds,
                            format!(
                                "facts[{at}]: {} {} {}",
                                fact.subject.node_type, fact.relation, fact.object.node_type
                            ),
                        ));
                    }
                }
            }
            let cited = fact.evidence();
            if cited.is_empty() {
                return Err(refusal(Code::FactWithoutEvidence, format!("facts[{at}]")));
            }
            if let Some(id) = cited.iter().find(|id| !listed.contains(id)) {
                return Err(refusal(
                    Code::FactEvidenceUnlisted,
                    format!("facts[{at}]: {id}"),
                ));
            }
            Ok(())
        };
        let mut refused = BTreeMap::new();
        for (at, fact) in self.facts.iter().enumerate() {
            if let Err(refusal) = check_fact(at, fact) {
                if strict {
                    return Err(refusal);
                }
                refused.insert(at, refusal);
            }
        }

        let mut ids = BTreeSet::new();
        for item in &self.evidence {
            let id = item.evidence.id;
            if !ids.insert(id) {
                return Err(refusal(Code::DuplicateIdentity, id.to_string()));
            }
            if !matches!(item.evidence.source, EvidenceSource::HumanStatement { .. }) {
                return Err(refusal(
                    Code::ExtractionEvidenceKindUnsupported,
                    id.to_string(),
                ));
            }
            if ContentHash::of_bytes(&item.payload) != item.evidence.content_hash {
                return Err(refusal(Code::EvidencePayloadMismatch, id.to_string()));
            }
        }
        Ok(refused)
    }
}

/// The reader's code for what the bounded observation refused.
const fn observed(bound: &ekr_core::decode::YamlRefusal) -> ExtractionRefusalCode {
    use ekr_core::decode::YamlRefusal;
    match bound {
        YamlRefusal::TooLarge { .. } => ExtractionRefusalCode::ExtractionDocumentTooLarge,
        YamlRefusal::TooDeep { .. } => ExtractionRefusalCode::ExtractionDocumentTooDeep,
        YamlRefusal::Alias { .. } => ExtractionRefusalCode::ExtractionYamlAlias,
        YamlRefusal::Malformed(_) => ExtractionRefusalCode::ExtractionDocumentMalformed,
    }
}

/// Refuses a property name `owner` declares twice.
fn unique_properties(owner: &str, properties: &[PropertySpec]) -> Result<(), ExtractionRefusal> {
    let mut seen = BTreeSet::new();
    match properties
        .iter()
        .find(|property| !seen.insert(property.name.as_str()))
    {
        Some(property) => Err(refusal(
            ExtractionRefusalCode::ExtractionNameDuplicate,
            format!("{owner}.{}", property.name),
        )),
        None => Ok(()),
    }
}

/// Checks a declared value type at `site`: its lists not empty and without a repeat, and every
/// node type it names, by `node_type`.
fn value_spec(
    site: &str,
    value: &ValueSpec,
    node_type: &impl Fn(&str) -> Result<(), ExtractionRefusal>,
) -> Result<(), ExtractionRefusal> {
    let listed = |names: &[String]| {
        if names.is_empty() {
            return Err(refusal(
                ExtractionRefusalCode::ExtractionValueTypeEmpty,
                site,
            ));
        }
        let mut seen = BTreeSet::new();
        match names.iter().find(|name| !seen.insert(name.as_str())) {
            Some(name) => Err(refusal(
                ExtractionRefusalCode::ExtractionNameDuplicate,
                format!("{site}: {name}"),
            )),
            None => Ok(()),
        }
    };
    match value {
        ValueSpec::NodeRef { allowed_types } => {
            listed(allowed_types)?;
            allowed_types.iter().try_for_each(|name| node_type(name))
        }
        ValueSpec::Enum { variants } => listed(variants),
        ValueSpec::List(element) => value_spec(site, element, node_type),
        ValueSpec::Record(fields) => fields.iter().try_for_each(|(field, value)| {
            value_spec(&format!("{site}.{field}"), value, node_type)
        }),
        _ => Ok(()),
    }
}

/// Whether a value of `value_type` can be `value`. A `NodeRef` names a node by id, which only the
/// store resolves, so its kind is what is checked here; a `Float` is never held.
fn holds(value_type: &ValueSpec, value: &Value) -> bool {
    match (value_type, value) {
        (ValueSpec::String, Value::String(_))
        | (ValueSpec::Boolean, Value::Boolean(_))
        | (ValueSpec::Integer, Value::Integer(_))
        | (ValueSpec::Decimal, Value::Decimal(_))
        | (ValueSpec::Timestamp, Value::Timestamp(_))
        | (ValueSpec::Duration, Value::Duration(_))
        | (ValueSpec::NodeRef { .. }, Value::NodeRef(_)) => true,
        (ValueSpec::Enum { variants }, Value::Enum(variant)) => variants.contains(variant),
        (ValueSpec::List(element), Value::List(items)) => {
            items.iter().all(|item| holds(element, item))
        }
        (ValueSpec::Record(fields), Value::Record(carried)) => {
            fields.len() == carried.len()
                && carried
                    .iter()
                    .all(|(field, value)| fields.get(field).is_some_and(|spec| holds(spec, value)))
        }
        _ => false,
    }
}

/// A node type by name: its parents by name, its abstractness and its own properties' types.
#[derive(Clone, Default)]
struct NodeEntry {
    parents: BTreeSet<String>,
    abstract_type: bool,
    properties: BTreeMap<String, ValueSpec>,
    /// Its own properties' whole declarations, as a schema change compares them.
    shapes: BTreeMap<String, Shape>,
}

/// A property's declaration as `Ontology::ensure` compares two: its value type, cardinality and
/// requiredness, and whether it carries constraints, which a document cannot declare.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shape {
    value: ValueSpec,
    cardinality: Cardinality,
    required: bool,
    constrained: bool,
}

impl Shape {
    fn of_spec(property: &PropertySpec) -> Self {
        Self {
            value: property.value.clone(),
            cardinality: property.cardinality,
            required: property.required,
            constrained: false,
        }
    }
}

/// An edge type by name: its ends by node type name and its cardinality.
#[derive(Clone)]
struct EdgeEntry {
    sources: BTreeSet<String>,
    targets: BTreeSet<String>,
    cardinality: Cardinality,
}

/// An ontology by name.
#[derive(Clone)]
struct Names {
    nodes: BTreeMap<String, NodeEntry>,
    edges: BTreeMap<String, EdgeEntry>,
}

impl Names {
    /// What `store` declares.
    fn of_store(store: &Ontology) -> Self {
        let held = store.to_document();
        let named: BTreeMap<TypeId, String> = held
            .node_types
            .iter()
            .map(|declared| (declared.id, declared.name.clone()))
            .collect();
        let names = |ids: &BTreeSet<TypeId>| -> BTreeSet<String> {
            ids.iter().filter_map(|id| named.get(id).cloned()).collect()
        };
        let nodes = held
            .node_types
            .iter()
            .map(|declared| {
                let entry = NodeEntry {
                    parents: names(&declared.parents),
                    abstract_type: declared.abstract_type,
                    properties: declared
                        .properties
                        .values()
                        .map(|property| {
                            (property.name.clone(), by_name(&property.value_type, &named))
                        })
                        .collect(),
                    shapes: declared
                        .properties
                        .values()
                        .map(|property| {
                            let shape = Shape {
                                value: by_name(&property.value_type, &named),
                                cardinality: property.cardinality,
                                required: property.required,
                                constrained: !property.constraints.is_empty(),
                            };
                            (property.name.clone(), shape)
                        })
                        .collect(),
                };
                (declared.name.clone(), entry)
            })
            .collect();
        let edges = held
            .edge_types
            .iter()
            .map(|declared| {
                let entry = EdgeEntry {
                    sources: names(&declared.source_types),
                    targets: names(&declared.target_types),
                    cardinality: declared.cardinality,
                };
                (declared.name.clone(), entry)
            })
            .collect();
        Self { nodes, edges }
    }

    /// This ontology with `spec` applied: a new type as declared, a held one with the document's
    /// properties declared over its own, and an edge type's ends widened to the document's.
    fn with(&self, spec: &OntologySpec) -> Self {
        let mut next = self.clone();
        for declared in &spec.node_types {
            let entry = next
                .nodes
                .entry(declared.name.clone())
                .or_insert_with(|| NodeEntry {
                    parents: declared.parents.iter().cloned().collect(),
                    abstract_type: declared.abstract_type,
                    properties: BTreeMap::new(),
                    shapes: BTreeMap::new(),
                });
            for property in &declared.properties {
                entry
                    .properties
                    .insert(property.name.clone(), property.value.clone());
                entry
                    .shapes
                    .insert(property.name.clone(), Shape::of_spec(property));
            }
        }
        for declared in &spec.edge_types {
            let entry = next
                .edges
                .entry(declared.name.clone())
                .or_insert_with(|| EdgeEntry {
                    sources: BTreeSet::new(),
                    targets: BTreeSet::new(),
                    cardinality: declared.cardinality,
                });
            entry.sources.extend(declared.source_types.iter().cloned());
            entry.targets.extend(declared.target_types.iter().cloned());
        }
        next
    }

    /// `node_type` and its ancestors, nearest first.
    fn lineage<'a>(&'a self, node_type: &'a str) -> Vec<&'a str> {
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut open = vec![node_type];
        while let Some(name) = open.pop() {
            if !seen.insert(name) {
                continue;
            }
            order.push(name);
            if let Some(entry) = self.nodes.get(name) {
                open.extend(entry.parents.iter().map(String::as_str).rev());
            }
        }
        order
    }

    /// The type of `property` on `node_type`, declared there or on an ancestor.
    fn property(&self, node_type: &str, property: &str) -> Option<&ValueSpec> {
        self.lineage(node_type)
            .into_iter()
            .find_map(|name| self.nodes.get(name)?.properties.get(property))
    }

    /// Whether `node_type` is abstract or another node type descends from it: a typed reference
    /// to it is one `ekr resolve` refuses (`reference-type-has-subtypes`).
    fn has_subtypes(&self, node_type: &str) -> bool {
        self.nodes
            .get(node_type)
            .is_some_and(|entry| entry.abstract_type)
            || self
                .nodes
                .keys()
                .any(|other| other != node_type && self.conforms(other, node_type))
    }

    /// Whether `node_type` is `ancestor` or a descendant of it.
    fn conforms(&self, node_type: &str, ancestor: &str) -> bool {
        self.lineage(node_type).contains(&ancestor)
    }
}

/// A store's value type with its node types named.
fn by_name(value_type: &ValueType, named: &BTreeMap<TypeId, String>) -> ValueSpec {
    match value_type {
        ValueType::String => ValueSpec::String,
        ValueType::Boolean => ValueSpec::Boolean,
        ValueType::Integer => ValueSpec::Integer,
        ValueType::Float => ValueSpec::Float,
        ValueType::Decimal => ValueSpec::Decimal,
        ValueType::Timestamp => ValueSpec::Timestamp,
        ValueType::Duration => ValueSpec::Duration,
        ValueType::NodeRef { allowed_types } => ValueSpec::NodeRef {
            allowed_types: allowed_types
                .iter()
                .map(|id| named.get(id).cloned().unwrap_or_else(|| id.to_string()))
                .collect(),
        },
        ValueType::Enum { variants } => ValueSpec::Enum {
            variants: variants.iter().cloned().collect(),
        },
        ValueType::List(element) => ValueSpec::List(Box::new(by_name(element, named))),
        ValueType::Record(fields) => ValueSpec::Record(
            fields
                .iter()
                .map(|(field, value)| (field.clone(), by_name(value, named)))
                .collect(),
        ),
    }
}
