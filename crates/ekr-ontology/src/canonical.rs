//! Complete declaration encodings, shared by ontology roots and schema operations.
use crate::{
    Cardinality, EdgeType, Lifecycle, NodeType, Ontology, OperationDefinition, PropertyDefinition,
    SchemaVersion, Transition, ValueType,
};
use ekr_core::canonical::{Canonical, Encoder};

impl Canonical for Cardinality {
    fn encode(&self, out: &mut Encoder) {
        out.variant(match self {
            Self::One => 0,
            Self::Many => 1,
        });
    }
}

impl Canonical for ValueType {
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::String => out.variant(0),
            Self::Boolean => out.variant(1),
            Self::Integer => out.variant(2),
            Self::Float => out.variant(3),
            Self::Decimal => out.variant(4),
            Self::Timestamp => out.variant(5),
            Self::Duration => out.variant(6),
            Self::NodeRef { allowed_types } => {
                out.variant(7);
                allowed_types.encode(out);
            }
            Self::Enum { variants } => {
                out.variant(8);
                variants.encode(out);
            }
            Self::List(element) => {
                out.variant(9);
                element.encode(out);
            }
            Self::Record(fields) => {
                out.variant(10);
                fields.encode(out);
            }
        }
    }
}

impl Canonical for PropertyDefinition {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.name.encode(out);
        self.value_type.encode(out);
        self.cardinality.encode(out);
        self.required.encode(out);
        self.constraints.encode(out);
    }
}

impl Canonical for Transition {
    fn encode(&self, out: &mut Encoder) {
        self.from.encode(out);
        self.to.encode(out);
    }
}

impl Canonical for Lifecycle {
    fn encode(&self, out: &mut Encoder) {
        self.initial.encode(out);
        self.states.encode(out);
        self.transitions.encode(out);
    }
}

impl Canonical for OperationDefinition {
    fn encode(&self, out: &mut Encoder) {
        self.name.encode(out);
        self.arguments.encode(out);
        self.preconditions.encode(out);
        self.transition.encode(out);
        self.emits.encode(out);
    }
}

impl Canonical for NodeType {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.name.encode(out);
        self.parents.encode(out);
        self.properties.encode(out);
        self.abstract_type.encode(out);
        self.lifecycle.encode(out);
        self.operations.encode(out);
    }
}

impl Canonical for EdgeType {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.name.encode(out);
        self.source_types.encode(out);
        self.target_types.encode(out);
        self.cardinality.encode(out);
        self.properties.encode(out);
        self.inverse.encode(out);
        self.symmetric.encode(out);
        self.transitive.encode(out);
    }
}

impl Canonical for SchemaVersion {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.number.encode(out);
        self.parent.encode(out);
        self.created_at.encode(out);
    }
}

impl Canonical for Ontology {
    fn encode(&self, out: &mut Encoder) {
        self.version().encode(out);
        out.map(self.node_types());
        out.map(self.edge_types());
    }
}
