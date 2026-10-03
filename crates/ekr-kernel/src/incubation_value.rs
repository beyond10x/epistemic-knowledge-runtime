//! Bounded decoding of the graph's existing canonical value projection.
use ekr_core::canonical::Canonical;
use ekr_core::contract_data::EkrGraphTypedValue;
use ekr_graph::{CanonicalRef, CanonicalValue};
use ekr_store::StoreError;

fn refused() -> StoreError {
    StoreError::Document("incubation-value-encoding".into())
}

pub(super) fn decode(input: &EkrGraphTypedValue) -> Result<ekr_ontology::Value, StoreError> {
    let bytes = ekr_core::bytes::decode(&input.canonical_bytes).map_err(|_| refused())?;
    let mut reader = Reader(&bytes);
    let value = reader.value(0)?;
    if !reader.0.is_empty() || value.canonical_bytes() != bytes {
        return Err(refused());
    }
    let value: ekr_ontology::Value = value.into();
    if serde_json::to_value(value.kind()).map_err(|_| refused())?
        != serde_json::to_value(&input.kind).map_err(|_| refused())?
    {
        return Err(refused());
    }
    Ok(value)
}

struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], StoreError> {
        let (head, tail) = self.0.split_at_checked(length).ok_or_else(refused)?;
        self.0 = tail;
        Ok(head)
    }
    fn tag(&mut self, expected: u8) -> Result<(), StoreError> {
        if self.take(1)? != [expected] {
            return Err(refused());
        }
        Ok(())
    }
    fn length(&mut self) -> Result<usize, StoreError> {
        let count = u64::from_be_bytes(self.take(8)?.try_into().map_err(|_| refused())?);
        let count: usize = count.try_into().map_err(|_| refused())?;
        // Every element occupies at least one byte: refuse hostile capacities before allocation.
        if count > self.0.len() {
            return Err(refused());
        }
        Ok(count)
    }
    fn string(&mut self) -> Result<String, StoreError> {
        self.tag(0x06)?;
        let length = self.length()?;
        String::from_utf8(self.take(length)?.to_vec()).map_err(|_| refused())
    }
    fn number(&mut self) -> Result<i64, StoreError> {
        self.tag(0x05)?;
        i128::from_be_bytes(self.take(16)?.try_into().map_err(|_| refused())?)
            .try_into()
            .map_err(|_| refused())
    }
    fn value(&mut self, depth: usize) -> Result<CanonicalValue, StoreError> {
        if depth > 32 {
            return Err(refused());
        }
        self.tag(0x0f)?;
        let variant = u32::from_be_bytes(self.take(4)?.try_into().map_err(|_| refused())?);
        Ok(match variant {
            0 => CanonicalValue::String(self.string()?),
            1 => CanonicalValue::Boolean(match self.take(1)? {
                [0x02] => false,
                [0x03] => true,
                _ => return Err(refused()),
            }),
            2 => CanonicalValue::Integer(self.number()?),
            3 => CanonicalValue::Decimal(self.string()?),
            4 => CanonicalValue::Timestamp(ekr_core::Timestamp::from_millis(self.number()?)),
            5 => CanonicalValue::Duration(self.number()?),
            6 => {
                self.tag(0x0d)?;
                let bits = u128::from_be_bytes(self.take(16)?.try_into().map_err(|_| refused())?);
                let text = format!(
                    "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                    bits >> 96,
                    (bits >> 80) & 0xffff,
                    (bits >> 64) & 0xffff,
                    (bits >> 48) & 0xffff,
                    bits & 0xffffffffffff
                );
                CanonicalValue::NodeRef(CanonicalRef::new(text.parse().map_err(|_| refused())?))
            }
            7 => CanonicalValue::Enum(self.string()?),
            8 => {
                self.tag(0x08)?;
                let count = self.length()?;
                CanonicalValue::List(
                    (0..count)
                        .map(|_| self.value(depth + 1))
                        .collect::<Result<_, _>>()?,
                )
            }
            9 => {
                self.tag(0x0a)?;
                let count = self.length()?;
                let mut fields = std::collections::BTreeMap::new();
                for _ in 0..count {
                    let key = self.string()?;
                    if fields.insert(key, self.value(depth + 1)?).is_some() {
                        return Err(refused());
                    }
                }
                CanonicalValue::Record(fields)
            }
            _ => return Err(refused()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_canonical_encoder_is_the_oracle_for_every_value_kind() {
        let values = vec![
            CanonicalValue::String("amber\n".into()),
            CanonicalValue::Boolean(true),
            CanonicalValue::Integer(i64::MIN),
            CanonicalValue::Decimal("1.25".into()),
            CanonicalValue::Timestamp(ekr_core::Timestamp::from_millis(-1)),
            CanonicalValue::Duration(25),
            CanonicalValue::NodeRef(CanonicalRef::new(ekr_core::NodeId::mint())),
            CanonicalValue::Enum("amber".into()),
            CanonicalValue::List(vec![
                CanonicalValue::Boolean(false),
                CanonicalValue::String(String::new()),
            ]),
            CanonicalValue::Record(
                [(
                    "nested".into(),
                    CanonicalValue::List(vec![CanonicalValue::Integer(8)]),
                )]
                .into(),
            ),
        ];
        for value in values {
            let native: ekr_ontology::Value = value.clone().into();
            let bytes = value.canonical_bytes();
            let mut projection: EkrGraphTypedValue = serde_json::from_value(serde_json::json!({
                "kind":native.kind(), "canonical_bytes":ekr_core::bytes::encode(&bytes)
            }))
            .unwrap();
            assert_eq!(decode(&projection).unwrap(), native);
            for length in 0..bytes.len() {
                projection.canonical_bytes = ekr_core::bytes::encode(&bytes[..length]);
                assert!(
                    decode(&projection).is_err(),
                    "truncation {length} accepted for {native:?}"
                );
            }
            let mut extended = bytes;
            extended.push(0);
            projection.canonical_bytes = ekr_core::bytes::encode(&extended);
            assert!(decode(&projection).is_err());
        }
    }
}
