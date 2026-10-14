//! `ekr_core::redact`, held by this crate's own suite: each public item of the module is used here
//! with an assertion that fails if it behaves otherwise. The acceptance cases of
//! `story:credential-redaction-before-model-input` run through the observation layer, in
//! `crates/ekr-observe/tests/redaction.rs`.
//!
//! The credential is synthetic and assembled at run time, so no credential-shaped literal sits in
//! this file.

use ekr_core::redact::{redact, CredentialShape, ModelInput, Redaction, RedactionSpan};

#[test]
fn shapes_name_themselves_and_their_markers() {
    let names: Vec<&str> = CredentialShape::ALL
        .iter()
        .map(|shape| shape.name())
        .collect();
    assert_eq!(
        names,
        [
            "AwsAccessKeyId",
            "GitHubToken",
            "SlackToken",
            "JsonWebToken",
            "BearerToken",
            "PrivateKeyBlock",
        ]
    );
    for shape in CredentialShape::ALL {
        assert_eq!(shape.marker(), format!("[REDACTED:{}]", shape.name()));
        assert_eq!(shape.to_string(), shape.name());
    }
}

#[test]
fn a_span_is_a_start_and_a_length_in_the_text_as_observed() {
    let secret = ["AK", "IA", "FIXTURE0", "FIXTURE1"].concat();
    let text = format!("ab {secret} cd");

    let projection: ModelInput = redact(&text);

    assert_eq!(projection.text, "ab [REDACTED:AwsAccessKeyId] cd");
    let [Redaction { shape, span }] = projection.redactions[..] else {
        panic!("one redaction: {:?}", projection.redactions);
    };
    let span: RedactionSpan = span;
    assert_eq!(shape, CredentialShape::AwsAccessKeyId);
    assert_eq!(span.start(), 3);
    assert_eq!(span.length(), secret.len());
    assert_eq!(span.end(), 3 + secret.len());
    assert_eq!(span.range(), 3..3 + secret.len());
    assert_eq!(&text[span.range()], secret);
}
