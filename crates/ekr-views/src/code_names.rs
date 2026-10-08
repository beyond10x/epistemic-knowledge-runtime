//! `ekr.code-names/1` (`ekr.views.FindCodeNames`): which of a revision's names a consumer's
//! source files carry as literals, so the consumer can hold its store-reading code generic over
//! any ontology.
//!
//! [`literals`] finds the literals of one text, [`code_names`] answers from a loaded revision, and
//! [`find_code_names`] loads the revision first. The rules — what a literal is, which names count,
//! which are exempt, and the document's order — are `systems/ekr/domains/views.yaml`'s, at
//! `ekr.views.CodeNamesV1`. Nothing here reads a file: the host reads each source and passes its
//! text as a [`SourceText`].

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use serde::Serialize;

use crate::query::{encode, hash, Answer};
use crate::{load, LoadedRevision, ProjectError};

/// The format literal every answer carries in `meta.format`.
pub const CODE_NAMES_FORMAT: &str = "ekr.code-names/1";

/// Which source occurrences `FindCodeNames` examines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub enum CodeNameMode {
    /// Quoted literals, preserving the original document format.
    #[default]
    Literals,
    /// Exact names bounded by characters other than Unicode alphanumerics or underscore.
    Words,
}

/// The ESS domains of this runtime, `systems/ekr/domains/*.yaml`, by file name, as this crate was
/// built from them: the source of [`runtime_vocabulary`]. `tests/code_names.rs` holds the list
/// to the directory, so a domain file added there and not here is named.
pub const EMBEDDED_DOMAINS: [(&str, &str); 8] = [
    (
        "cli.yaml",
        include_str!("../../../systems/ekr/domains/cli.yaml"),
    ),
    (
        "graph.yaml",
        include_str!("../../../systems/ekr/domains/graph.yaml"),
    ),
    (
        "integrate.yaml",
        include_str!("../../../systems/ekr/domains/integrate.yaml"),
    ),
    (
        "kernel.yaml",
        include_str!("../../../systems/ekr/domains/kernel.yaml"),
    ),
    (
        "observe.yaml",
        include_str!("../../../systems/ekr/domains/observe.yaml"),
    ),
    (
        "ontology.yaml",
        include_str!("../../../systems/ekr/domains/ontology.yaml"),
    ),
    (
        "store.yaml",
        include_str!("../../../systems/ekr/domains/store.yaml"),
    ),
    (
        "views.yaml",
        include_str!("../../../systems/ekr/domains/views.yaml"),
    ),
];

/// The runtime's own words: every field name, enum variant, union tag and union variant name the
/// [`EMBEDDED_DOMAINS`] declare — the fields of their types, errors and events, and the inputs and
/// responses of their commands. A reader of the runtime's documents names these whatever the
/// ontology, so a finding whose literal is one of them carries `runtime_word: true`. Derived once,
/// on first use; a new field in a domain file needs no edit here.
///
/// # Panics
///
/// If an embedded domain is not YAML, which `tests/code_names.rs` rules out for the files this
/// crate is built from.
#[must_use]
pub fn runtime_vocabulary() -> &'static BTreeSet<String> {
    static WORDS: std::sync::OnceLock<BTreeSet<String>> = std::sync::OnceLock::new();
    WORDS.get_or_init(|| {
        use serde_yaml_ng::Value;
        fn names_of(list: &Value, into: &mut BTreeSet<String>) {
            for entry in list.as_sequence().into_iter().flatten() {
                if let Some(name) = entry.get("name").and_then(Value::as_str) {
                    into.insert(name.to_owned());
                }
            }
        }
        let mut words = BTreeSet::new();
        for (file, text) in EMBEDDED_DOMAINS {
            let domain: Value = serde_yaml_ng::from_str(text)
                .unwrap_or_else(|error| panic!("the embedded ESS domain {file}: {error}"));
            let section = |key: &str| {
                domain
                    .get(key)
                    .and_then(Value::as_sequence)
                    .cloned()
                    .unwrap_or_default()
            };
            for declared in section("types") {
                if let Some(fields) = declared.get("fields") {
                    names_of(fields, &mut words);
                }
                if let Some(tag) = declared.get("tag").and_then(Value::as_str) {
                    words.insert(tag.to_owned());
                }
                match declared.get("variants") {
                    Some(Value::Sequence(variants)) => {
                        words.extend(variants.iter().filter_map(Value::as_str).map(str::to_owned))
                    }
                    Some(Value::Mapping(variants)) => {
                        words.extend(variants.keys().filter_map(Value::as_str).map(str::to_owned))
                    }
                    _ => {}
                }
            }
            for declared in section("errors").into_iter().chain(section("events")) {
                if let Some(fields) = declared.get("fields") {
                    names_of(fields, &mut words);
                }
            }
            for command in section("commands") {
                for part in ["input", "response"] {
                    if let Some(list) = command.get(part) {
                        names_of(list, &mut words);
                    }
                }
            }
        }
        words
    })
}

/// One source file as the host read it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceText {
    /// The path exactly as the caller gave it, never resolved.
    pub path: String,
    /// The file's whole text.
    pub text: String,
}

/// One literal of a text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Literal {
    /// The 1-based line, lines split at `\n`.
    pub line: u64,
    /// The 1-based position of the opening quote on its line, in Unicode scalar values.
    pub column: u64,
    /// The raw text between the quotes; no escape is decoded.
    pub text: String,
}

/// The three quote characters a literal is delimited by.
const QUOTES: [char; 3] = ['"', '\'', '`'];

/// Every literal of `text`, ordered by line then column, each once. On each line (lines split at
/// `\n`) two scans are made, and a literal either finds is one:
///
/// - one pass left to right over every quote character at once, which skips a backslash and the
///   character after it and consumes a whole literal before looking for the next opening quote,
///   so a quote inside a literal of another kind opens nothing;
/// - one pass per quote character, blind to the other two, so a literal of one kind is found
///   inside a literal of another.
///
/// In both, a literal is the text between an opening quote and the next quote of the same
/// character that no backslash escapes, and a quote with no partner on its line opens nothing.
/// The cost is linear in the length of the text.
#[must_use]
pub fn literals(text: &str) -> Vec<Literal> {
    let mut found = BTreeSet::new();
    for (index, line) in text.split('\n').enumerate() {
        let chars: Vec<char> = line.chars().collect();
        let closings = Closings::of(&chars);
        let mut add = |open: usize, close: usize| {
            found.insert((
                index as u64 + 1,
                open as u64 + 1,
                chars[open + 1..close].iter().collect::<String>(),
            ));
        };
        // Every quote character at once, escapes and whole literals consumed.
        let mut at = 0;
        while at < chars.len() {
            match chars[at] {
                '\\' => at += 2,
                quote if QUOTES.contains(&quote) => match closings.after(quote, at) {
                    Some(close) => {
                        add(at, close);
                        at = close + 1;
                    }
                    None => at += 1,
                },
                _ => at += 1,
            }
        }
        // Each quote character on its own.
        for quote in QUOTES {
            let mut at = 0;
            while at < chars.len() {
                if chars[at] != quote {
                    at += 1;
                    continue;
                }
                match closings.after(quote, at) {
                    Some(close) => {
                        add(at, close);
                        at = close + 1;
                    }
                    None => at += 1,
                }
            }
        }
    }
    found
        .into_iter()
        .map(|(line, column, text)| Literal { line, column, text })
        .collect()
}

/// For each quote character and each position of one line, the first quote of that character at
/// or after it that no backslash escapes, reading from that position: a backslash skips itself
/// and the character after it. Built right to left in one pass per character, so finding a
/// literal's closing quote costs nothing however often it is asked.
struct Closings {
    next: [Vec<Option<usize>>; 3],
}

impl Closings {
    fn of(chars: &[char]) -> Self {
        let next = QUOTES.map(|quote| {
            let mut next = vec![None; chars.len() + 2];
            for at in (0..chars.len()).rev() {
                next[at] = match chars[at] {
                    '\\' => next[at + 2],
                    found if found == quote => Some(at),
                    _ => next[at + 1],
                };
            }
            next
        });
        Self { next }
    }

    /// The closing quote of a `quote` opened at `open`.
    fn after(&self, quote: char, open: usize) -> Option<usize> {
        let kind = QUOTES.iter().position(|known| *known == quote)?;
        self.next[kind].get(open + 1).copied().flatten()
    }
}

/// `ekr.views.CodeNamesFound`: what one answer returned, every field a function of its bytes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeNamesFound {
    /// `meta.revision`.
    pub revision: u64,
    /// `meta.files`.
    pub files: u64,
    /// `meta.literals`.
    pub literals: u64,
    /// `meta.exempt`.
    pub exempt: u64,
    /// `meta.findings`.
    pub findings: u64,
    /// `meta.runtime_word_findings`.
    pub runtime_word_findings: u64,
    /// Names of kind `NodeType` over every finding.
    pub node_types: u64,
    /// Names of kind `EdgeType` over every finding.
    pub edge_types: u64,
    /// Names of kind `Property` over every finding.
    pub properties: u64,
    /// Names of kind `CanonicalName` over every finding.
    pub canonical_names: u64,
    /// Names of kind `Alias` over every finding.
    pub aliases: u64,
    /// The first finding's file, if there is one.
    pub first_file: Option<String>,
    /// The first finding's line, if there is one.
    pub first_line: Option<u64>,
    /// The lowercase hex SHA-256 of the document's exact bytes.
    pub code_names_hash: String,
}

/// `ekr.views.CodeNameKind`, ordered as it declares its variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
enum Kind {
    NodeType,
    EdgeType,
    Property,
    CanonicalName,
    Alias,
}

/// `ekr.views.CodeNameMatch`, ordered by kind then id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
struct Match {
    kind: Kind,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_name: Option<String>,
}

#[derive(Serialize)]
struct Meta {
    format: &'static str,
    revision: u64,
    files: u64,
    literals: u64,
    exempt: u64,
    findings: u64,
    runtime_word_findings: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    mode: Option<CodeNameMode>,
}

#[derive(Serialize)]
struct Finding<'a> {
    file: &'a str,
    line: u64,
    column: u64,
    literal: String,
    runtime_word: bool,
    names: &'a BTreeSet<Match>,
}

#[derive(Serialize)]
struct Document<'a> {
    meta: Meta,
    findings: Vec<Finding<'a>>,
}

/// Every name `loaded` holds, with what it names, and the text of every id it holds.
fn names(loaded: &LoadedRevision) -> (BTreeMap<String, BTreeSet<Match>>, BTreeSet<String>) {
    let graph = &loaded.graph;
    let mut names: BTreeMap<String, BTreeSet<Match>> = BTreeMap::new();
    let mut name = |text: &str, found: Match| {
        if !text.trim().is_empty() {
            names.entry(text.to_owned()).or_default().insert(found);
        }
    };
    let plain = |kind: Kind, id: String| Match {
        kind,
        id,
        type_id: None,
        type_name: None,
    };
    let ontology = graph.ontology.to_document();
    let mut type_names = BTreeMap::new();
    for declared in &ontology.node_types {
        type_names.insert(declared.id, declared.name.clone());
        name(
            &declared.name,
            plain(Kind::NodeType, declared.id.to_string()),
        );
        for property in declared.properties.values() {
            name(
                &property.name,
                plain(Kind::Property, property.id.to_string()),
            );
        }
    }
    for declared in &ontology.edge_types {
        name(
            &declared.name,
            plain(Kind::EdgeType, declared.id.to_string()),
        );
        for property in declared.properties.values() {
            name(
                &property.name,
                plain(Kind::Property, property.id.to_string()),
            );
        }
    }
    for node in graph.nodes.values() {
        let named = |kind: Kind| Match {
            kind,
            id: node.id.to_string(),
            type_id: Some(node.type_id.to_string()),
            type_name: type_names.get(&node.type_id).cloned(),
        };
        name(&node.canonical_name, named(Kind::CanonicalName));
        for alias in &node.aliases {
            name(alias, named(Kind::Alias));
        }
    }

    let mut ids = BTreeSet::new();
    ids.insert(graph.root.id.to_string());
    for (version, (_, schema)) in &loaded.schemas {
        ids.insert(version.to_string());
        let schema = schema.to_document();
        for declared in &schema.node_types {
            ids.insert(declared.id.to_string());
            ids.extend(declared.properties.keys().map(ToString::to_string));
        }
        for declared in &schema.edge_types {
            ids.insert(declared.id.to_string());
            ids.extend(declared.properties.keys().map(ToString::to_string));
        }
    }
    for entry in &loaded.revisions {
        ids.insert(entry.schema_version.to_string());
        ids.extend(entry.transaction_id.map(|id| id.to_string()));
    }
    for declared in &ontology.node_types {
        ids.insert(declared.id.to_string());
        ids.extend(declared.properties.keys().map(ToString::to_string));
    }
    for declared in &ontology.edge_types {
        ids.insert(declared.id.to_string());
        ids.extend(declared.properties.keys().map(ToString::to_string));
    }
    ids.extend(graph.nodes.keys().map(ToString::to_string));
    ids.extend(graph.edges.keys().map(ToString::to_string));
    for (id, assertion) in &graph.assertions {
        ids.insert(id.to_string());
        ids.insert(assertion.proposed_by.to_string());
    }
    for (id, evidence) in &graph.evidence {
        ids.insert(id.to_string());
        ids.insert(evidence.extracted_by.to_string());
    }
    (names, ids)
}

/// Whether the store name `name` is exempt: the text, in any case, of a store id.
fn exempt(name: &str, ids: &BTreeSet<String>) -> bool {
    ids.contains(&name.to_lowercase())
}

/// Answers `ekr.views.FindCodeNames` from `loaded`: the `ekr.code-names/1` document of
/// `sources` against its names, and its counts.
///
/// Pure: the same revision and the same sources answer the same bytes, whatever order the
/// sources came in. A path given twice is read once, with the text it was first given.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] when the document does not encode.
pub fn code_names(
    loaded: &LoadedRevision,
    sources: &[SourceText],
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    code_names_with_mode(loaded, sources, CodeNameMode::Literals)
}

/// [`code_names`] with an explicit occurrence mode.
///
/// # Errors
/// [`ProjectError::Inconsistent`] when the document does not encode.
pub fn code_names_with_mode(
    loaded: &LoadedRevision,
    sources: &[SourceText],
    mode: CodeNameMode,
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    let (names, ids) = names(loaded);
    let mut given = std::collections::HashSet::with_capacity(sources.len());
    let mut read: Vec<&SourceText> = sources
        .iter()
        .filter(|source| given.insert(source.path.as_str()))
        .collect();
    read.sort_by(|a, b| a.path.cmp(&b.path));
    let vocabulary = runtime_vocabulary();
    let (mut literal_count, mut exempt_count) = (0_u64, 0_u64);
    let mut findings = Vec::new();
    for source in &read {
        let candidates = match mode {
            CodeNameMode::Literals => literals(&source.text),
            CodeNameMode::Words => words(&source.text, names.keys().map(String::as_str)),
        };
        for literal in candidates {
            literal_count += 1;
            let Some((name, matched)) = names.get_key_value(&literal.text) else {
                continue;
            };
            if exempt(name, &ids) {
                exempt_count += 1;
                continue;
            }
            findings.push(Finding {
                file: &source.path,
                line: literal.line,
                column: literal.column,
                runtime_word: vocabulary.contains(name),
                literal: literal.text,
                names: matched,
            });
        }
    }
    let document = Document {
        meta: Meta {
            format: CODE_NAMES_FORMAT,
            revision: loaded.graph.revision.get(),
            files: read.len() as u64,
            literals: literal_count,
            exempt: exempt_count,
            findings: findings.len() as u64,
            runtime_word_findings: findings
                .iter()
                .filter(|finding| finding.runtime_word)
                .count() as u64,
            mode: (mode == CodeNameMode::Words).then_some(mode),
        },
        findings,
    };
    let bytes = encode(&document)?;
    let count = |kind: Kind| {
        document
            .findings
            .iter()
            .flat_map(|finding| finding.names.iter())
            .filter(|found| found.kind == kind)
            .count() as u64
    };
    let summary = CodeNamesFound {
        revision: document.meta.revision,
        files: document.meta.files,
        literals: document.meta.literals,
        exempt: document.meta.exempt,
        findings: document.meta.findings,
        runtime_word_findings: document.meta.runtime_word_findings,
        node_types: count(Kind::NodeType),
        edge_types: count(Kind::EdgeType),
        properties: count(Kind::Property),
        canonical_names: count(Kind::CanonicalName),
        aliases: count(Kind::Alias),
        first_file: document
            .findings
            .first()
            .map(|finding| finding.file.to_owned()),
        first_line: document.findings.first().map(|finding| finding.line),
        code_names_hash: hash(&bytes),
    };
    Ok(Answer { bytes, summary })
}

/// Loads revision `at` of `runtime`'s store (the head when `None`) and answers
/// `ekr.views.FindCodeNames` for `sources`: [`load`] then [`code_names`]. Reads only.
///
/// # Errors
///
/// Whatever [`load`] or [`code_names`] refuses: [`ProjectError::NotSeeded`] and
/// [`ProjectError::RevisionNotFound`] among them.
pub fn find_code_names(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    sources: &[SourceText],
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    code_names(&load(runtime, at)?, sources)
}

/// Loads the requested revision and examines sources with an explicit occurrence mode.
///
/// # Errors
/// Whatever [`load`] or [`code_names_with_mode`] refuses.
pub fn find_code_names_with_mode(
    runtime: &Runtime,
    at: Option<RevisionNumber>,
    sources: &[SourceText],
    mode: CodeNameMode,
) -> Result<Answer<CodeNamesFound>, ProjectError> {
    code_names_with_mode(&load(runtime, at)?, sources, mode)
}

fn words<'a>(text: &str, names: impl Iterator<Item = &'a str> + Clone) -> Vec<Literal> {
    let word = |character: char| character.is_alphanumeric() || character == '_';
    let mut found = BTreeSet::new();
    for (line_index, line) in text.split('\n').enumerate() {
        for name in names.clone() {
            let mut from = 0;
            while let Some(offset) = line[from..].find(name) {
                let start = from + offset;
                let end = start + name.len();
                if !line[..start].chars().next_back().is_some_and(word)
                    && !line[end..].chars().next().is_some_and(word)
                {
                    found.insert((
                        line_index as u64 + 1,
                        line[..start].chars().count() as u64 + 1,
                        name.to_owned(),
                    ));
                }
                // Advancing one scalar preserves overlapping names, including punctuation.
                from = start
                    + line[start..]
                        .chars()
                        .next()
                        .expect("nonempty name")
                        .len_utf8();
            }
        }
    }
    found
        .into_iter()
        .map(|(line, column, text)| Literal { line, column, text })
        .collect()
}
