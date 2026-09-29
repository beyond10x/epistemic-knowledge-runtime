//! The frozen limits of `ekr.transaction-document/2` (`docs/cli.md`, "Document limits"), and the
//! check that holds an SDK document to them before `ekr propose` would refuse it.
//!
//! The limits are the kernel's (`ekr_kernel::DOCUMENT_V2_LIMITS`), pinned here because the SDK
//! does not link the kernel and `ekr-core` does not carry them; the format version freezes them,
//! and `crates/ekr-sdk/tests/document_drift.rs` holds this table and every limit's name to the
//! kernel's, and each limit's verdict to the kernel reader's at and past its bound.
//!
//! The check counts what the kernel's reader counts, on the YAML the SDK writes: the bytes; every
//! value and every mapping key as a node; every mapping and sequence as a level of nesting, and
//! every tag (`!AddAssertion`, `!Node`, …) as one more node and one more level, since the reader
//! opens an enum there; and the bytes of every string, key and tag name.

use serde_yaml_ng::Value;

use super::DocumentError;

/// Inclusive limits of one transaction-document format version, as the kernel names them.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DocumentLimits {
    /// Bytes of the whole document.
    pub input_bytes: usize,
    /// Nesting of mappings, sequences and tagged variants; the document itself is one.
    pub depth: usize,
    /// Values, mapping keys and tagged variants.
    pub nodes: usize,
    /// Entries in one mapping.
    pub mapping_entries: usize,
    /// Elements in one sequence other than `operations` and `evidence`.
    pub sequence_elements: usize,
    /// Bytes of one string or tag name.
    pub string_bytes: usize,
    /// Bytes of one mapping key.
    pub key_bytes: usize,
    /// Bytes of every string, key and tag name together.
    pub total_string_bytes: usize,
    /// Operations in one transaction.
    pub operations: usize,
    /// Ids in the transaction's `evidence` list.
    pub evidence: usize,
}

/// The limits of `ekr.transaction-document/2`, frozen with the format.
pub const TRANSACTION_LIMITS: DocumentLimits = DocumentLimits {
    input_bytes: 8_388_608,
    depth: 32,
    nodes: 1_048_576,
    mapping_entries: 4_096,
    sequence_elements: 16_384,
    string_bytes: 65_536,
    key_bytes: 4_096,
    total_string_bytes: 33_554_432,
    operations: 10_000,
    evidence: 10_000,
};

/// One limit of [`DocumentLimits`], by the name the kernel's refusal gives it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DocumentLimit {
    /// `input_bytes`.
    InputBytes,
    /// `container_depth`.
    Depth,
    /// `expanded_nodes`.
    Nodes,
    /// `mapping_entries`.
    MappingEntries,
    /// `sequence_elements`.
    SequenceElements,
    /// `string_bytes`.
    StringBytes,
    /// `key_bytes`.
    KeyBytes,
    /// `total_string_bytes`.
    TotalStringBytes,
    /// `operations`.
    Operations,
    /// `evidence_elements`.
    Evidence,
}

impl DocumentLimit {
    /// Every limit.
    pub const ALL: [Self; 10] = [
        Self::InputBytes,
        Self::Depth,
        Self::Nodes,
        Self::MappingEntries,
        Self::SequenceElements,
        Self::StringBytes,
        Self::KeyBytes,
        Self::TotalStringBytes,
        Self::Operations,
        Self::Evidence,
    ];

    /// The name `ekr propose` refuses it by: `transaction document limit: <name>`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::InputBytes => "input_bytes",
            Self::Depth => "container_depth",
            Self::Nodes => "expanded_nodes",
            Self::MappingEntries => "mapping_entries",
            Self::SequenceElements => "sequence_elements",
            Self::StringBytes => "string_bytes",
            Self::KeyBytes => "key_bytes",
            Self::TotalStringBytes => "total_string_bytes",
            Self::Operations => "operations",
            Self::Evidence => "evidence_elements",
        }
    }

    /// Its inclusive bound in [`TRANSACTION_LIMITS`].
    #[must_use]
    pub const fn bound(self) -> usize {
        let limits = TRANSACTION_LIMITS;
        match self {
            Self::InputBytes => limits.input_bytes,
            Self::Depth => limits.depth,
            Self::Nodes => limits.nodes,
            Self::MappingEntries => limits.mapping_entries,
            Self::SequenceElements => limits.sequence_elements,
            Self::StringBytes => limits.string_bytes,
            Self::KeyBytes => limits.key_bytes,
            Self::TotalStringBytes => limits.total_string_bytes,
            Self::Operations => limits.operations,
            Self::Evidence => limits.evidence,
        }
    }
}

/// Which list a sequence is, for the limit its length is held to.
#[derive(Copy, Clone)]
enum Role {
    Envelope,
    Transaction,
    Operations,
    Evidence,
    Other,
}

/// The running count of one document.
#[derive(Default)]
struct Count {
    nodes: usize,
    strings: usize,
    depth: usize,
    first: Option<(DocumentLimit, usize)>,
}

impl Count {
    fn over(&mut self, limit: DocumentLimit, value: usize) {
        if value > limit.bound() && self.first.is_none() {
            self.first = Some((limit, value));
        }
    }

    fn string(&mut self, text: &str, key: bool) {
        if key {
            self.over(DocumentLimit::KeyBytes, text.len());
        }
        self.over(DocumentLimit::StringBytes, text.len());
        self.strings += text.len();
    }

    fn level(&mut self, depth: usize) -> usize {
        let depth = depth + 1;
        self.depth = self.depth.max(depth);
        depth
    }

    /// One value: a node, and whatever it holds.
    fn visit(&mut self, value: &Value, depth: usize, role: Role) {
        self.nodes += 1;
        self.payload(value, depth, role);
    }

    /// What a value holds, its own node already counted.
    fn payload(&mut self, value: &Value, depth: usize, role: Role) {
        match value {
            Value::Tagged(tagged) => {
                let name = tagged.tag.to_string();
                self.string(name.strip_prefix('!').unwrap_or(&name), false);
                self.nodes += 1;
                let depth = self.level(depth);
                self.payload(&tagged.value, depth, role);
            }
            Value::Sequence(elements) => {
                let depth = self.level(depth);
                let limit = match role {
                    Role::Operations => DocumentLimit::Operations,
                    Role::Evidence => DocumentLimit::Evidence,
                    _ => DocumentLimit::SequenceElements,
                };
                self.over(limit, elements.len());
                for element in elements {
                    self.visit(element, depth, Role::Other);
                }
            }
            Value::Mapping(entries) => {
                let depth = self.level(depth);
                self.over(DocumentLimit::MappingEntries, entries.len());
                for (key, value) in entries {
                    self.nodes += 1;
                    let key = key.as_str().unwrap_or_default();
                    self.string(key, true);
                    let role = match (role, key) {
                        (Role::Envelope, "transaction") => Role::Transaction,
                        (Role::Transaction, "operations") => Role::Operations,
                        (Role::Transaction, "evidence") => Role::Evidence,
                        _ => Role::Other,
                    };
                    self.visit(value, depth, role);
                }
            }
            Value::String(text) => self.string(text, false),
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

/// Holds `yaml`, a transaction document the SDK wrote, to [`TRANSACTION_LIMITS`].
pub(super) fn check(yaml: &str) -> Result<(), DocumentError> {
    let refused = |limit: DocumentLimit, value: usize| {
        Err(DocumentError::Limit {
            limit,
            bound: limit.bound(),
            value,
        })
    };
    if yaml.len() > DocumentLimit::InputBytes.bound() {
        return refused(DocumentLimit::InputBytes, yaml.len());
    }
    let value: Value = serde_yaml_ng::from_str(yaml)?;
    let mut count = Count::default();
    count.visit(&value, 0, Role::Envelope);
    if let Some((limit, value)) = count.first {
        return refused(limit, value);
    }
    for (limit, value) in [
        (DocumentLimit::Depth, count.depth),
        (DocumentLimit::Nodes, count.nodes),
        (DocumentLimit::TotalStringBytes, count.strings),
    ] {
        if value > limit.bound() {
            return refused(limit, value);
        }
    }
    Ok(())
}
