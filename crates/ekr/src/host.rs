//! Strict local host configuration and explicit valid-time selectors.
//!
//! Trusted local configuration is separate from seed/proposal input. Its decoding supplies no
//! authority: the kernel still checks registered actors, the profile and the retained anchor.
//! This module neither opens a provider nor samples a clock.

use std::fmt;

use ekr_core::Timestamp;
use ekr_kernel::{AuthorityStateV1, BootstrapContext};
use serde::{Deserialize, Serialize};
use time::{Date, Month};

/// The exact version of the CLI host-configuration transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum CliHostFormatV1 {
    /// The original JSON host envelope, unrelated to persisted record versions.
    #[serde(rename = "ekr.cli-host/1")]
    V1,
}

impl<'de> Deserialize<'de> for CliHostFormatV1 {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        let format = String::deserialize(decoder)?;
        if format == "ekr.cli-host/1" {
            Ok(Self::V1)
        } else {
            Err(serde::de::Error::unknown_variant(
                &format,
                &["ekr.cli-host/1"],
            ))
        }
    }
}

/// Explicit trusted-host inputs for opening a runtime.
///
/// Successful decoding does not establish that the authority profile is supported or that its
/// registered agents and retained anchor agree. Those semantic checks belong to the kernel.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct CliHostConfigurationV1 {
    /// Exactly `ekr.cli-host/1`.
    pub format: CliHostFormatV1,
    /// The configured provider namespace; the provider validates namespace semantics.
    pub tenant: String,
    /// The host's operator and distinct validator.
    pub context: BootstrapContext,
    /// The complete anchor, using the existing strict kernel carrier.
    pub authority: AuthorityStateV1,
}

impl<'de> Deserialize<'de> for CliHostConfigurationV1 {
    fn deserialize<D: serde::Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Fields {
            format: CliHostFormatV1,
            tenant: String,
            context: BootstrapContext,
            authority: AuthorityStateV1,
        }

        let fields = Fields::deserialize(NamedObjects(decoder))?;
        Ok(Self {
            format: fields.format,
            tenant: fields.tenant,
            context: fields.context,
            authority: fields.authority,
        })
    }
}

impl schemars::JsonSchema for CliHostFormatV1 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "CliHostFormatV1".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "const": "ekr.cli-host/1" })
    }
}

impl CliHostConfigurationV1 {
    /// The JSON Schema (draft 2020-12) of an `ekr.cli-host/1` document, generated from this
    /// type: what `ekr schema ekr.cli-host/1` prints.
    #[must_use]
    pub fn json_schema_document() -> schemars::Schema {
        // The derive copies maintainers' rustdoc into descriptions; an agent reads this one.
        let mut schema = schemars::generate::SchemaSettings::draft2020_12()
            .with_transform(schemars::transform::RecursiveTransform(
                |schema: &mut schemars::Schema| {
                    schema.remove("description");
                },
            ))
            .into_generator()
            .into_root_schema_for::<Self>();
        for (field, description) in [
            ("format", "Exactly `ekr.cli-host/1`."),
            ("tenant", "The provider namespace the store verbs open."),
            (
                "context",
                "The host's operator (proposes and commits) and its distinct validator.",
            ),
            (
                "authority",
                "The authority anchor: the registered agents and the P1 validation profile, \
                 as `ekr example ekr.cli-host/1` prints them.",
            ),
        ] {
            schema
                .get_mut("properties")
                .and_then(|properties| properties.get_mut(field))
                .and_then(serde_json::Value::as_object_mut)
                .map(|property| property.insert("description".to_owned(), description.into()));
        }
        schema
            .get_mut("$defs")
            .and_then(|definitions| definitions.get_mut("AgentId"))
            .and_then(serde_json::Value::as_object_mut)
            .map(|agent| {
                agent.insert(
                    "description".to_owned(),
                    "An agent id: a UUID in lowercase hyphenated form.".into(),
                )
            });
        schema.insert("title".to_owned(), "ekr.cli-host/1".into());
        schema.insert(
            "description".to_owned(),
            "The trusted host document for --host or EKR_HOST: the provider namespace, the \
             operator and validator, and the authority anchor (`ekr example ekr.cli-host/1`). \
             Beyond the schema, the reader also refuses a key written twice."
                .into(),
        );
        schema
    }

    /// Decode one complete UTF-8 JSON host document, allowing surrounding JSON whitespace.
    ///
    /// Decodes the original input directly into typed carriers, preserving duplicate detection.
    ///
    /// # Errors
    ///
    /// Refuses malformed input, missing/unknown/duplicate fields, duplicate decoded registry
    /// keys or capability members, unsupported envelope formats and trailing data.
    pub fn from_json(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}

// This transport adapter changes only named-struct syntax. Every field, value, enum, map-key,
// set-member and semantic decoder remains the actual carrier's decoder. Recursing through seeds
// is essential: wrapping only the outer host would still admit positional nested authorities.
struct NamedObjects<D>(D);

macro_rules! forward_deserialization {
    ($($method:ident $(($($argument:ident: $argument_type:ty),*))?);* $(;)?) => {
        $(
            fn $method<V: serde::de::Visitor<'de>>(
                self,
                $($($argument: $argument_type,)*)?
                visitor: V,
            ) -> Result<V::Value, Self::Error> {
                self.0.$method($($($argument,)*)? Nested(visitor))
            }
        )*
    };
}

impl<'de, D: serde::Deserializer<'de>> serde::Deserializer<'de> for NamedObjects<D> {
    type Error = D::Error;

    forward_deserialization! {
        deserialize_any;
        deserialize_bool;
        deserialize_i8;
        deserialize_i16;
        deserialize_i32;
        deserialize_i64;
        deserialize_i128;
        deserialize_u8;
        deserialize_u16;
        deserialize_u32;
        deserialize_u64;
        deserialize_u128;
        deserialize_f32;
        deserialize_f64;
        deserialize_char;
        deserialize_str;
        deserialize_string;
        deserialize_bytes;
        deserialize_byte_buf;
        deserialize_option;
        deserialize_unit;
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_seq;
        deserialize_tuple(len: usize);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_map;
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier;
        deserialize_ignored_any;
    }

    fn deserialize_struct<V: serde::de::Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.deserialize_map(Nested(visitor))
    }

    fn is_human_readable(&self) -> bool {
        self.0.is_human_readable()
    }
}

struct Nested<V>(V);

macro_rules! forward_scalar_visits {
    ($($method:ident($value:ident: $kind:ty));* $(;)?) => {
        $(
            fn $method<E: serde::de::Error>(self, $value: $kind) -> Result<Self::Value, E> {
                self.0.$method($value)
            }
        )*
    };
}

impl<'de, V: serde::de::Visitor<'de>> serde::de::Visitor<'de> for Nested<V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.expecting(formatter)
    }

    forward_scalar_visits! {
        visit_bool(value: bool);
        visit_i8(value: i8);
        visit_i16(value: i16);
        visit_i32(value: i32);
        visit_i64(value: i64);
        visit_i128(value: i128);
        visit_u8(value: u8);
        visit_u16(value: u16);
        visit_u32(value: u32);
        visit_u64(value: u64);
        visit_u128(value: u128);
        visit_f32(value: f32);
        visit_f64(value: f64);
        visit_char(value: char);
        visit_str(value: &str);
        visit_borrowed_str(value: &'de str);
        visit_string(value: String);
        visit_bytes(value: &[u8]);
        visit_borrowed_bytes(value: &'de [u8]);
        visit_byte_buf(value: Vec<u8>);
    }

    fn visit_none<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_none()
    }

    fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
        self.0.visit_unit()
    }

    fn visit_some<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.0.visit_some(NamedObjects(decoder))
    }

    fn visit_newtype_struct<D: serde::Deserializer<'de>>(
        self,
        decoder: D,
    ) -> Result<Self::Value, D::Error> {
        self.0.visit_newtype_struct(NamedObjects(decoder))
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
        self.0.visit_seq(Nested(seq))
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
        self.0.visit_map(Nested(map))
    }

    fn visit_enum<A: serde::de::EnumAccess<'de>>(self, value: A) -> Result<Self::Value, A::Error> {
        self.0.visit_enum(Nested(value))
    }
}

impl<'de, S: serde::de::DeserializeSeed<'de>> serde::de::DeserializeSeed<'de> for Nested<S> {
    type Value = S::Value;

    fn deserialize<D: serde::Deserializer<'de>>(self, decoder: D) -> Result<Self::Value, D::Error> {
        self.0.deserialize(NamedObjects(decoder))
    }
}

impl<'de, A: serde::de::MapAccess<'de>> serde::de::MapAccess<'de> for Nested<A> {
    type Error = A::Error;

    fn next_key_seed<K: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        self.0.next_key_seed(Nested(seed))
    }

    fn next_value_seed<S: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.0.next_value_seed(Nested(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}

impl<'de, A: serde::de::SeqAccess<'de>> serde::de::SeqAccess<'de> for Nested<A> {
    type Error = A::Error;

    fn next_element_seed<S: serde::de::DeserializeSeed<'de>>(
        &mut self,
        seed: S,
    ) -> Result<Option<S::Value>, Self::Error> {
        self.0.next_element_seed(Nested(seed))
    }

    fn size_hint(&self) -> Option<usize> {
        self.0.size_hint()
    }
}

impl<'de, A: serde::de::EnumAccess<'de>> serde::de::EnumAccess<'de> for Nested<A> {
    type Error = A::Error;
    type Variant = Nested<A::Variant>;

    fn variant_seed<S: serde::de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<(S::Value, Self::Variant), Self::Error> {
        self.0
            .variant_seed(Nested(seed))
            .map(|(value, variant)| (value, Nested(variant)))
    }
}

impl<'de, A: serde::de::VariantAccess<'de>> serde::de::VariantAccess<'de> for Nested<A> {
    type Error = A::Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        self.0.unit_variant()
    }

    fn newtype_variant_seed<S: serde::de::DeserializeSeed<'de>>(
        self,
        seed: S,
    ) -> Result<S::Value, Self::Error> {
        self.0.newtype_variant_seed(Nested(seed))
    }

    fn tuple_variant<V: serde::de::Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.tuple_variant(len, Nested(visitor))
    }

    fn struct_variant<V: serde::de::Visitor<'de>>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.0.struct_variant(fields, Nested(visitor))
    }
}

/// An invalid explicit valid-time selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidAtError {
    input: String,
}

impl ValidAtError {
    /// The original selector that was refused.
    #[must_use]
    pub fn input(&self) -> &str {
        &self.input
    }
}

impl fmt::Display for ValidAtError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:?} is not a valid-time selector: expected canonical decimal milliseconds or YYYY-MM-DD",
            self.input
        )
    }
}

impl std::error::Error for ValidAtError {}

/// Parse canonical decimal milliseconds or exactly `YYYY-MM-DD` at midnight UTC.
///
/// Decimal selectors retain the full signed range and grammar of [`Timestamp`]. The `time`
/// crate validates the calendar, including a four-digit year zero; no clock or local timezone
/// participates in conversion.
///
/// # Errors
///
/// Refuses invalid dates, noncanonical or out-of-range decimal values, other timestamp
/// spellings and arithmetic overflow.
pub fn parse_valid_at(input: &str) -> Result<Timestamp, ValidAtError> {
    if let Ok(timestamp) = input.parse::<Timestamp>() {
        return Ok(timestamp);
    }
    let refuse = || ValidAtError {
        input: input.to_owned(),
    };
    let bytes = input.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..].iter().all(u8::is_ascii_digit)
    {
        return Err(refuse());
    }
    // The ASCII checks establish valid UTF-8 boundaries before slicing.
    let year = input[..4].parse::<i32>().map_err(|_| refuse())?;
    let month = input[5..7].parse::<u8>().map_err(|_| refuse())?;
    let day = input[8..].parse::<u8>().map_err(|_| refuse())?;
    let month = Month::try_from(month).map_err(|_| refuse())?;
    let date = Date::from_calendar_date(year, month, day).map_err(|_| refuse())?;
    let millis = date
        .midnight()
        .assume_utc()
        .unix_timestamp()
        .checked_mul(1_000)
        .ok_or_else(refuse)?;
    Ok(Timestamp::from_millis(millis))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPERATOR: &str = "00000000-0000-0000-0000-000000000001";
    const VALIDATOR: &str = "00000000-0000-0000-0000-000000000002";
    const DOCUMENT: &str = r#"{
  "format": "ekr.cli-host/1",
  "tenant": "runtime",
  "context": {
    "operator": "00000000-0000-0000-0000-000000000001",
    "validator": "00000000-0000-0000-0000-000000000002"
  },
  "authority": {
    "format": "ekr.authority-state/1",
    "agents": {
      "00000000-0000-0000-0000-000000000001": {
        "id": "00000000-0000-0000-0000-000000000001",
        "name": "Runtime operator",
        "capabilities": ["propose", "read"]
      },
      "00000000-0000-0000-0000-000000000002": {
        "id": "00000000-0000-0000-0000-000000000002",
        "name": "Runtime validator",
        "capabilities": ["validate"]
      }
    },
    "validation_profile": {
      "format": "ekr.p1-validation-profile/1",
      "ruleset": "ekr.p1-deterministic/1",
      "checks": ["Structural", "Reference", "Type", "Cardinality",
                 "OntologyConstraint", "Provenance", "Authorization"],
      "validator": "00000000-0000-0000-0000-000000000002",
      "proposer_separation": "distinct-authenticated-actor/1",
      "provenance": "retained-admissible-evidence/1",
      "application": "ekr.p1-apply/1"
    }
  }
}"#;

    fn decode(input: &str) -> Result<CliHostConfigurationV1, serde_json::Error> {
        CliHostConfigurationV1::from_json(input.as_bytes())
    }

    fn refusal(input: &str, message: &str) {
        let error = decode(input).expect_err("the production decoder must refuse");
        assert!(error.to_string().contains(message), "{error}");
    }

    #[test]
    fn complete_host_preserves_the_actual_nested_authority() {
        let host = decode(DOCUMENT).unwrap();
        assert_eq!(host.format, CliHostFormatV1::V1);
        assert_eq!(host.tenant, "runtime");
        assert_eq!(host.context.operator.to_string(), OPERATOR);
        assert_eq!(host.context.validator.to_string(), VALIDATOR);
        assert_eq!(host.authority.agents.len(), 2);
        let agent = &host.authority.agents[&host.context.operator];
        assert_eq!(agent.id, host.context.operator);
        assert_eq!(agent.name, "Runtime operator");
        assert_eq!(
            agent
                .capabilities
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["propose", "read"]
        );
        assert_eq!(
            host.authority.validation_profile,
            ekr_kernel::ValidationProfileV1::deterministic(host.context.validator)
        );
        let encoded = serde_json::to_vec(&host).unwrap();
        assert_eq!(CliHostConfigurationV1::from_json(&encoded).unwrap(), host);
    }

    #[test]
    fn escaped_names_and_values_are_decoded_without_normalizing_the_carrier() {
        let input = DOCUMENT
            .replacen(
                r#""tenant": "runtime""#,
                r#""ten\u0061nt": "run\u0074ime""#,
                1,
            )
            .replacen(
                r#""00000000-0000-0000-0000-000000000001": {"#,
                r#""\u00300000000-0000-0000-0000-000000000001": {"#,
                1,
            );
        assert_eq!(decode(&input).unwrap(), decode(DOCUMENT).unwrap());
    }

    #[test]
    fn required_outer_fields_cannot_be_missing_null_or_unknown() {
        for field in ["format", "tenant", "context", "authority"] {
            let mut input: serde_json::Value = serde_json::from_str(DOCUMENT).unwrap();
            input.as_object_mut().unwrap().remove(field);
            refusal(&input.to_string(), "missing field");
            input[field] = serde_json::Value::Null;
            assert!(decode(&input.to_string()).is_err(), "{field}");
        }
        refusal(
            &DOCUMENT.replacen('{', r#"{"other":0,"#, 1),
            "unknown field",
        );
    }

    #[test]
    fn the_envelope_format_is_exact() {
        for version in ["ekr.cli-host/2", "EKR.cli-host/1", "ekr.cli-host/1 "] {
            refusal(
                &DOCUMENT.replacen("ekr.cli-host/1", version, 1),
                "unknown variant",
            );
        }
        for input in ["null", "[]", "42", r#""host""#] {
            assert!(decode(input).is_err(), "{input}");
        }
    }

    #[test]
    fn duplicate_outer_fields_refuse_before_decoding_the_repeated_value() {
        for (field, escaped) in [
            ("format", r#"f\u006frmat"#),
            ("tenant", r#"ten\u0061nt"#),
            ("context", r#"cont\u0065xt"#),
            ("authority", r#"auth\u006frity"#),
        ] {
            let input = format!(
                r#"{}, "{escaped}": ["unterminated"#,
                &DOCUMENT[..DOCUMENT.len() - 1]
            );
            refusal(&input, &format!("duplicate field `{field}`"));
        }
    }

    #[test]
    fn nested_unknown_fields_are_refused_by_the_existing_kernel_carriers() {
        for (before, after) in [
            (r#""operator":"#, r#""unknown": 0, "operator":"#),
            (r#""agents":"#, r#""unknown": 0, "agents":"#),
            (r#""name":"#, r#""unknown": 0, "name":"#),
            (r#""ruleset":"#, r#""unknown": 0, "ruleset":"#),
        ] {
            refusal(&DOCUMENT.replacen(before, after, 1), "unknown field");
        }
    }

    #[test]
    fn escaped_duplicate_nested_fields_are_refused_before_their_values() {
        for (before, after, field) in [
            (
                r#""operator": "00000000-0000-0000-0000-000000000001""#,
                r#""operator": "00000000-0000-0000-0000-000000000001", "oper\u0061tor": null"#,
                "operator",
            ),
            (
                r#""name": "Runtime operator""#,
                r#""name": "Runtime operator", "n\u0061me": null"#,
                "name",
            ),
            (
                r#""ruleset": "ekr.p1-deterministic/1""#,
                r#""ruleset": "ekr.p1-deterministic/1", "rule\u0073et": null"#,
                "ruleset",
            ),
        ] {
            refusal(
                &DOCUMENT.replacen(before, after, 1),
                &format!("duplicate field `{field}`"),
            );
        }
        let input = DOCUMENT.replacen(
            r#""validation_profile":"#,
            r#""ag\u0065nts": null, "validation_profile":"#,
            1,
        );
        refusal(&input, "duplicate field `agents`");
    }

    #[test]
    fn decoded_agent_duplicates_refuse_before_the_repeated_agent() {
        let key = format!(r#""{VALIDATOR}": {{"#);
        for duplicate in [
            format!(r#""{OPERATOR}": null, {key}"#),
            format!(r#""\u00300000000-0000-0000-0000-000000000001": null, {key}"#),
        ] {
            refusal(
                &DOCUMENT.replacen(&key, &duplicate, 1),
                "duplicate decoded map key",
            );
        }
    }

    #[test]
    fn capability_duplicates_are_detected_after_json_escape_decoding() {
        for capabilities in [r#"["read", "read"]"#, r#"["read", "r\u0065ad"]"#] {
            refusal(
                &DOCUMENT.replacen(r#"["propose", "read"]"#, capabilities, 1),
                "duplicate decoded set member",
            );
        }
    }

    #[test]
    fn host_parsing_preserves_canonical_identity_spelling() {
        for invalid in [
            "00000000000000000000000000000001",
            "{00000000-0000-0000-0000-000000000001}",
            "00000000-0000-0000-0000-00000000000A",
        ] {
            assert!(
                decode(&DOCUMENT.replace(OPERATOR, invalid)).is_err(),
                "{invalid}"
            );
        }
    }

    #[test]
    fn complete_json_and_utf8_are_required() {
        assert!(decode(&format!(" \n{DOCUMENT}\r\t ")).is_ok());
        for suffix in ["{}", "null", "0", "trailing", "// comment"] {
            assert!(decode(&format!("{DOCUMENT}{suffix}")).is_err(), "{suffix}");
        }
        assert!(decode(&DOCUMENT[..DOCUMENT.len() - 1]).is_err());
        assert!(decode(&format!("\u{feff}{DOCUMENT}")).is_err());
        let mut invalid_utf8 = DOCUMENT.as_bytes().to_vec();
        invalid_utf8.insert(1, 0xff);
        assert!(CliHostConfigurationV1::from_json(&invalid_utf8).is_err());
    }

    #[test]
    fn parsing_does_not_invent_semantic_authority_or_namespace_rules() {
        let input = DOCUMENT
            .replacen(r#""tenant": "runtime""#, r#""tenant": """#, 1)
            .replacen("ekr.p1-deterministic/1", "unsupported-profile", 1);
        let host = decode(&input).unwrap();
        assert_eq!(host.tenant, "");
        assert_eq!(
            host.authority.validation_profile.ruleset,
            "unsupported-profile"
        );
    }

    #[test]
    fn valid_time_decimals_retain_the_full_core_timestamp_range() {
        for value in [i64::MIN, -1, 0, 1, i64::MAX] {
            let text = value.to_string();
            assert_eq!(
                parse_valid_at(&text).unwrap(),
                Timestamp::from_millis(value)
            );
            assert_eq!(
                parse_valid_at(&text).unwrap(),
                text.parse::<Timestamp>().unwrap()
            );
        }
    }

    #[test]
    fn dates_are_midnight_utc_with_leap_and_pre_epoch_controls() {
        for (input, millis) in [
            ("1970-01-01", 0),
            ("1969-12-31", -86_400_000),
            ("2000-02-29", 951_782_400_000),
            ("2024-02-29", 1_709_164_800_000),
            ("0000-01-01", -62_167_219_200_000),
            ("9999-12-31", 253_402_214_400_000),
        ] {
            assert_eq!(parse_valid_at(input).unwrap().millis(), millis, "{input}");
            assert!(input.parse::<Timestamp>().is_err());
        }
    }

    #[test]
    fn invalid_calendars_and_noncanonical_date_spellings_refuse() {
        for input in [
            "1900-02-29",
            "2023-02-29",
            "2024-02-30",
            "2026-04-31",
            "2026-00-12",
            "2026-13-12",
            "2026-03-00",
            "2026-03-32",
            "2026-3-12",
            "2026-03-2",
            "026-03-12",
            "02026-03-12",
            "+2026-03-12",
            "-0001-01-01",
            "2026/03/12",
            "2026-03-12Z",
            "2026-03-12T00:00:00Z",
            " 2026-03-12",
            "2026-03-12\n",
            "２０２６-03-12",
            "2026‐03-12",
            "2026-03-１２",
        ] {
            let error = parse_valid_at(input).expect_err(input);
            assert_eq!(error.input(), input);
        }
    }

    #[test]
    fn overflowing_and_noncanonical_decimal_selectors_refuse() {
        for input in [
            "9223372036854775808",
            "-9223372036854775809",
            "",
            "-",
            "+0",
            "-0",
            "00",
            "01",
            "-01",
            "+1",
            "1.0",
            "1e3",
            " 1",
            "1 ",
            "1\n",
            "NaN",
            "inf",
            "１２",
        ] {
            assert!(parse_valid_at(input).is_err(), "{input:?}");
        }
    }

    #[test]
    fn every_named_struct_requires_json_object_syntax() {
        let original: serde_json::Value = serde_json::from_str(DOCUMENT).unwrap();
        let mut admitted = Vec::new();
        for (path, fields) in [
            ("", vec!["format", "tenant", "context", "authority"]),
            ("/context", vec!["operator", "validator"]),
            ("/authority", vec!["format", "agents", "validation_profile"]),
            (
                "/authority/agents/00000000-0000-0000-0000-000000000001",
                vec!["id", "name", "capabilities"],
            ),
            (
                "/authority/validation_profile",
                vec![
                    "format",
                    "ruleset",
                    "checks",
                    "validator",
                    "proposer_separation",
                    "provenance",
                    "application",
                ],
            ),
        ] {
            let mut input = original.clone();
            let object = input.pointer_mut(path).unwrap();
            let values = fields.iter().map(|field| object[field].clone()).collect();
            *object = serde_json::Value::Array(values);
            let bytes = input.to_string();
            if decode(&bytes).is_ok()
                || serde_json::from_str::<CliHostConfigurationV1>(&bytes).is_ok()
            {
                admitted.push(path);
            }
        }
        assert!(
            admitted.is_empty(),
            "positional arrays admitted at {admitted:?}"
        );
    }

    #[test]
    fn format_uses_a_json_string_and_not_an_externally_tagged_enum_object() {
        let input = DOCUMENT.replacen(
            r#""format": "ekr.cli-host/1""#,
            r#""format": {"ekr.cli-host/1": null}"#,
            1,
        );
        assert!(decode(&input).is_err());
        assert!(serde_json::from_str::<CliHostConfigurationV1>(&input).is_err());
    }

    #[test]
    fn capabilities_and_ordered_profile_checks_keep_their_real_list_syntax() {
        let input = DOCUMENT.replacen(r#"["propose", "read"]"#, "[]", 1);
        let host = decode(&input).unwrap();
        assert!(host.authority.agents[&host.context.operator]
            .capabilities
            .is_empty());
        assert_eq!(host.authority.validation_profile.checks.len(), 7);
        for path in [
            "/authority/agents/00000000-0000-0000-0000-000000000001/capabilities",
            "/authority/validation_profile/checks",
        ] {
            let mut document: serde_json::Value = serde_json::from_str(DOCUMENT).unwrap();
            *document.pointer_mut(path).unwrap() = serde_json::json!({});
            assert!(decode(&document.to_string()).is_err(), "{path}");
        }
    }

    #[test]
    fn required_fields_of_nested_carriers_cannot_be_missing_or_null() {
        for (path, fields) in [
            ("/context", vec!["operator", "validator"]),
            ("/authority", vec!["format", "agents", "validation_profile"]),
            (
                "/authority/agents/00000000-0000-0000-0000-000000000001",
                vec!["id", "name", "capabilities"],
            ),
            (
                "/authority/validation_profile",
                vec![
                    "format",
                    "ruleset",
                    "checks",
                    "validator",
                    "proposer_separation",
                    "provenance",
                    "application",
                ],
            ),
        ] {
            for field in fields {
                let mut document: serde_json::Value = serde_json::from_str(DOCUMENT).unwrap();
                let object = document.pointer_mut(path).unwrap().as_object_mut().unwrap();
                object.remove(field);
                refusal(&document.to_string(), "missing field");
                document.pointer_mut(path).unwrap()[field] = serde_json::Value::Null;
                assert!(decode(&document.to_string()).is_err(), "{path}/{field}");
            }
        }
    }
}
