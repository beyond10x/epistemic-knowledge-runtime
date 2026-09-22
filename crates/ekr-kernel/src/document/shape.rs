//! Bounded traversal of the pinned loader's safe event observations.
use std::{cell::RefCell, collections::BTreeSet};

use serde::de::Error as _;
use serde_yaml_ng::{
    observation::{Document, Documents, Event, Tag},
    Error,
};

use super::{Budget, DocumentLimit, DOCUMENT_V1_LIMITS};

pub(super) enum Shape {
    Scalar,
    // Known key or enum-tag string bytes already charged by preflight.
    String(usize),
    Sequence(Vec<Shape>),
    Mapping(Vec<(String, Shape)>),
    Tagged {
        tag_bytes: usize,
        payload: Box<Shape>,
    },
}

impl Shape {
    pub(super) fn string_credit(&self) -> usize {
        match self {
            Self::String(bytes) => *bytes,
            Self::Tagged { payload, .. } => payload.string_credit(),
            _ => 0,
        }
    }
    pub(super) fn payload(&self) -> &Self {
        match self {
            Self::Tagged { payload, .. } => payload.payload(),
            other => other,
        }
    }
}

#[derive(Clone, Copy)]
enum Role {
    Envelope,
    Transaction,
    Operations,
    Evidence,
    Other,
}

pub(super) fn read(text: &str, budget: &RefCell<Budget>) -> Result<Shape, Error> {
    let mut documents = Documents::from_str(text)?;
    let document = documents
        .next_document()
        .ok_or_else(|| Error::custom("expected one document"))?;
    let mut traversal = Traversal {
        document: &document,
        budget,
        active: BTreeSet::new(),
    };
    let (shape, end) = traversal.visit(0, 0, Role::Envelope)?;
    document.check()?;
    if end != document.event_count() {
        return Err(Error::custom("unconsumed document events"));
    }
    // Check termination, including a second empty document. Loading remains
    // eager under the raw input cap; alias expansion is only this bounded walk.
    if documents.next_document().is_some() {
        return Err(budget.borrow_mut().refuse(DocumentLimit::Documents));
    }
    Ok(shape)
}

struct Traversal<'document, 'input, 'budget> {
    document: &'document Document<'input>,
    budget: &'budget RefCell<Budget>,
    active: BTreeSet<usize>,
}

impl<'document, 'input> Traversal<'document, 'input, '_> {
    fn event(&self, index: usize) -> Result<Event<'document, 'input>, Error> {
        match self.document.event(index)? {
            Some(event) => Ok(event),
            None => {
                self.document.check()?;
                Err(Error::custom("unexpected end of document"))
            }
        }
    }
    fn resolve(&self, mut index: usize) -> Result<usize, Error> {
        let mut aliases = BTreeSet::new();
        while let Event::Alias { target } = self.event(index)? {
            if !aliases.insert(index) {
                return Err(Error::custom("recursive alias"));
            }
            index = target;
        }
        if self.active.contains(&index) {
            return Err(Error::custom("recursive alias"));
        }
        Ok(index)
    }
    fn container(&self, depth: usize) -> Result<usize, Error> {
        depth
            .checked_add(1)
            .filter(|depth| *depth <= DOCUMENT_V1_LIMITS.depth)
            .ok_or_else(|| self.budget.borrow_mut().refuse(DocumentLimit::Depth))
    }
    fn tag(&self, tag: Option<Tag<'_>>) -> Result<Option<usize>, Error> {
        let Some(tag) = tag else { return Ok(None) };
        // Local enum spelling is already the profile's tag string. A global tag
        // retains its complete URI and never manufactures an enum boundary.
        let text = tag.enum_variant().unwrap_or(tag.decoded());
        self.budget.borrow_mut().string::<Error>(text, false)?;
        Ok(tag.enum_variant().map(str::len))
    }
    fn visit(&mut self, index: usize, depth: usize, role: Role) -> Result<(Shape, usize), Error> {
        let resolved = self.resolve(index)?;
        let event = self.event(resolved)?;
        let tag = match &event {
            Event::Scalar(scalar) => scalar.tag()?,
            Event::MappingStart(tag) | Event::SequenceStart(tag) => *tag,
            _ => None,
        };
        let local = self.tag(tag)?;
        // Tags always contribute strings. Only the actual typed enum visitor
        // can establish an enum boundary; requested scalars/collections may
        // ignore this same local tag. Their extra node/depth is charged there.
        self.budget.borrow_mut().node::<Error>()?;
        self.active.insert(resolved);
        let (shape, end) = match event {
            // Generic YAML classification may disagree with a requested String or
            // number. This pass allocates no scalar content; the actual typed
            // visitor charges Strings before allocating them.
            Event::Scalar(_) => (Shape::Scalar, resolved + 1),
            Event::Empty => (Shape::Scalar, resolved + 1),
            Event::SequenceStart(_) => self.sequence(resolved + 1, self.container(depth)?, role)?,
            Event::MappingStart(_) => self.mapping(resolved + 1, self.container(depth)?, role)?,
            _ => return Err(Error::custom("unexpected container end")),
        };
        self.active.remove(&resolved);
        let shape = match local {
            Some(tag_bytes) => Shape::Tagged {
                tag_bytes,
                payload: Box::new(shape),
            },
            None => shape,
        };
        Ok((shape, if resolved == index { end } else { index + 1 }))
    }
    fn sequence(
        &mut self,
        mut index: usize,
        depth: usize,
        role: Role,
    ) -> Result<(Shape, usize), Error> {
        let (cap, limit) = match role {
            Role::Operations => (DOCUMENT_V1_LIMITS.operations, DocumentLimit::Operations),
            Role::Evidence => (DOCUMENT_V1_LIMITS.evidence, DocumentLimit::Evidence),
            _ => (
                DOCUMENT_V1_LIMITS.sequence_elements,
                DocumentLimit::SequenceElements,
            ),
        };
        let mut elements = Vec::new();
        loop {
            if matches!(self.event(index)?, Event::SequenceEnd) {
                break;
            }
            // Presence is checked without resolving an alias or visiting a child.
            if elements.len() == cap {
                return Err(self.budget.borrow_mut().refuse(limit));
            }
            let (shape, next) = self.visit(index, depth, Role::Other)?;
            elements.push(shape);
            index = next;
        }
        if matches!(role, Role::Operations) && elements.is_empty() {
            return Err(self
                .budget
                .borrow_mut()
                .refuse(DocumentLimit::EmptyOperations));
        }
        Ok((Shape::Sequence(elements), index + 1))
    }
    fn key(&self, index: usize) -> Result<(String, usize), Error> {
        let resolved = self.resolve(index)?;
        self.budget.borrow_mut().node::<Error>()?;
        let Event::Scalar(scalar) = self.event(resolved)? else {
            return Err(Error::custom("expected a scalar mapping key"));
        };
        // Requested String keys expose raw decoded text, irrespective of numeric,
        // Boolean or tag interpretation. Their tags are metadata, not enums.
        self.tag(scalar.tag()?)?;
        let text = scalar.text()?;
        self.budget.borrow_mut().string::<Error>(text, true)?;
        Ok((text.to_owned(), index + 1))
    }
    fn mapping(
        &mut self,
        mut index: usize,
        depth: usize,
        role: Role,
    ) -> Result<(Shape, usize), Error> {
        let mut entries = Vec::new();
        let mut seen = BTreeSet::new();
        loop {
            if matches!(self.event(index)?, Event::MappingEnd) {
                break;
            }
            if entries.len() == DOCUMENT_V1_LIMITS.mapping_entries {
                return Err(self
                    .budget
                    .borrow_mut()
                    .refuse(DocumentLimit::MappingEntries));
            }
            let (key, next) = self.key(index)?;
            if !seen.insert(key.clone()) {
                return Err(Error::custom(format_args!(
                    "duplicate decoded mapping key {key:?}"
                )));
            }
            let child_role = match (role, key.as_str()) {
                (Role::Envelope, "transaction") => Role::Transaction,
                (Role::Transaction, "operations") => Role::Operations,
                (Role::Transaction, "evidence") => Role::Evidence,
                _ => Role::Other,
            };
            let (shape, next) = self.visit(next, depth, child_role)?;
            entries.push((key, shape));
            index = next;
        }
        Ok((Shape::Mapping(entries), index + 1))
    }
}
