//! Byte strings in retained JSON: one canonical base64 spelling, and the two spellings a
//! versioned format chooses between.
use ekr_core::bytes::{self, Spell, Spelled};
use proptest::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Held {
    #[serde(with = "ekr_core::bytes::base64")]
    bytes: Vec<u8>,
}

proptest! {
    /// Every byte string has exactly one spelling, and it reads back as those bytes.
    #[test]
    fn every_byte_string_round_trips_through_its_one_spelling(held in proptest::collection::vec(any::<u8>(), 0..512)) {
        let text = bytes::encode(&held);
        prop_assert_eq!(text.len(), held.len().div_ceil(3) * 4);
        prop_assert_eq!(bytes::decode(&text).unwrap(), held.clone());
        let json = serde_json::to_string(&Held { bytes: held.clone() }).unwrap();
        prop_assert_eq!(&json, &format!(r#"{{"bytes":"{text}"}}"#));
        prop_assert_eq!(serde_json::from_str::<Held>(&json).unwrap(), Held { bytes: held });
    }
}

/// Base64 of a payload is about four bytes for every three, where the number array a `Vec<u8>`
/// serialises as by default is more than three for every one.
#[test]
fn base64_is_a_third_larger_and_a_number_array_several_times_larger() {
    let payload: Vec<u8> = (0..30_000_u32).map(|n| (n * 7 % 251) as u8).collect();
    let compact = serde_json::to_vec(&Spell {
        bytes: &payload,
        numbers: false,
    })
    .unwrap();
    let numbers = serde_json::to_vec(&Spell {
        bytes: &payload,
        numbers: true,
    })
    .unwrap();
    assert_eq!(numbers, serde_json::to_vec(&payload).unwrap());
    assert!(
        compact.len() <= payload.len() * 4 / 3 + 4,
        "{}",
        compact.len()
    );
    assert!(numbers.len() > payload.len() * 3, "{}", numbers.len());
}

/// The `with` module writes and reads exactly that one string.
#[test]
fn the_serde_module_writes_and_reads_the_one_string() {
    let mut out = Vec::new();
    bytes::base64::serialize(&[0xff, 0xfe], &mut serde_json::Serializer::new(&mut out)).unwrap();
    assert_eq!(out, br#""//4=""#);
    let read = bytes::base64::deserialize(&mut serde_json::Deserializer::from_slice(&out)).unwrap();
    assert_eq!(read, [0xff, 0xfe]);
}

/// A reader learns which spelling it met, so a format can admit only its own.
#[test]
fn a_reader_learns_the_spelling_and_refuses_a_loose_one() {
    let text: Spelled = serde_json::from_str(r#""AQID""#).unwrap();
    assert_eq!(
        text,
        Spelled {
            bytes: vec![1, 2, 3],
            numbers: false
        }
    );
    let numbers: Spelled = serde_json::from_str("[1,2,3]").unwrap();
    assert_eq!(
        numbers,
        Spelled {
            bytes: vec![1, 2, 3],
            numbers: true
        }
    );
    let loose: bytes::Base64Error = bytes::decode("AQJ=").unwrap_err();
    assert!(loose.to_string().contains("nonzero bits"), "{loose}");
    for refused in [r#""AQJ=""#, r#""AQID=""#, r#""AQ I""#, "[256]", "{}", "1"] {
        assert!(
            serde_json::from_str::<Spelled>(refused).is_err(),
            "{refused}"
        );
        assert!(serde_json::from_str::<Held>(&format!(r#"{{"bytes":{refused}}}"#)).is_err());
    }
}
