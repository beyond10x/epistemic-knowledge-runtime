//! Conformance checks of `ekr_core::redaction` against its own contract and invariant 9: every
//! credential of a declared shape is replaced in the model-facing text, each redaction's span
//! points at exactly the bytes it replaced, and adversarial input neither panics nor leaks.
//!
//! Every credential-shaped value is synthetic and assembled at run time from pieces, so no
//! credential-shaped literal sits in this file.

use ekr_core::redaction::{redact_for_model, CredentialShape, ModelInput};

fn aws() -> String {
    ["AK", "IA", "FIXTURE0", "FIXTURE1"].concat()
}

fn github() -> String {
    ["gh", "p_", &"Fixture".repeat(5), "x"].concat()
}

fn slack() -> String {
    ["xo", "xb-", "0000000000-", "fixture-token"].concat()
}

fn jwt() -> String {
    [
        "ey",
        "JhbGciOiJub25lIn0",
        ".",
        "ey",
        "JzdWIiOiJmaXh0dXJlIn0",
        ".",
        "Zml4dHVyZS1zaWduYXR1cmU",
    ]
    .concat()
}

const KEY_BODY: &str = "Zml4dHVyZSBrZXkgbWF0ZXJpYWw";

fn pem(label: &str, newline: &str) -> String {
    [
        "-----", "BEGIN ", label, "-----", newline, KEY_BODY, newline, "-----", "END ", label,
        "-----",
    ]
    .concat()
}

fn private_key() -> String {
    pem("RSA PRIVATE KEY", "\n")
}

/// The secret bytes that must not reach the model for each shape: for a PEM block its body.
fn shapes() -> Vec<(CredentialShape, String, String)> {
    vec![
        (CredentialShape::AwsAccessKeyId, aws(), aws()),
        (CredentialShape::GitHubToken, github(), github()),
        (CredentialShape::SlackToken, slack(), slack()),
        (CredentialShape::JsonWebToken, jwt(), jwt()),
        (
            CredentialShape::PrivateKeyBlock,
            private_key(),
            KEY_BODY.to_owned(),
        ),
    ]
}

/// Rebuilding the projection from the observed text and the recorded spans must give the
/// projection's text: each span is exactly the bytes its marker replaced.
fn assert_spans_rebuild(observed: &str, projection: &ModelInput) {
    let mut rebuilt = String::new();
    let mut kept = 0;
    for redaction in &projection.redactions {
        let range = redaction.span.range();
        assert!(range.start >= kept, "spans overlap or go backwards");
        assert!(redaction.span.length() >= 1, "an empty span");
        assert!(observed.is_char_boundary(range.start) && observed.is_char_boundary(range.end));
        rebuilt.push_str(&observed[kept..range.start]);
        rebuilt.push_str(&redaction.shape.marker());
        kept = range.end;
    }
    rebuilt.push_str(&observed[kept..]);
    assert_eq!(
        rebuilt, projection.text,
        "spans do not point at the replaced bytes"
    );
}

/// Observed text is often a serialised record or a URL, where a credential follows an escape
/// (`\n`, `\t`) or a percent-encoded `=` (`%3D`). Its letter is a word byte to the detector's
/// anchor, so the credential is not detected.
#[test]
fn a_credential_after_an_escape_or_a_percent_encoding_is_replaced() {
    let mut escaped = Vec::new();
    for lead in ["\\n", "\\t", "%3D"] {
        for (shape, credential, secret) in shapes() {
            let observed = format!("{{\"text\":\"line one{lead}{credential}{lead}line two\"}}");
            let projection = redact_for_model(&observed);
            assert_spans_rebuild(&observed, &projection);
            if projection.text.contains(&secret) {
                escaped.push(format!("{shape} after {lead}"));
            }
        }
    }
    assert!(
        escaped.is_empty(),
        "credentials reached the model-facing text: {escaped:?}"
    );
}

/// An OpenPGP private key is armoured with a `BEGIN PGP PRIVATE KEY BLOCK` line between runs of
/// five hyphens; its key material is replaced, not only PEM's.
#[test]
fn an_armoured_pgp_private_key_is_replaced() {
    let block = pem("PGP PRIVATE KEY BLOCK", "\n");
    let observed = format!("exported:\n{block}\n");
    let projection = redact_for_model(&observed);
    assert_spans_rebuild(&observed, &projection);
    assert!(
        !projection.text.contains(KEY_BODY),
        "the private key body reached the model-facing text: {:?}",
        projection.text
    );
}

/// HTTP authorisation schemes are case-insensitive and the separator is one or more spaces
/// (RFC 9110 § 11.1, § 11.4), or `%20` when the header travels in a URL; each form below carries
/// the same credential.
#[test]
fn a_bearer_credential_in_any_scheme_spelling_is_replaced() {
    let token = ["fixture", "-opaque-", "token-0123456789"].concat();
    let mut escaped = Vec::new();
    for scheme in ["BEARER ", "Bearer  ", "Bearer\t", "BeArEr ", "Bearer%20"] {
        let observed = format!("Authorization: {scheme}{token}\r\n");
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        if projection.text.contains(&token) {
            escaped.push(format!("{scheme:?}"));
        }
    }
    assert!(
        escaped.is_empty(),
        "bearer credentials reached the model-facing text after {escaped:?}"
    );
}

/// A token whose run admits `-` (Slack, a JWT signature) swallows the armour that opens a
/// following PEM block, and the key body after it is left in the text.
#[test]
fn a_private_key_directly_after_a_token_is_still_replaced() {
    for credential in [slack(), jwt()] {
        let observed = format!("{credential}{}", private_key());
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        assert!(
            !projection.text.contains(KEY_BODY),
            "the private key body reached the model-facing text: {:?}",
            projection.text
        );
    }
}

/// Green: CRLF and unterminated PEM blocks, credentials at both text boundaries, adjacent
/// credentials and multi-byte UTF-8 neighbours are all replaced, with spans that rebuild.
#[test]
fn boundaries_crlf_unterminated_and_utf8_neighbours_are_replaced() {
    let crlf = pem("EC PRIVATE KEY", "\r\n");
    let unterminated = ["-----", "BEGIN PRIVATE KEY-----\r\n", KEY_BODY, "\r\n"].concat();
    let cases = [
        aws(),
        format!("{} {}", github(), github()),
        format!("é{}ü", aws()),
        format!("→{}\u{1F512}", jwt()),
        format!("{}\n{}", slack(), aws()),
        crlf,
        format!("ß{unterminated}"),
    ];
    for observed in cases {
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        for secret in [aws(), github(), slack(), jwt(), KEY_BODY.to_owned()] {
            assert!(
                !projection.text.contains(&secret),
                "{observed:?} leaked into {:?}",
                projection.text
            );
        }
    }
}

/// Green: a fixed-seed corpus of credential pieces, markers and multi-byte characters never
/// panics, and every projection's spans rebuild it.
#[test]
fn adversarial_corpus_never_panics_and_spans_rebuild() {
    let pieces: Vec<String> = vec![
        aws(),
        github(),
        slack(),
        jwt(),
        private_key(),
        ["-----", "BEGIN PRIVATE KEY-----"].concat(),
        ["-----", "END PRIVATE KEY-----"].concat(),
        "Bearer ".to_owned(),
        "eyJ".to_owned(),
        "xoxb-".to_owned(),
        "AKIA".to_owned(),
        "-".to_owned(),
        ".".to_owned(),
        "_".to_owned(),
        " ".to_owned(),
        "\r\n".to_owned(),
        "é".to_owned(),
        "\u{1F512}".to_owned(),
        "[REDACTED:SlackToken]".to_owned(),
    ];
    let mut state: u64 = 0x5eed_0014_a0c0_ffee;
    for _ in 0..2_000 {
        let mut observed = String::new();
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let count = (state % 12) as usize;
        for _ in 0..count {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            observed.push_str(&pieces[(state % pieces.len() as u64) as usize]);
        }
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        if projection.redactions.is_empty() {
            assert_eq!(projection.text, observed);
        }
    }
}

/// `n` letters or digits, cycling, so no run of the fixture reads as a word.
fn body(n: usize) -> String {
    "Fixture0".chars().cycle().take(n).collect()
}

/// `n` upper-case letters or digits, cycling.
fn upper(n: usize) -> String {
    "FIXTURE0".chars().cycle().take(n).collect()
}

/// A JSON Web Token whose segments are each exactly `n` characters (`n >= 3`).
fn jwt_of(n: usize) -> String {
    let lead = ["ey", "J"].concat();
    [
        lead.clone(),
        body(n - 3),
        ".".to_owned(),
        lead,
        body(n - 3),
        ".".to_owned(),
        body(n),
    ]
    .concat()
}

/// Green when the contract holds: every prefix, scheme spelling and armour label the doc comments
/// of `CredentialShape` declare is replaced at its stated minimum length. The unit's own suite uses
/// one prefix per shape (`AKIA`, `ghp_`, `xoxb-`, `Bearer `), so dropping any other declared prefix
/// leaves it green; this case is the measure of the rest.
#[test]
fn every_declared_prefix_is_replaced_at_its_minimum_length() {
    let mut cases: Vec<(CredentialShape, String, String)> = Vec::new();
    for prefix in [["AK", "IA"].concat(), ["AS", "IA"].concat()] {
        let credential = [prefix, upper(16)].concat();
        cases.push((
            CredentialShape::AwsAccessKeyId,
            credential.clone(),
            credential,
        ));
    }
    for kind in ["p", "o", "u", "s", "r"] {
        let credential = ["gh", kind, "_", &body(36)].concat();
        cases.push((CredentialShape::GitHubToken, credential.clone(), credential));
    }
    let pat = ["github", "_pat_", &body(22)].concat();
    cases.push((CredentialShape::GitHubToken, pat.clone(), pat));
    for kind in ["a", "b", "p", "r", "s"] {
        let credential = ["xo", "x", kind, "-", &body(10)].concat();
        cases.push((CredentialShape::SlackToken, credential.clone(), credential));
    }
    let jwt = jwt_of(10);
    cases.push((CredentialShape::JsonWebToken, jwt.clone(), jwt));
    for scheme in ["Bearer ", "bearer "] {
        let token = body(16);
        cases.push((
            CredentialShape::BearerToken,
            [scheme, &token].concat(),
            token,
        ));
    }
    for label in [
        "PRIVATE KEY",
        "RSA PRIVATE KEY",
        "EC PRIVATE KEY",
        "OPENSSH PRIVATE KEY",
        "ENCRYPTED PRIVATE KEY",
    ] {
        cases.push((
            CredentialShape::PrivateKeyBlock,
            pem(label, "\n"),
            KEY_BODY.to_owned(),
        ));
    }
    let mut missed = Vec::new();
    for (shape, credential, secret) in cases {
        let observed = format!("value: {credential} end");
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        let shapes: Vec<CredentialShape> = projection.redactions.iter().map(|r| r.shape).collect();
        if projection.text.contains(&secret) || shapes != [shape] {
            missed.push(format!("{shape}: {:?} -> {shapes:?}", &credential[..6]));
        }
    }
    assert!(missed.is_empty(), "declared forms not replaced: {missed:?}");
}

/// Green when the contract holds: one character short of each stated minimum, and an access key
/// id one character long, is not a credential of that shape and passes through byte-identical.
/// Relaxing a length comparison in the detector turns this case red.
#[test]
fn one_past_each_declared_bound_passes_through_unchanged() {
    let cases = [
        [["AK", "IA"].concat(), upper(15)].concat(),
        [["AK", "IA"].concat(), upper(17)].concat(),
        ["gh", "p_", &body(35)].concat(),
        ["github", "_pat_", &body(21)].concat(),
        ["xo", "xb-", &body(9)].concat(),
        ["Bearer ", &body(15)].concat(),
        jwt_of(9),
    ];
    for credential in cases {
        let observed = format!("value: {credential} end");
        let projection = redact_for_model(&observed);
        assert!(
            projection.redactions.is_empty() && projection.text == observed,
            "{observed:?} was redacted: {:?}",
            projection.redactions
        );
    }
}

/// Green when the contract holds: a text value inside JSON carries a PEM key with escaped line
/// breaks (`\n` as two characters), as a service-account key file does; the block is replaced.
#[test]
fn a_private_key_in_an_escaped_json_string_is_replaced() {
    let escaped = pem("PRIVATE KEY", "\\n");
    let observed = format!("{{\"type\":\"fixture\",\"private_key\":\"{escaped}\\n\"}}");
    let projection = redact_for_model(&observed);
    assert_spans_rebuild(&observed, &projection);
    assert!(
        !projection.text.contains(KEY_BODY),
        "the private key body reached the model-facing text: {:?}",
        projection.text
    );
}

/// Green when the contract holds: an invisible or non-ASCII character directly before or after a
/// credential (zero-width space, no-break space, byte-order mark, full-width colon) is not a word
/// byte, so it neither hides the credential nor splits a span inside a character.
#[test]
fn invisible_and_non_ascii_neighbours_do_not_hide_a_credential() {
    let mut escaped = Vec::new();
    for neighbour in ["\u{200B}", "\u{00A0}", "\u{FEFF}", "\u{FF1A}"] {
        for (shape, credential, secret) in shapes() {
            let observed = format!("key{neighbour}{credential}{neighbour}end");
            let projection = redact_for_model(&observed);
            assert_spans_rebuild(&observed, &projection);
            if projection.text.contains(&secret) {
                escaped.push(format!("{shape} beside {neighbour:?}"));
            }
        }
    }
    assert!(
        escaped.is_empty(),
        "credentials reached the model-facing text: {escaped:?}"
    );
}

/// Invariant 9 names credential-shaped content, and a password in the userinfo of a URL is one: a
/// connection string in a log or a configuration dump. `CredentialShape::UrlPassword` covers it:
/// the password is replaced and the scheme, user and host stay visible.
#[test]
fn a_password_in_url_userinfo_is_replaced() {
    let password = ["fixture", "-pass-", &body(12)].concat();
    let mut escaped = Vec::new();
    for scheme in ["https", "postgres", "redis"] {
        let observed =
            format!("connecting to {scheme}://fixture-user:{password}@store.invalid:1/db\n");
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        if projection.text.contains(&password) {
            escaped.push(scheme);
        }
    }
    assert!(
        escaped.is_empty(),
        "a URL password reached the model-facing text for {escaped:?}"
    );
}

/// The access key id is the public half of an AWS credential; the secret access key is the half
/// that signs. Its usual spellings (an environment assignment, a credentials-file line) carry a
/// forty-character key, which `CredentialShape::AwsSecretAccessKey` covers: the key is replaced and
/// the name stays visible.
#[test]
fn an_aws_secret_access_key_assignment_is_replaced() {
    let secret: String = "Fixture0/Fixture1+".chars().cycle().take(40).collect();
    let mut escaped = Vec::new();
    for name in [
        ["AWS_SECRET", "_ACCESS_KEY="].concat(),
        ["aws_secret", "_access_key = "].concat(),
    ] {
        let observed = format!("{}\n{name}{secret}\n", ["AK", "IA", &upper(16)].concat());
        let projection = redact_for_model(&observed);
        assert_spans_rebuild(&observed, &projection);
        if projection.text.contains(&secret) {
            escaped.push(name);
        }
    }
    assert!(
        escaped.is_empty(),
        "a secret access key reached the model-facing text after {escaped:?}"
    );
}

/// An unsecured JSON Web Token (`alg` `none`) has an empty third segment: `header.payload.`. Its
/// claims are no less readable for being unsigned, so `CredentialShape::JsonWebToken` admits an
/// empty third segment and the whole token is replaced.
#[test]
fn an_unsigned_json_web_token_is_replaced() {
    let lead = ["ey", "J"].concat();
    let claims = [lead.clone(), body(24)].concat();
    let token = [
        lead,
        body(14),
        ".".to_owned(),
        claims.clone(),
        ".".to_owned(),
    ]
    .concat();
    let observed = format!("token={token} next");
    let projection = redact_for_model(&observed);
    assert_spans_rebuild(&observed, &projection);
    assert!(
        !projection.text.contains(&claims),
        "an unsigned token's claims reached the model-facing text: {:?}",
        projection.text
    );
}
