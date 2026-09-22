//! Opt-in observations of buffered YAML loader events, without alias expansion.
//!
//! This facade does not change the normal Serde deserializer. It exposes metadata
//! that typed visitors may ignore, including complete decoded global tags. Callers
//! must bound input before constructing [`Documents`] and bound their own alias
//! traversal. Loading still allocates the existing event tape eagerly.

use crate::de::{self, Event as LoadedEvent, Progress};
use crate::error::{self, Error, Result};
use crate::libyaml::parser::Scalar as LoadedScalar;
use crate::libyaml::tag::Tag as LoadedTag;
use crate::loader::{Document as LoadedDocument, Loader};
use serde::de::{Error as _, Visitor};
use std::{fmt, str, sync::Arc};

/// An opaque stream using the same loader as the default deserializer.
pub struct Documents<'input> {
    loader: Loader<'input>,
}

impl<'input> Documents<'input> {
    /// Begin observing an already bounded UTF-8 input.
    pub fn from_str(input: &'input str) -> Result<Self> {
        Ok(Self {
            loader: Loader::new(Progress::Str(input))?,
        })
    }

    /// Buffer the next document, retaining any loader error on that document.
    /// A document containing an error must never be admitted without [`Document::check`].
    pub fn next_document(&mut self) -> Option<Document<'input>> {
        self.loader.next_document().map(|inner| Document { inner })
    }
}

/// One immutable event tape. Alias positions refer to this document only.
pub struct Document<'input> {
    inner: LoadedDocument<'input>,
}

impl<'input> Document<'input> {
    /// Number of buffered events, before expanding any alias.
    pub fn event_count(&self) -> usize {
        self.inner.events.len()
    }

    /// Read an event without expansion. Out-of-range positions return `None`.
    /// Alias targets are resolved and checked against this document's tape.
    pub fn event(&self, index: usize) -> Result<Option<Event<'_, 'input>>> {
        let Some((event, _)) = self.inner.events.get(index) else {
            return Ok(None);
        };
        Ok(Some(match event {
            LoadedEvent::Scalar(inner) => Event::Scalar(Scalar { inner }),
            LoadedEvent::SequenceStart(start) => Event::SequenceStart(tag(&start.tag)?),
            LoadedEvent::SequenceEnd => Event::SequenceEnd,
            LoadedEvent::MappingStart(start) => Event::MappingStart(tag(&start.tag)?),
            LoadedEvent::MappingEnd => Event::MappingEnd,
            LoadedEvent::Void => Event::Empty,
            LoadedEvent::Alias(id) => {
                let target = self
                    .inner
                    .aliases
                    .get(id)
                    .copied()
                    .filter(|target| *target < self.inner.events.len())
                    .ok_or_else(|| Error::custom("alias has no buffered target"))?;
                Event::Alias { target }
            }
        }))
    }

    /// Return the loader's original error, if any.
    ///
    /// Errors are retained instead of raised when loading so a caller can refuse
    /// an earlier duplicate key without visiting its malformed second value.
    /// Successful inspection of an event alone does not establish a valid document.
    pub fn check(&self) -> Result<()> {
        match &self.inner.error {
            Some(error) => Err(error::shared(Arc::clone(error))),
            None => Ok(()),
        }
    }
}

/// A borrowed loader event, with all tag metadata intact.
pub enum Event<'document, 'input> {
    /// Decoded scalar text, tag and the existing resolver.
    Scalar(Scalar<'document, 'input>),
    /// Start of a sequence and its optional tag.
    SequenceStart(Option<Tag<'document>>),
    /// End of a sequence.
    SequenceEnd,
    /// Start of a mapping and its optional tag.
    MappingStart(Option<Tag<'document>>),
    /// End of a mapping.
    MappingEnd,
    /// Resolved alias, deliberately not expanded.
    Alias {
        /// Index of the anchored event in this document.
        target: usize,
    },
    /// The loader's empty-stream placeholder.
    Empty,
}

/// Complete decoded tag text and its existing Serde enum interpretation.
#[derive(Clone, Copy, Debug)]
pub struct Tag<'document> {
    decoded: &'document str,
    enum_variant: Option<&'document str>,
}

impl<'document> Tag<'document> {
    /// Complete decoded tag, including a global URI or local leading `!`.
    pub fn decoded(self) -> &'document str {
        self.decoded
    }
    /// The local variant spelling used by the default deserializer, if any.
    /// Global and built-in tags do not become enum variants.
    pub fn enum_variant(self) -> Option<&'document str> {
        self.enum_variant
    }
}

fn tag(value: &Option<LoadedTag>) -> Result<Option<Tag<'_>>> {
    let Some(raw) = value else { return Ok(None) };
    Ok(Some(Tag {
        decoded: str::from_utf8(raw).map_err(Error::custom)?,
        enum_variant: de::parse_tag(value),
    }))
}

/// A borrowed scalar retaining the loader's decoded text and resolution context.
pub struct Scalar<'document, 'input> {
    inner: &'document LoadedScalar<'input>,
}

impl Scalar<'_, '_> {
    /// Decoded scalar text, identical to the text requested String decoding sees.
    pub fn text(&self) -> Result<&str> {
        str::from_utf8(&self.inner.value).map_err(Error::custom)
    }
    /// Complete tag metadata, including tags ignored by typed String decoding.
    pub fn tag(&self) -> Result<Option<Tag<'_>>> {
        tag(&self.inner.tag)
    }
    /// Classify through the existing scalar resolver without allocating a Value.
    ///
    /// `local_enum_payload` is true only when a caller has entered the local
    /// enum boundary reported by [`Tag::enum_variant`], matching the default
    /// deserializer's `current_enum` context. It is false for global tags.
    pub fn kind(&self, local_enum_payload: bool) -> Result<ScalarKind> {
        de::visit_scalar(Classify, self.inner, local_enum_payload)
    }
}

/// Scalar categories emitted by the existing resolver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarKind {
    /// Null or an empty plain scalar.
    Null,
    /// Boolean.
    Boolean,
    /// Signed or unsigned integer, including wide integer forms.
    Integer,
    /// Floating-point number, including nonfinite forms.
    Float,
    /// String content.
    String,
}

struct Classify;
impl Visitor<'_> for Classify {
    type Value = ScalarKind;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a scalar")
    }
    fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Null)
    }
    fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Boolean)
    }
    fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Integer)
    }
    fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Integer)
    }
    fn visit_i128<E: serde::de::Error>(self, _: i128) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Integer)
    }
    fn visit_u128<E: serde::de::Error>(self, _: u128) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Integer)
    }
    fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::Float)
    }
    fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<ScalarKind, E> {
        Ok(ScalarKind::String)
    }
}
