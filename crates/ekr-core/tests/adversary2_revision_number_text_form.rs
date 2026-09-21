//! `RevisionNumber` keeps the one-text rule on `FromStr` and drops it on the boundary that matters.
//!
//! The correction round gave `RevisionNumber` a strict `FromStr` and its own
//! `RevisionNumberParseError`, documented as: "Reads the decimal form `Display` writes, and no
//! other spelling of it … A number is not exempt from the one-text rule." The suite states the
//! same class in `identity_serde.rs`: "A type whose `Display` documents one form must refuse every
//! other spelling of it, whatever the form is made of."
//!
//! But `RevisionNumber` derives `Serialize`/`Deserialize` as `#[serde(transparent)]` over `u64`,
//! so the serde path never reaches `FromStr`. The story's own reason for caring about text — "an
//! id crosses a process boundary as text: a stored record, an event payload, a CLI argument" — is
//! about the serde path, not the `FromStr` path, and that is the path the rule is not kept on.
//!
//! These cases state the rule where the boundary is, and pin the leading-zero refusal at the
//! lengths the existing cases do not reach.

use ekr_core::RevisionNumber;

/// Every JSON text that a `RevisionNumber` accepts must be the text it writes back.
///
/// This is the one-text rule as a round-trip, so it names no spelling in particular: a second
/// spelling is any text that is read and then written differently.
#[test]
fn serde_reads_the_number_it_writes_and_no_other_spelling_of_it() {
    let candidates = [
        "0",
        "7",
        "-0",
        "+7",
        "007",
        "0e0",
        "7.0",
        "18446744073709551615",
    ];

    let mut second_spellings: Vec<(&str, String)> = Vec::new();
    for candidate in candidates {
        let Ok(number) = serde_json::from_str::<RevisionNumber>(candidate) else {
            continue;
        };
        let written = serde_json::to_string(&number).expect("a revision number serialises");
        if written != candidate {
            second_spellings.push((candidate, written));
        }
    }

    assert_eq!(
        second_spellings,
        Vec::<(&str, String)>::new(),
        "a stored record carrying a second spelling of a revision number was accepted, and writes \
         back as a different text"
    );
}

/// The refusal `FromStr` performs and serde does not, stated on its own so the disagreement is
/// visible rather than inferred.
#[test]
fn the_two_reading_paths_agree_about_what_a_revision_number_is() {
    let mut disagreements: Vec<&str> = Vec::new();
    for candidate in ["0", "7", "-0", "18446744073709551615"] {
        let by_parse = candidate.parse::<RevisionNumber>().is_ok();
        let by_serde = serde_json::from_str::<RevisionNumber>(candidate).is_ok();
        if by_parse != by_serde {
            disagreements.push(candidate);
        }
    }

    assert_eq!(
        disagreements,
        Vec::<&str>::new(),
        "FromStr and Deserialize disagree about which texts are a revision number"
    );
}

/// A leading zero is refused whatever the length — `00` included, which is the shortest one and
/// the one `identity_serde.rs`'s list and its `0{text}` proptest between them never reach for the
/// seed.
///
/// Green today. It is here because `from_str`'s guard is `text.len() > 1 && starts_with('0')`, and
/// nothing in the suite goes red if that `1` becomes a `2`.
#[test]
fn a_leading_zero_is_refused_at_every_length() {
    for padded in [
        "00",
        "000",
        "01",
        "0000000000000000000",
        "00000000000000000000",
    ] {
        assert!(
            padded.parse::<RevisionNumber>().is_err(),
            "parsing {padded:?} must be refused: it is not the text `Display` writes"
        );
    }

    assert_eq!(
        "0".parse::<RevisionNumber>().expect("zero parses"),
        RevisionNumber::SEED,
        "the one spelling of the seed still reads"
    );
}
