//! `ekr compare`: whether two stores hold the same knowledge, ignoring commit times and
//! transaction ids — the `ekr.store-comparison/1` document (`ekr.cli.CompareStores`,
//! `systems/ekr/domains/cli.yaml`, story:store-equivalence-check).
//!
//! Each store is opened as a verb that only reads opens one and read once, through the kernel's
//! verified read ([`Runtime::read`]) at its requested revision or its head. Nothing is written to
//! either. What is compared is the domain's: per category, entries keyed by id, each entry the
//! JSON of the graph's own record without the fields a commit's clock or its transaction decides.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::RevisionNumber;
use ekr_kernel::{CommitError, Runtime, VerifiedRead};
use serde::Serialize;
use serde_json::{Map, Value};

use crate::exit::Failure;

/// The most differing ids a category lists; `differences` counts all of them.
const SHOWN: usize = 20;

/// The `ekr.store-comparison/1` document (`ekr.cli.StoreComparisonV1`).
#[derive(Serialize)]
pub(super) struct Comparison {
    format: &'static str,
    equal: bool,
    left: Side,
    right: Side,
    categories: Vec<Category>,
}

/// `ekr.cli.ComparedSide`: the revision a store was read at.
#[derive(Serialize)]
struct Side {
    revision: u64,
}

/// `ekr.cli.CategoryComparison`.
#[derive(Serialize)]
struct Category {
    category: &'static str,
    equal: bool,
    left: usize,
    right: usize,
    differences: usize,
    differing: Vec<Difference>,
}

/// `ekr.cli.Difference`: an id one store holds and the other does not, or holds differently.
#[derive(Serialize)]
struct Difference {
    id: String,
    presence: &'static str,
    fields: Vec<String>,
}

/// One category's entries: id → the compared fields.
type Entries = BTreeMap<String, Map<String, Value>>;

/// Which store a failure came from, named in its message.
#[derive(Clone, Copy)]
pub(super) enum Which {
    /// The configured store (`--store`).
    Store,
    /// The other store (`--other`).
    Other,
}

impl Which {
    const fn name(self) -> &'static str {
        match self {
            Self::Store => "the store (--store)",
            Self::Other => "the other store (--other)",
        }
    }
}

/// `failure`, its message prefixed with the store it came from.
pub(super) fn from(which: Which, failure: Failure) -> Failure {
    match failure {
        Failure::Refused { name, message } => Failure::Refused {
            name,
            message: format!("{}: {message}", which.name()),
        },
        Failure::Fault {
            message,
            diverged,
            replaced,
        } => Failure::Fault {
            message: format!("{}: {message}", which.name()),
            diverged,
            replaced,
        },
        usage @ Failure::Usage { .. } => usage,
    }
}

/// The verified read of `runtime` at `at`, or at its head. A store never seeded is the named
/// refusal `ekr.views.NotSeeded`, and a revision it does not hold `ekr.kernel.RevisionNotFound`
/// (exit 2), each naming the store.
pub(super) fn read(
    runtime: &Runtime,
    at: Option<u64>,
    which: Which,
) -> Result<VerifiedRead, Failure> {
    runtime
        .read(at.map(RevisionNumber::new))
        .map_err(|error| match error {
            error @ CommitError::NotSeeded => Failure::refused("ekr.views.NotSeeded", error),
            error => Failure::from(error),
        })
        .map_err(|failure| from(which, failure))
}

/// Compares two verified reads.
pub(super) fn run(left: &VerifiedRead, right: &VerifiedRead) -> Result<Comparison, Failure> {
    let (left_entries, right_entries) = (entries(left)?, entries(right)?);
    let mut categories: Vec<Category> = left_entries
        .into_iter()
        .zip(right_entries)
        .map(|((category, left), (_, right))| compare(category, &left, &right))
        .collect();
    // The seventh category is listed only where either store holds a supersession record, so a
    // pair that never superseded compares, and prints, exactly as before it existed.
    let (left_superseded, right_superseded) = (superseded(left)?, superseded(right)?);
    if !(left_superseded.is_empty() && right_superseded.is_empty()) {
        categories.push(compare("superseded", &left_superseded, &right_superseded));
    }
    Ok(Comparison {
        format: "ekr.store-comparison/1",
        equal: categories.iter().all(|category| category.equal),
        left: Side {
            revision: left.root.revision.get(),
        },
        right: Side {
            revision: right.root.revision.get(),
        },
        categories,
    })
}

/// One category of both stores.
fn compare(category: &'static str, left: &Entries, right: &Entries) -> Category {
    let ids: BTreeSet<&String> = left.keys().chain(right.keys()).collect();
    let mut differing = Vec::new();
    for id in ids {
        let difference = match (left.get(id), right.get(id)) {
            (Some(_), None) => Difference {
                id: id.clone(),
                presence: "left",
                fields: Vec::new(),
            },
            (None, Some(_)) => Difference {
                id: id.clone(),
                presence: "right",
                fields: Vec::new(),
            },
            (Some(l), Some(r)) if l != r => Difference {
                id: id.clone(),
                presence: "both",
                fields: l
                    .keys()
                    .chain(r.keys())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .filter(|field| l.get(*field) != r.get(*field))
                    .cloned()
                    .collect(),
            },
            _ => continue,
        };
        differing.push(difference);
    }
    let differences = differing.len();
    differing.truncate(SHOWN);
    Category {
        category,
        equal: differences == 0,
        left: left.len(),
        right: right.len(),
        differences,
        differing,
    }
}

/// The id `id` as the document prints it.
fn key(id: &impl Serialize) -> Result<String, Failure> {
    match serde_json::to_value(id).map_err(Failure::fault)? {
        Value::String(id) => Ok(id),
        other => Ok(other.to_string()),
    }
}

/// `record` as a JSON object, without `excluded`.
fn fields(record: &impl Serialize, excluded: &[&str]) -> Result<Map<String, Value>, Failure> {
    match serde_json::to_value(record).map_err(Failure::fault)? {
        Value::Object(mut map) => {
            for field in excluded {
                map.remove(*field);
            }
            Ok(map)
        }
        other => Err(Failure::fault(format!(
            "compare: a record serialised as {other} rather than an object"
        ))),
    }
}

/// The supersession records of one read, keyed by the superseded node's id
/// (`story:supersede-node`): the node and the edges retired with it as `nodes` and `edges` compare
/// them, `by`, `revision`, `reason` and `evidence`. Two stores that superseded one duplicate by
/// different nodes resolve its key to different nodes, so they differ here.
fn superseded(read: &VerifiedRead) -> Result<Entries, Failure> {
    let mut records = Entries::new();
    for (id, record) in &read.graph.superseded {
        let mut edges = Map::new();
        for (edge_id, edge) in &record.edges {
            edges.insert(
                key(edge_id)?,
                Value::Object(fields(edge, &["id", "root_id"])?),
            );
        }
        let mut entry = Map::new();
        entry.insert(
            "node".to_owned(),
            Value::Object(fields(&record.node, &["id", "root_id"])?),
        );
        entry.insert("edges".to_owned(), Value::Object(edges));
        entry.insert(
            "by".to_owned(),
            serde_json::to_value(record.by).map_err(Failure::fault)?,
        );
        entry.insert("revision".to_owned(), Value::from(record.revision.get()));
        entry.insert("reason".to_owned(), Value::String(record.reason.clone()));
        entry.insert(
            "evidence".to_owned(),
            serde_json::to_value(&record.evidence).map_err(Failure::fault)?,
        );
        records.insert(key(id)?, entry);
    }
    Ok(records)
}

/// The six categories of one read, in `ekr.cli.ComparisonCategory`'s order.
fn entries(read: &VerifiedRead) -> Result<[(&'static str, Entries); 6], Failure> {
    let graph = &read.graph;
    let ontology = graph.ontology.to_document();

    let mut types = Entries::new();
    let mut properties = Entries::new();
    let mut typed = Vec::new();
    for node_type in &ontology.node_types {
        let mut entry = fields(node_type, &["id"])?;
        entry.insert("kind".to_owned(), Value::String("node".to_owned()));
        typed.push((key(&node_type.id)?, entry, &node_type.properties));
    }
    for edge_type in &ontology.edge_types {
        let mut entry = fields(edge_type, &["id"])?;
        entry.insert("kind".to_owned(), Value::String("edge".to_owned()));
        typed.push((key(&edge_type.id)?, entry, &edge_type.properties));
    }
    // The lowest declaring type id's definition is the property's; every declaring type is in
    // `declared_by`, in type id order.
    typed.sort_by(|a, b| a.0.cmp(&b.0));
    for (id, entry, declared) in typed {
        for (property, definition) in declared {
            let property = key(property)?;
            if !properties.contains_key(&property) {
                let mut first = fields(definition, &["id"])?;
                first.insert("declared_by".to_owned(), Value::Array(Vec::new()));
                properties.insert(property.clone(), first);
            }
            if let Some(Value::Array(by)) = properties
                .get_mut(&property)
                .and_then(|entry| entry.get_mut("declared_by"))
            {
                by.push(Value::String(id.clone()));
            }
        }
        types.insert(id, entry);
    }

    let mut nodes = Entries::new();
    for (id, node) in &graph.nodes {
        nodes.insert(key(id)?, fields(node, &["id", "root_id"])?);
    }
    let mut edges = Entries::new();
    for (id, edge) in &graph.edges {
        edges.insert(key(id)?, fields(edge, &["id", "root_id"])?);
    }
    let mut assertions = Entries::new();
    for (id, assertion) in &graph.assertions {
        let mut entry = fields(
            assertion,
            &["id", "root_id", "proposed_by", "transaction_time"],
        )?;
        let attached: Vec<&ekr_graph::AttachedEvidence> = graph.attached(*id).collect();
        entry.insert(
            "attached_evidence".to_owned(),
            serde_json::to_value(attached).map_err(Failure::fault)?,
        );
        assertions.insert(key(id)?, entry);
    }
    let mut evidence = Entries::new();
    for (id, record) in &graph.evidence {
        evidence.insert(key(id)?, fields(record, &["id"])?);
    }
    Ok([
        ("types", types),
        ("properties", properties),
        ("nodes", nodes),
        ("edges", edges),
        ("assertions", assertions),
        ("evidence", evidence),
    ])
}
