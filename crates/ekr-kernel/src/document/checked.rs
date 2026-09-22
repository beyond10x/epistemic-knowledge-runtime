//! Strict typed traversal of the original bytes using the bounded syntax shape.
//!
//! Serde newtypes/options do not introduce semantic depth. Buffered enum content
//! is charged when first visited, and is already bounded before Serde replays it.
use serde::de::{self, DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};
use serde::Deserializer;
use std::{cell::RefCell, fmt};

use super::{shape::Shape, Budget, DocumentLimit, DOCUMENT_V1_LIMITS};

pub(super) struct Checked<'a, D> {
    inner: D,
    shape: &'a Shape,
    budget: &'a RefCell<Budget>,
    key: bool,
    depth: usize,
}
impl<'a, D> Checked<'a, D> {
    pub(super) fn new(inner: D, shape: &'a Shape, budget: &'a RefCell<Budget>) -> Self {
        Self {
            inner,
            shape,
            budget,
            key: false,
            depth: 0,
        }
    }
    fn visitor<V>(&self, inner: V) -> CheckVisitor<'a, V> {
        CheckVisitor {
            inner,
            shape: self.shape,
            budget: self.budget,
            key: self.key,
            depth: self.depth,
        }
    }
}

macro_rules! forward {
    ($($method:ident $(($($arg:ident: $ty:ty),*))?),* $(,)?) => {$(
        fn $method<V: Visitor<'de>>(self, $($($arg: $ty,)*)? visitor: V) -> Result<V::Value, D::Error> {
            let checked = self.visitor(visitor);
            self.inner.$method($($($arg,)*)? checked)
        }
    )*};
}
impl<'de, D: Deserializer<'de>> Deserializer<'de> for Checked<'_, D> {
    type Error = D::Error;
    forward!(deserialize_any, deserialize_bool, deserialize_i8, deserialize_i16, deserialize_i32, deserialize_i64,
        deserialize_i128, deserialize_u8, deserialize_u16, deserialize_u32, deserialize_u64, deserialize_u128,
        deserialize_f32, deserialize_f64, deserialize_char, deserialize_str, deserialize_string, deserialize_bytes,
        deserialize_byte_buf, deserialize_option, deserialize_unit, deserialize_identifier, deserialize_ignored_any,
        deserialize_unit_struct(name: &'static str), deserialize_newtype_struct(name: &'static str),
        deserialize_enum(name: &'static str, variants: &'static [&'static str]));
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        if !matches!(self.shape.payload(), Shape::Sequence(_)) {
            return Err(de::Error::custom("expected an explicit sequence"));
        }
        let checked = self.visitor(visitor);
        self.inner.deserialize_seq(checked)
    }
    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        if !matches!(self.shape.payload(), Shape::Sequence(_)) {
            return Err(de::Error::custom("expected an explicit sequence"));
        }
        let checked = self.visitor(visitor);
        self.inner.deserialize_tuple(len, checked)
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        if !matches!(self.shape.payload(), Shape::Sequence(_)) {
            return Err(de::Error::custom("expected an explicit sequence"));
        }
        let checked = self.visitor(visitor);
        self.inner.deserialize_tuple_struct(name, len, checked)
    }
    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, D::Error> {
        if !matches!(self.shape.payload(), Shape::Mapping(_)) {
            return Err(de::Error::custom("expected an explicit mapping"));
        }
        let checked = self.visitor(visitor);
        self.inner.deserialize_map(checked)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error> {
        if !matches!(self.shape.payload(), Shape::Mapping(_)) {
            return Err(de::Error::custom("expected an explicit mapping"));
        }
        let checked = self.visitor(visitor);
        self.inner.deserialize_struct(name, fields, checked)
    }
    fn is_human_readable(&self) -> bool {
        self.inner.is_human_readable()
    }
}

struct CheckVisitor<'a, V> {
    inner: V,
    shape: &'a Shape,
    budget: &'a RefCell<Budget>,
    key: bool,
    depth: usize,
}
impl<V> CheckVisitor<'_, V> {
    fn container<E: de::Error>(&self) -> Result<usize, E> {
        self.depth
            .checked_add(1)
            .filter(|depth| *depth <= DOCUMENT_V1_LIMITS.depth)
            .ok_or_else(|| self.budget.borrow_mut().refuse(DocumentLimit::Depth))
    }
}
macro_rules! scalar {
    ($($method:ident: $ty:ty),* $(,)?) => {$(
        fn $method<E: de::Error>(self, value: $ty) -> Result<Self::Value, E> { self.inner.$method(value) }
    )*};
}
impl<'de, V: Visitor<'de>> Visitor<'de> for CheckVisitor<'_, V> {
    type Value = V::Value;
    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.expecting(formatter)
    }
    scalar!(visit_bool: bool, visit_i8: i8, visit_i16: i16, visit_i32: i32, visit_i64: i64, visit_i128: i128,
        visit_u8: u8, visit_u16: u16, visit_u32: u32, visit_u64: u64, visit_u128: u128, visit_f32: f32, visit_f64: f64, visit_char: char);
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
        self.budget
            .borrow_mut()
            .string_with_credit(value, self.key, self.shape.string_credit())?;
        self.inner.visit_str(value)
    }
    fn visit_borrowed_str<E: de::Error>(self, value: &'de str) -> Result<Self::Value, E> {
        self.budget
            .borrow_mut()
            .string_with_credit(value, self.key, self.shape.string_credit())?;
        self.inner.visit_borrowed_str(value)
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
        self.budget.borrow_mut().string_with_credit(
            &value,
            self.key,
            self.shape.string_credit(),
        )?;
        self.inner.visit_string(value)
    }
    fn visit_bytes<E: de::Error>(self, value: &[u8]) -> Result<Self::Value, E> {
        self.inner.visit_bytes(value)
    }
    fn visit_borrowed_bytes<E: de::Error>(self, value: &'de [u8]) -> Result<Self::Value, E> {
        self.inner.visit_borrowed_bytes(value)
    }
    fn visit_byte_buf<E: de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
        self.inner.visit_byte_buf(value)
    }
    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_unit()
    }
    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.inner.visit_none()
    }
    fn visit_some<D: Deserializer<'de>>(self, input: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_some(Checked {
            inner: input,
            shape: self.shape,
            budget: self.budget,
            key: self.key,
            depth: self.depth,
        })
    }
    fn visit_newtype_struct<D: Deserializer<'de>>(self, input: D) -> Result<Self::Value, D::Error> {
        self.inner.visit_newtype_struct(Checked {
            inner: input,
            shape: self.shape,
            budget: self.budget,
            key: self.key,
            depth: self.depth,
        })
    }
    fn visit_seq<A: SeqAccess<'de>>(self, input: A) -> Result<Self::Value, A::Error> {
        let Shape::Sequence(children) = self.shape.payload() else {
            return Err(de::Error::custom("expected an explicit sequence"));
        };
        let depth = self.container()?;
        self.inner.visit_seq(Sequence {
            inner: input,
            children: children.iter(),
            budget: self.budget,
            depth,
        })
    }
    fn visit_map<A: MapAccess<'de>>(self, input: A) -> Result<Self::Value, A::Error> {
        let Shape::Mapping(entries) = self.shape.payload() else {
            return Err(de::Error::custom("expected an explicit mapping"));
        };
        let depth = self.container()?;
        self.inner.visit_map(Mapping {
            inner: input,
            entries: entries.iter(),
            pending: None,
            budget: self.budget,
            depth,
        })
    }
    fn visit_enum<A: EnumAccess<'de>>(self, input: A) -> Result<Self::Value, A::Error> {
        let (payload, variant_credit, depth) = match self.shape {
            Shape::Tagged { tag_bytes, payload } => {
                self.budget.borrow_mut().node()?;
                (payload.as_ref(), *tag_bytes, self.container()?)
            }
            other => (other, other.string_credit(), self.depth),
        };
        self.inner.visit_enum(Enumeration {
            inner: input,
            payload,
            variant_credit,
            budget: self.budget,
            depth,
        })
    }
}

struct Seed<'a, S> {
    inner: S,
    shape: &'a Shape,
    budget: &'a RefCell<Budget>,
    key: bool,
    depth: usize,
}
impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for Seed<'_, S> {
    type Value = S::Value;
    fn deserialize<D: Deserializer<'de>>(self, input: D) -> Result<Self::Value, D::Error> {
        self.inner.deserialize(Checked {
            inner: input,
            shape: self.shape,
            budget: self.budget,
            key: self.key,
            depth: self.depth,
        })
    }
}
struct End;
impl<'de> DeserializeSeed<'de> for End {
    type Value = ();
    fn deserialize<D: Deserializer<'de>>(self, _: D) -> Result<(), D::Error> {
        Err(de::Error::custom("typed traversal exceeded bounded syntax"))
    }
}
struct Sequence<'a, A> {
    inner: A,
    children: std::slice::Iter<'a, Shape>,
    budget: &'a RefCell<Budget>,
    depth: usize,
}
impl<'de, A: SeqAccess<'de>> SeqAccess<'de> for Sequence<'_, A> {
    type Error = A::Error;
    fn next_element_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        let Some(shape) = self.children.next() else {
            self.inner.next_element_seed(End)?;
            return Ok(None);
        };
        self.inner.next_element_seed(Seed {
            inner: seed,
            shape,
            budget: self.budget,
            key: false,
            depth: self.depth,
        })
    }
    fn size_hint(&self) -> Option<usize> {
        None
    }
}
struct Mapping<'a, A> {
    inner: A,
    entries: std::slice::Iter<'a, (String, Shape)>,
    pending: Option<&'a Shape>,
    budget: &'a RefCell<Budget>,
    depth: usize,
}
impl<'de, A: MapAccess<'de>> MapAccess<'de> for Mapping<'_, A> {
    type Error = A::Error;
    fn next_key_seed<S: DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, A::Error> {
        let Some((key, shape)) = self.entries.next() else {
            self.inner.next_key_seed(End)?;
            return Ok(None);
        };
        self.pending = Some(shape);
        self.inner.next_key_seed(Seed {
            inner: seed,
            shape: &Shape::String(key.len()),
            budget: self.budget,
            key: true,
            depth: self.depth,
        })
    }
    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value, A::Error> {
        let shape = self
            .pending
            .take()
            .ok_or_else(|| de::Error::custom("mapping value without key"))?;
        self.inner.next_value_seed(Seed {
            inner: seed,
            shape,
            budget: self.budget,
            key: false,
            depth: self.depth,
        })
    }
    fn size_hint(&self) -> Option<usize> {
        None
    }
}
struct Enumeration<'a, A> {
    inner: A,
    payload: &'a Shape,
    variant_credit: usize,
    budget: &'a RefCell<Budget>,
    depth: usize,
}
impl<'de, 'a, A: EnumAccess<'de>> EnumAccess<'de> for Enumeration<'a, A> {
    type Error = A::Error;
    type Variant = Variant<'a, A::Variant>;
    fn variant_seed<S: DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), A::Error> {
        let (value, variant) = self.inner.variant_seed(Seed {
            inner: seed,
            shape: &Shape::String(self.variant_credit),
            budget: self.budget,
            key: false,
            depth: self.depth,
        })?;
        Ok((
            value,
            Variant {
                inner: variant,
                shape: self.payload,
                budget: self.budget,
                depth: self.depth,
            },
        ))
    }
}
struct Variant<'a, A> {
    inner: A,
    shape: &'a Shape,
    budget: &'a RefCell<Budget>,
    depth: usize,
}
impl<'de, A: VariantAccess<'de>> VariantAccess<'de> for Variant<'_, A> {
    type Error = A::Error;
    fn unit_variant(self) -> Result<(), A::Error> {
        self.inner.unit_variant()
    }
    fn newtype_variant_seed<S: DeserializeSeed<'de>>(self, seed: S) -> Result<S::Value, A::Error> {
        self.inner.newtype_variant_seed(Seed {
            inner: seed,
            shape: self.shape,
            budget: self.budget,
            key: false,
            depth: self.depth,
        })
    }
    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, A::Error> {
        self.inner.tuple_variant(
            len,
            CheckVisitor {
                inner: visitor,
                shape: self.shape,
                budget: self.budget,
                key: false,
                depth: self.depth,
            },
        )
    }
    fn struct_variant<V: Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, A::Error> {
        self.inner.struct_variant(
            fields,
            CheckVisitor {
                inner: visitor,
                shape: self.shape,
                budget: self.budget,
                key: false,
                depth: self.depth,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::document::{DocumentLimit, DOCUMENT_V1_LIMITS};

    struct Allocate<'a>(&'a Cell<bool>);
    impl Visitor<'_> for Allocate<'_> {
        type Value = String;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("text")
        }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<String, E> {
            self.0.set(true);
            Ok(value.to_owned())
        }
    }

    #[test]
    fn string_refusals_precede_the_allocating_application_visitor() {
        for (text, key, initial, expected) in [
            (
                "x".repeat(DOCUMENT_V1_LIMITS.string_bytes + 1),
                false,
                0,
                DocumentLimit::StringBytes,
            ),
            (
                "x".repeat(DOCUMENT_V1_LIMITS.key_bytes + 1),
                true,
                0,
                DocumentLimit::KeyBytes,
            ),
            (
                "x".into(),
                false,
                DOCUMENT_V1_LIMITS.total_string_bytes,
                DocumentLimit::TotalStringBytes,
            ),
        ] {
            let called = Cell::new(false);
            let budget = RefCell::new(Budget {
                strings: initial,
                ..Budget::default()
            });
            let input = serde::de::value::StrDeserializer::<serde::de::value::Error>::new(&text);
            let checked = Checked {
                inner: input,
                shape: &Shape::Scalar,
                budget: &budget,
                key,
                depth: 0,
            };
            assert!(checked.deserialize_string(Allocate(&called)).is_err());
            assert!(!called.get(), "allocating visitor ran for {expected}");
            assert_eq!(budget.borrow().refused, Some(expected));
        }
        let called = Cell::new(false);
        let budget = RefCell::new(Budget::default());
        let input = serde::de::value::StrDeserializer::<serde::de::value::Error>::new("ok");
        assert_eq!(
            Checked::new(input, &Shape::Scalar, &budget)
                .deserialize_string(Allocate(&called))
                .unwrap(),
            "ok"
        );
        assert!(called.get());
    }

    #[test]
    fn mixed_accounting_refuses_before_allocating_a_coerced_string() {
        for over in [0, 1] {
            let budget = RefCell::new(Budget {
                strings: DOCUMENT_V1_LIMITS.total_string_bytes - 8 + over,
                ..Budget::default()
            });
            // Local tag "bound" contributes five; numeric scalar text contributes
            // three only when the actual typed String visitor requests it.
            let text = "!bound 0.0";
            let shape = crate::document::shape::read(text, &budget).unwrap();
            let called = Cell::new(false);
            let result = Checked::new(serde_yaml_ng::Deserializer::from_str(text), &shape, &budget)
                .deserialize_string(Allocate(&called));
            if over == 0 {
                assert_eq!(result.unwrap(), "0.0");
                assert!(called.get());
                assert_eq!(
                    budget.borrow().strings,
                    DOCUMENT_V1_LIMITS.total_string_bytes
                );
            } else {
                assert!(result.is_err());
                assert!(!called.get());
                assert_eq!(
                    budget.borrow().refused,
                    Some(DocumentLimit::TotalStringBytes)
                );
            }
        }
        // A quoted String is charged when the typed visitor requests it; the
        // tag was already charged in preflight and is not charged again.
        let budget = RefCell::new(Budget {
            strings: DOCUMENT_V1_LIMITS.total_string_bytes - 10,
            ..Budget::default()
        });
        let text = "!bound 'value'";
        let shape = crate::document::shape::read(text, &budget).unwrap();
        let called = Cell::new(false);
        assert_eq!(
            Checked::new(serde_yaml_ng::Deserializer::from_str(text), &shape, &budget)
                .deserialize_string(Allocate(&called))
                .unwrap(),
            "value"
        );
        assert!(called.get());
        assert_eq!(
            budget.borrow().strings,
            DOCUMENT_V1_LIMITS.total_string_bytes
        );
    }
}
