//! The named formats (`views.yaml`, the domain summary's "Named formats"; ekr-mcp-fixes-20261015
//! #3, #13): each bounded read's selector between its predecessor and its named successor, and
//! the names map those successors, `ekr.explanation/3` and `ekr.resolution/1` carry.
//!
//! `ekr.graph-slice/2`, `ekr.graph-overview/3`, `ekr.graph-timeline/2` and `ekr.graph-changes/2`
//! are their predecessors with the names of the ontology ids they hold. A command answers its
//! predecessor when the request names no format; each host names its own default
//! ([`SliceFormat`], [`OverviewFormat`], [`TimelineFormat`], [`ChangesFormat`]).
//!
//! [`ontology_names`] is the one rule for the names map: every string of a document, as a value
//! or as an object key, that is the text of a node type, edge type or property id of the
//! ontology, mapped to its name; keys ascending. A host that renders a document of its own — the
//! CLI's explanation and resolution — calls it with the ontology of the revision it read.

use std::collections::BTreeMap;

use ekr_core::TypeId;
use ekr_ontology::Ontology;
use serde_json::Value;

use crate::query::{OVERVIEW_V2_FORMAT, SLICE_FORMAT};
use crate::{CHANGES_FORMAT, TIMELINE_FORMAT};

/// The format literal of `ekr.graph-overview/3`: [`OverviewFormat::V3`].
pub const OVERVIEW_V3_FORMAT: &str = "ekr.graph-overview/3";
/// The format literal of `ekr.graph-slice/2`: [`SliceFormat::V2`].
pub const SLICE_V2_FORMAT: &str = "ekr.graph-slice/2";
/// The format literal of `ekr.graph-timeline/2`: [`TimelineFormat::V2`].
pub const TIMELINE_V2_FORMAT: &str = "ekr.graph-timeline/2";
/// The format literal of `ekr.graph-changes/2`: [`ChangesFormat::V2`].
pub const CHANGES_V2_FORMAT: &str = "ekr.graph-changes/2";

/// A format name a read does not answer: malformed input that never reaches the command.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{given:?} is no format of this read; it answers {}", .expected.join(" or "))]
pub struct UnknownFormat {
    /// The name as the request gave it.
    pub given: String,
    /// The format literals the read answers, its predecessor first.
    pub expected: &'static [&'static str],
}

/// A selector between a read's predecessor format and its named successor.
macro_rules! format_selector {
    (
        $(#[$meta:meta])*
        $name:ident { $old:ident => $old_literal:expr, $new:ident => $new_literal:expr $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            /// The predecessor, which the command answers when none is named.
            $old,
            /// The named successor.
            $new,
        }

        impl $name {
            /// Both, the predecessor first.
            pub const ALL: [Self; 2] = [Self::$old, Self::$new];
            const LITERALS: [&'static str; 2] = [$old_literal, $new_literal];

            /// The format literal, as the document's meta.format and a request write it.
            #[must_use]
            pub const fn name(self) -> &'static str {
                match self {
                    Self::$old => $old_literal,
                    Self::$new => $new_literal,
                }
            }

            /// Whether this is the named successor, whose document carries names.
            #[must_use]
            pub const fn is_named(self) -> bool {
                matches!(self, Self::$new)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.name())
            }
        }

        impl std::str::FromStr for $name {
            type Err = UnknownFormat;

            fn from_str(text: &str) -> Result<Self, Self::Err> {
                Self::ALL
                    .into_iter()
                    .find(|format| format.name() == text)
                    .ok_or_else(|| UnknownFormat {
                        given: text.to_owned(),
                        expected: &Self::LITERALS,
                    })
            }
        }
    };
}

format_selector! {
    /// `ekr.views.OverviewFormat`: `ekr.graph-overview/2` or `ekr.graph-overview/3`.
    OverviewFormat { V2 => OVERVIEW_V2_FORMAT, V3 => OVERVIEW_V3_FORMAT }
}

format_selector! {
    /// `ekr.views.GraphSliceFormat`: `ekr.graph-slice/1` or `ekr.graph-slice/2`.
    SliceFormat { V1 => SLICE_FORMAT, V2 => SLICE_V2_FORMAT }
}

format_selector! {
    /// `ekr.views.GraphTimelineFormat`: `ekr.graph-timeline/1` or `ekr.graph-timeline/2`.
    TimelineFormat { V1 => TIMELINE_FORMAT, V2 => TIMELINE_V2_FORMAT }
}

format_selector! {
    /// `ekr.views.GraphChangesFormat`: `ekr.graph-changes/1` or `ekr.graph-changes/2`.
    ChangesFormat { V1 => CHANGES_FORMAT, V2 => CHANGES_V2_FORMAT }
}

/// `ekr.views.OntologyNames` of `document`: one entry for every string of it, as a value or as
/// an object key, whose text is the id of a node type, an edge type or a property of `ontology`,
/// mapped to its name, and no other entry; keys ascending. A type is named by its declared name;
/// a property by the definition the type of least id that has it — declared or inherited, node
/// and edge types alike — gives it; an id that is a type's and a property's is named as the type.
/// The cost is the ontology's declarations and the document's strings.
#[must_use]
pub fn ontology_names(ontology: &Ontology, document: &Value) -> BTreeMap<String, String> {
    let table = NameTable::of(ontology);
    let mut names = BTreeMap::new();
    table.collect(document, &mut names);
    names
}

/// Every id's text the names map can hold, with its name.
struct NameTable(BTreeMap<String, String>);

impl NameTable {
    fn of(ontology: &Ontology) -> Self {
        let declared = ontology.to_document();
        // Each type that has a property, ascending by id, node and edge types alike: the first
        // to name a property id names it.
        let mut owners: Vec<(TypeId, Vec<(String, String)>)> = Vec::new();
        for node_type in &declared.node_types {
            let properties = ontology
                .properties_of(node_type.id)
                .into_iter()
                .map(|(id, definition)| (id.to_string(), definition.name.clone()))
                .collect();
            owners.push((node_type.id, properties));
        }
        for edge_type in &declared.edge_types {
            let properties = edge_type
                .properties
                .iter()
                .map(|(id, definition)| (id.to_string(), definition.name.clone()))
                .collect();
            owners.push((edge_type.id, properties));
        }
        owners.sort_by_key(|(type_id, _)| *type_id);
        let mut table = BTreeMap::new();
        for (_, properties) in owners {
            for (id, name) in properties {
                table.entry(id).or_insert(name);
            }
        }
        // A type's own name wins over a property's of the same id.
        for (id, name) in declared
            .node_types
            .iter()
            .map(|declared| (declared.id, &declared.name))
            .chain(
                declared
                    .edge_types
                    .iter()
                    .map(|declared| (declared.id, &declared.name)),
            )
        {
            table.insert(id.to_string(), name.clone());
        }
        Self(table)
    }

    fn name(&self, text: &str, names: &mut BTreeMap<String, String>) {
        if names.contains_key(text) {
            return;
        }
        if let Some(name) = self.0.get(text) {
            names.insert(text.to_owned(), name.clone());
        }
    }

    fn collect(&self, value: &Value, names: &mut BTreeMap<String, String>) {
        match value {
            Value::String(text) => self.name(text, names),
            Value::Array(items) => {
                for item in items {
                    self.collect(item, names);
                }
            }
            Value::Object(fields) => {
                for (key, item) in fields {
                    self.name(key, names);
                    self.collect(item, names);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

/// The names map of a document about to be encoded, read off its JSON value: what the named
/// formats of this crate carry.
pub(crate) fn names_of(
    ontology: &Ontology,
    document: &impl serde::Serialize,
) -> Result<BTreeMap<String, String>, crate::ProjectError> {
    let value = serde_json::to_value(document).map_err(|error| {
        crate::ProjectError::Inconsistent(format!("reading the document's ids: {error}"))
    })?;
    Ok(ontology_names(ontology, &value))
}
