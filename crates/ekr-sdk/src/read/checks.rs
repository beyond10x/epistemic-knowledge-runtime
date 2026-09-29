//! The store checks `ekr quality`, `ekr rejections` and `ekr code-names`, typed
//! (`docs/cli.md` § `ekr quality`, `ekr rejections`, `ekr code-names`), and their calls on
//! [`Reader`] and [`OneShotReader`].
//!
//! Each type writes back exactly the document it was read from: every field the format carries,
//! and an optional one only when the document had it. `crates/ekr-sdk/tests/check_reads.rs`
//! holds that against every quality and code-names document the `ekr-views` conformance fixture
//! stores render and against real `ekr` output, so a field a format gains without an update
//! here fails there. A reader ignores a field it does not know, so a newer `ekr` does not break
//! an older consumer.

use ekr_core::{AgentId, IssueId, NodeId, Timestamp, TransactionId, TypeId};
use serde::{Deserialize, Serialize};

use super::{Argv, OneShotReader, ReadError, Reader};
use crate::transport::Transport;

// ---- ekr.store-quality/1 ----------------------------------------------------------------------

/// `ekr quality`: the `ekr.store-quality/1` document of one revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreQuality {
    /// The format and the revision read.
    pub meta: QualityMeta,
    /// How the revision's active assertions are evidenced.
    pub assertions: AssertionQuality,
    /// How the revision's property declarations are constrained.
    pub properties: PropertyQuality,
    /// Every name two or more nodes of one type hold, by type id then name.
    pub shared_names: Vec<SharedName>,
    /// The distinct nodes `shared_names` lists.
    pub sharing_nodes: u64,
}

/// The `meta` of `ekr.store-quality/1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualityMeta {
    /// `ekr.store-quality/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
}

/// The `assertions` of `ekr.store-quality/1`. A share is basis points, 10000 times the count
/// divided by `active`, rounded down; `None` when `active` is 0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionQuality {
    /// The assertions whose lifecycle is `Active`.
    pub active: u64,
    /// Of those, the ones citing evidence the store holds with its bytes.
    pub with_evidence: u64,
    /// Of those, the ones citing evidence an `AddEvidence` added after the seed.
    pub with_item_evidence: u64,
    /// `with_evidence` in basis points of `active`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_evidence_share: Option<u64>,
    /// `with_item_evidence` in basis points of `active`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub with_item_evidence_share: Option<u64>,
}

/// The `properties` of `ekr.store-quality/1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyQuality {
    /// The property declarations of the revision's schema.
    pub declared: u64,
    /// Of those, the ones declaring a constraint.
    pub constrained: u64,
    /// `constrained` in basis points of `declared`; `None` when `declared` is 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub constrained_share: Option<u64>,
}

/// A name two or more nodes of one type hold, canonical name or alias.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedName {
    /// The type, written `type`.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// The name.
    pub name: String,
    /// The nodes holding it.
    pub nodes: Vec<NodeId>,
}

// ---- ekr.rejections/1 -------------------------------------------------------------------------

/// `ekr rejections`: the `ekr.rejections/1` document, each rejected transaction whose basis
/// revision is in the range, with the issues its rejection recorded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejections {
    /// `ekr.rejections/1`.
    pub format: String,
    /// The lowest basis revision, as requested; `None` when it was not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<u64>,
    /// The highest basis revision, as requested; `None` when it was not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u64>,
    /// By `against`, then transaction id.
    pub rejections: Vec<RejectedTransaction>,
}

/// One rejected transaction of `ekr rejections`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectedTransaction {
    /// Its id.
    pub transaction_id: TransactionId,
    /// The revision it was validated against.
    pub against: u64,
    /// Who submitted it.
    pub proposer: AgentId,
    /// When it was rejected.
    pub rejected_at: Timestamp,
    /// The issues of its rejection, in the order it recorded them.
    pub issues: Vec<RejectionIssue>,
}

/// A recorded validation issue, exactly as `ekr validate` printed it in the rejection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RejectionIssue {
    /// Its id.
    pub id: IssueId,
    /// The transaction refused.
    pub transaction_id: TransactionId,
    /// The validator that raised it, such as `Structural`.
    pub validator: String,
    /// Its stable code.
    pub code: String,
    /// What was wrong.
    pub message: String,
}

// ---- ekr.code-names/1 -------------------------------------------------------------------------

/// `ekr code-names`: the `ekr.code-names/1` document, every literal in the files read that
/// equals one of the store's names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeNames {
    /// The format and the counts.
    pub meta: CodeNamesMeta,
    /// In file, line and column order.
    pub findings: Vec<CodeNameFinding>,
}

/// The `meta` of `ekr.code-names/1`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeNamesMeta {
    /// `ekr.code-names/1`.
    pub format: String,
    /// The revision whose names were read.
    pub revision: u64,
    /// The distinct files read.
    pub files: u64,
    /// The literals found in them.
    pub literals: u64,
    /// The literals equal to a name that is the text of a store id, not reported.
    pub exempt: u64,
    /// The findings.
    pub findings: u64,
    /// The findings flagged `runtime_word`.
    pub runtime_word_findings: u64,
}

/// One literal equal to a store name.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeNameFinding {
    /// The file, as the path was given.
    pub file: String,
    /// The 1-based line.
    pub line: u64,
    /// The 1-based position of the opening quote, in characters.
    pub column: u64,
    /// The literal's text, raw.
    pub literal: String,
    /// Whether the literal is also one of the runtime's own words.
    pub runtime_word: bool,
    /// Every store name it equals.
    pub names: Vec<CodeNameMatch>,
}

/// What a store name that a literal equals names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodeNameMatch {
    /// What kind of name it is.
    pub kind: CodeNameKind,
    /// The type's, property's or node's id, by `kind`.
    pub id: String,
    /// For a node's name, its type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<TypeId>,
    /// For a node's name, its type's name, when the ontology names it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

/// The kind of a store name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodeNameKind {
    /// A node type's name.
    NodeType,
    /// An edge type's name.
    EdgeType,
    /// A property's name.
    Property,
    /// A node's canonical name.
    CanonicalName,
    /// A node's alias.
    Alias,
}

// ---- the calls ---------------------------------------------------------------------------------

impl<T: Transport> Reader<T> {
    /// `quality`: the `ekr.store-quality/1` document of `revision` (the head when `None`).
    ///
    /// # Errors
    /// [`ReadError`]; `ekr.views.NotSeeded` and `ekr.views.RevisionNotFound` are
    /// [`ReadError::Refused`].
    pub fn quality(&mut self, revision: Option<u64>) -> Result<StoreQuality, ReadError> {
        self.read(Argv::new("quality").flag("revision", revision))
    }

    /// `rejections`: each rejected transaction whose basis revision is in `from..=to`, an absent
    /// bound unbounded.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn rejections(
        &mut self,
        from: Option<u64>,
        to: Option<u64>,
    ) -> Result<Rejections, ReadError> {
        self.read(Argv::new("rejections").flag("from", from).flag("to", to))
    }

    /// `code-names`: every literal in `files` that equals one of the store's names at `at` (the
    /// head when `None`). `ekr` reads a relative path from its working directory, and each
    /// finding's `file` is the path as given here; a path may start with `-`.
    ///
    /// # Errors
    /// [`ReadError`]; no file is a [`ReadError::Usage`], a file that does not read or is not
    /// UTF-8 a [`ReadError::Fault`], and `ekr.views.RevisionNotFound` a [`ReadError::Refused`].
    pub fn code_names<I, S>(&mut self, files: I, at: Option<u64>) -> Result<CodeNames, ReadError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut argv = Argv::new("code-names").flag("at", at).arg("--");
        for file in files {
            argv = argv.arg(file.into());
        }
        self.read(argv)
    }
}

impl OneShotReader {
    /// [`Reader::quality`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn quality(&mut self, revision: Option<u64>) -> Result<StoreQuality, ReadError> {
        self.reader.quality(revision)
    }

    /// [`Reader::rejections`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn rejections(
        &mut self,
        from: Option<u64>,
        to: Option<u64>,
    ) -> Result<Rejections, ReadError> {
        self.reader.rejections(from, to)
    }

    /// [`Reader::code_names`], one-shot.
    ///
    /// # Errors
    /// [`ReadError`].
    pub fn code_names<I, S>(&mut self, files: I, at: Option<u64>) -> Result<CodeNames, ReadError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.reader.code_names(files, at)
    }
}
