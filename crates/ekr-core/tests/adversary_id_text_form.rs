//! An id has one text form, or it has none — the same rule `ContentHash` already keeps.
//!
//! `identity.rs` documents its `Display` as "The hyphenated lowercase UUID form — what serde
//! writes and [`FromStr`] reads", and `hash.rs` states the reason a content address is read in one
//! spelling only: "two spellings of one address are two addresses to everything that compares
//! text". An id crosses the same boundaries as a hash — a stored record, an event payload, a map
//! key, a file name — so the two types cannot disagree about whether a second spelling is a
//! record the runtime accepts.
//!
//! `FromStr` delegates to `uuid::Uuid::parse_str`, which accepts the simple, braced and URN
//! wrappings as well as the hyphenated one, and uppercase hex in all four. These cases state the
//! documented contract: everything but the form `Display` writes is refused.

use ekr_core::{AgentId, EdgeId, NodeId};

/// Every spelling of `canonical` that is not `canonical` itself but which `uuid` accepts.
fn other_spellings(canonical: &str) -> Vec<(&'static str, String)> {
    let bare = canonical.replace('-', "");
    vec![
        ("uppercase hyphenated", canonical.to_ascii_uppercase()),
        ("simple, unhyphenated", bare.clone()),
        ("simple, uppercase", bare.to_ascii_uppercase()),
        ("braced", format!("{{{canonical}}}")),
        ("urn", format!("urn:uuid:{canonical}")),
    ]
}

/// The spellings `FromStr` accepts beside the one `Display` writes.
fn accepted_by_from_str<T>(canonical: &str) -> Vec<&'static str>
where
    T: std::str::FromStr,
{
    other_spellings(canonical)
        .into_iter()
        .filter(|(_, text)| text.parse::<T>().is_ok())
        .map(|(label, _)| label)
        .collect()
}

/// The spellings `serde` accepts beside the one it writes.
fn accepted_by_serde<T>(canonical: &str) -> Vec<&'static str>
where
    T: serde::de::DeserializeOwned,
{
    other_spellings(canonical)
        .into_iter()
        .filter(|(_, text)| serde_json::from_str::<T>(&format!("\"{text}\"")).is_ok())
        .map(|(label, _)| label)
        .collect()
}

#[test]
fn an_id_parses_from_the_form_it_writes_and_from_no_other() {
    let node = NodeId::mint().to_string();
    assert_eq!(
        accepted_by_from_str::<NodeId>(&node),
        Vec::<&str>::new(),
        "FromStr read spellings Display never writes"
    );

    let edge = EdgeId::mint().to_string();
    assert_eq!(
        accepted_by_from_str::<EdgeId>(&edge),
        Vec::<&str>::new(),
        "FromStr read spellings Display never writes"
    );
}

#[test]
fn serde_reads_the_form_it_writes_and_no_other() {
    let agent = AgentId::mint().to_string();
    assert_eq!(
        accepted_by_serde::<AgentId>(&agent),
        Vec::<&str>::new(),
        "a stored record carrying a second spelling of an id was accepted"
    );
}

#[test]
fn a_second_spelling_does_not_round_trip_to_itself() {
    // The consequence the adversary wrote this case for: text in, different text out. Anything
    // that compares the two — a cache key, a snapshot diff, a JSON document held for byte
    // equality — sees a change that is not one.
    //
    // The case was written against the defect, so its premise was `expect("accepted today")` on
    // the urn form; once `FromStr` is strict that premise is gone and the panic is the *fix*
    // firing. The assertion below is the same statement with the premise made explicit and
    // stronger: a second spelling never enters, so no value can be written back under a text
    // other than the one it reads. Nothing was removed — the round-trip assertion that follows
    // it is the same one, over the only spelling that survives.
    let canonical = NodeId::mint().to_string();
    let urn = format!("urn:uuid:{canonical}");

    let refused = serde_json::from_str::<NodeId>(&format!("\"{urn}\""));
    assert!(
        refused.is_err(),
        "a second spelling was accepted, so an id read from one text writes a different text"
    );

    let parsed: NodeId = serde_json::from_str(&format!("\"{canonical}\"")).expect("the one form");
    assert_eq!(
        serde_json::to_string(&parsed).expect("serialises"),
        format!("\"{canonical}\""),
        "an id that was read from one text writes a different text"
    );
}
