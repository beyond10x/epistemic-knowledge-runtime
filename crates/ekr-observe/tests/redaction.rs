//! `story:credential-redaction-before-model-input`: credentials are replaced in the model-facing
//! projection of an observed text, each replacement is recorded by shape and span only, the
//! observed bytes stay as they were, and a text with no credential passes through byte-identical.
//!
//! Every credential below is synthetic and assembled at run time from pieces, so no
//! credential-shaped literal sits in this file for a secret scan to find, and none is real.

use ekr_observe::observe_jsonl;
use ekr_observe::redact::{redact, CredentialShape, ModelInput, Redaction, RedactionSpan};

/// One synthetic credential of `shape`, built from pieces.
fn credential(shape: CredentialShape) -> String {
    match shape {
        CredentialShape::AwsAccessKeyId => ["AK", "IA", "FIXTURE0", "FIXTURE1"].concat(),
        CredentialShape::GitHubToken => ["gh", "p_", &"Fixture".repeat(5), "x"].concat(),
        CredentialShape::SlackToken => ["xo", "xb-", "0000000000-", "fixture-token"].concat(),
        CredentialShape::JsonWebToken => [
            "ey",
            "JhbGciOiJub25lIn0",
            ".",
            "ey",
            "JzdWIiOiJmaXh0dXJlIn0",
            ".",
            "Zml4dHVyZS1zaWduYXR1cmU",
        ]
        .concat(),
        CredentialShape::BearerToken => ["fixture", "-opaque-", "token-0123456789"].concat(),
        CredentialShape::PrivateKeyBlock => [
            "-----BEGIN ",
            "RSA PRIVATE KEY-----\n",
            "Zml4dHVyZSBrZXkgbWF0ZXJpYWw=\n",
            "-----END ",
            "RSA PRIVATE KEY-----",
        ]
        .concat(),
    }
}

/// The text a credential of `shape` is observed in: before it, the credential, after it.
fn observed_text(shape: CredentialShape) -> (String, String) {
    let secret = credential(shape);
    let before = match shape {
        CredentialShape::BearerToken => "the request carried Authorization: Bearer ",
        _ => "the deploy log printed ",
    };
    (format!("{before}{secret} and then stopped."), secret)
}

/// One JSONL source record holding `text`, as the observation layer reads it.
fn record_line(text: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "source": "fixture-source-a",
        "source_native_id": "record-0001",
        "captured_at": 1_767_225_600_000_i64,
        "text": text,
    }))
    .expect("a record serialises")
}

/// The `text` of a stored source record line, read back.
fn text_of(line: &[u8]) -> String {
    let value: serde_json::Value = serde_json::from_slice(line).expect("a stored record parses");
    value["text"].as_str().expect("text is a string").to_owned()
}

/// Acceptance 1: each credential shape in the corpus is replaced in the model-facing text, and
/// the stored observation's bytes are unchanged.
#[test]
fn each_credential_shape_is_replaced_and_the_stored_bytes_are_unchanged() {
    for shape in CredentialShape::ALL {
        let (text, secret) = observed_text(shape);
        let stored = record_line(&text);
        let kept = stored.clone();
        let observation = observe_jsonl(&stored).expect("the record maps").remove(0);

        let projection: ModelInput = redact(&text_of(&stored));

        assert!(
            !projection.text.contains(&secret),
            "{shape}: the credential reached the model-facing text: {:?}",
            projection.text
        );
        assert!(
            projection.text.contains(&shape.marker()),
            "{shape}: no marker naming the shape in {:?}",
            projection.text
        );
        assert_eq!(
            projection
                .redactions
                .iter()
                .map(|r| r.shape)
                .collect::<Vec<_>>(),
            vec![shape],
            "{shape}: exactly one redaction, of this shape"
        );
        assert_eq!(stored, kept, "{shape}: the stored record's bytes changed");
        assert_eq!(
            observe_jsonl(&stored)
                .expect("the record still maps")
                .remove(0),
            observation,
            "{shape}: the observation of the stored bytes changed"
        );
    }
}

/// Acceptance 1, whole corpus at once: every shape in one text, each replaced, the spans in order
/// and disjoint, and the text outside the spans kept exactly.
#[test]
fn every_shape_in_one_text_is_replaced_in_span_order() {
    let mut text = String::from("corpus:");
    let mut secrets = Vec::new();
    for shape in CredentialShape::ALL {
        let (piece, secret) = observed_text(shape);
        text.push('\n');
        text.push_str(&piece);
        secrets.push((shape, secret));
    }

    let projection = redact(&text);

    assert_eq!(
        projection
            .redactions
            .iter()
            .map(|r| r.shape)
            .collect::<Vec<_>>(),
        CredentialShape::ALL.to_vec()
    );
    let mut rebuilt = String::new();
    let mut at = 0;
    for (redaction, (_, secret)) in projection.redactions.iter().zip(&secrets) {
        let span = redaction.span;
        assert!(span.start() >= at, "spans overlap or run backwards");
        assert_eq!(&text[span.range()], secret, "{} span", redaction.shape);
        rebuilt.push_str(&text[at..span.start()]);
        rebuilt.push_str(&redaction.shape.marker());
        at = span.end();
    }
    rebuilt.push_str(&text[at..]);
    assert_eq!(projection.text, rebuilt);
}

/// Acceptance 2: the redaction record names shape and span and holds no credential bytes.
#[test]
fn the_redaction_record_names_shape_and_span_and_holds_no_credential_bytes() {
    for shape in CredentialShape::ALL {
        let (text, secret) = observed_text(shape);
        let projection = redact(&text);
        assert_eq!(projection.redactions.len(), 1, "{shape}");

        // Exhaustive: a field added to the record fails to compile here.
        let Redaction {
            shape: named,
            span: _,
        } = projection.redactions[0];
        let span: RedactionSpan = projection.redactions[0].span;
        assert_eq!(named, shape);
        assert!(span.length() >= 1, "{shape}: an empty span");
        assert_eq!(
            &text.as_bytes()[span.range()],
            secret.as_bytes(),
            "{shape}: the span is not where the credential sat"
        );

        // No eight-byte window of the credential appears in any rendering of the record.
        let renderings = [
            format!("{:?}", projection.redactions[0]),
            format!("{:#?}", projection.redactions),
        ];
        for rendering in &renderings {
            for window in secret.as_bytes().windows(8) {
                let window = std::str::from_utf8(window).expect("ascii fixture");
                assert!(
                    !rendering.contains(window),
                    "{shape}: the record carries credential bytes {window:?}: {rendering}"
                );
            }
        }
    }
}

/// Acceptance 3: a text with no credential passes through byte-identical, near misses included.
#[test]
fn a_text_with_no_credential_passes_through_byte_identical() {
    let clean = [
        String::new(),
        "An assertion was proposed against the canonical root.".to_owned(),
        "Grüße aus der Beobachtungsschicht: ✓ — nothing secret here.\n\ttabs and lines\n"
            .to_owned(),
        // One character short of an access key id.
        ["AK", "IA", "FIXTURE0", "FIXTURE"].concat(),
        // Inside a longer word, so not a key id at all.
        ["XAK", "IA", "FIXTURE0", "FIXTURE1"].concat(),
        // A token prefix with too short a body.
        ["gh", "p_", "short"].concat(),
        // The word bearer before ordinary prose.
        "The bearer of this note is welcome.".to_owned(),
        // A JSON-looking fragment that is not three segments.
        ["ey", "Jfixture", ".only-two"].concat(),
        // A public key is not a private one.
        [
            "-----BEGIN ",
            "PUBLIC KEY-----\nZml4dHVyZQ==\n-----END ",
            "PUBLIC KEY-----",
        ]
        .concat(),
        "[REDACTED:AwsAccessKeyId] is a marker, not a credential.".to_owned(),
    ];
    for text in &clean {
        let projection = redact(text);
        assert_eq!(
            projection.text.as_bytes(),
            text.as_bytes(),
            "a clean text changed"
        );
        assert!(
            projection.redactions.is_empty(),
            "{text:?}: {:?}",
            projection.redactions
        );
    }
}

/// A private key whose end line is missing is redacted to the end of the text rather than left
/// half visible.
#[test]
fn a_private_key_without_its_end_line_is_redacted_to_the_end() {
    let text = ["key: -----BEGIN ", "PRIVATE KEY-----\nZml4dHVyZQ==\nZml4"].concat();
    let projection = redact(&text);
    assert_eq!(projection.text, "key: [REDACTED:PrivateKeyBlock]");
    assert_eq!(projection.redactions[0].span.end(), text.len());
}

/// The shapes the crate detects are the shapes `ekr.observe.CredentialShape` declares, in order,
/// by name. The document and the code are read together here, since `ess specify validate` reads
/// only the document.
#[test]
fn the_detected_shapes_are_the_declared_shapes() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let path = std::path::PathBuf::from(root).join("../../systems/ekr/domains/observe.yaml");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()));
    let domain: serde_yaml_ng::Value = serde_yaml_ng::from_str(&text).expect("the domain parses");
    let declared = domain["types"]
        .as_sequence()
        .expect("types")
        .iter()
        .find(|t| t["name"].as_str() == Some("ekr.observe.CredentialShape"))
        .expect("observe.yaml declares ekr.observe.CredentialShape");
    let variants: Vec<&str> = declared["variants"]
        .as_sequence()
        .expect("variants")
        .iter()
        .map(|v| v.as_str().expect("a variant is a name"))
        .collect();
    assert_eq!(
        CredentialShape::ALL.map(CredentialShape::name).to_vec(),
        variants
    );

    let fields_of = |name: &str| -> Vec<String> {
        domain["types"]
            .as_sequence()
            .expect("types")
            .iter()
            .find(|t| t["name"].as_str() == Some(name))
            .unwrap_or_else(|| panic!("observe.yaml declares {name}"))["fields"]
            .as_sequence()
            .expect("fields")
            .iter()
            .map(|f| f["name"].as_str().expect("a field name").to_owned())
            .collect()
    };
    assert_eq!(fields_of("ekr.observe.Redaction"), ["shape", "span"]);
    assert_eq!(fields_of("ekr.observe.RedactionSpan"), ["start", "length"]);
    assert_eq!(fields_of("ekr.observe.ModelInput"), ["text", "redactions"]);
}

/// The observation layer's `redact` is `ekr_core::redact`'s, re-exported, not a second detector:
/// the assignment below compiles only if both name one function, and both project alike.
#[test]
fn the_observation_layer_calls_the_one_detector() {
    let core: fn(&str) -> ekr_core::redact::ModelInput = ekr_observe::redact::redact;
    let (text, _) = observed_text(CredentialShape::GitHubToken);
    assert_eq!(redact(&text), core(&text));
}
