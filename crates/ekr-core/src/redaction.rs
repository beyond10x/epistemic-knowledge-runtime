//! Credential redaction before model input (predecessors A6, invariant 9).
//!
//! [`redact_for_model`] is the model-facing projection of an observed text:
//! `ekr.observe.ModelInput` of `systems/ekr/domains/observe.yaml`. Every credential of a
//! [`CredentialShape`] the text holds is replaced by a marker naming its shape, and each
//! replacement is recorded as a [`Redaction`]: its shape and its [`RedactionSpan`] in the text as
//! observed, never its value.
//!
//! It is a pure function over the text it is given. It borrows that text and returns a new one, so
//! the observation and the bytes it was observed from are left exactly as they were; a text with
//! no credential projects to itself, byte for byte, with no redactions. Which fields of which
//! source reach a model is instance policy and is not decided here, and nothing here names the
//! pipeline step, adapter or checkpoint a projection runs at: that waits on
//! `decision-blocker:observation-retention-path`.
//!
//! Where a credential is looked for, as `observe.yaml` states it: a self-delimiting shape where
//! the byte before it does not continue a word, or where it follows an escape the text carries as
//! characters (`\n`, `\t`, `\r`, `\b`, `\f`, `\uXXXX`, or a percent-encoded byte such as `%3D`),
//! since observed text is often a serialised record or a URL; a private key block and a URL
//! password wherever they stand. Detection examines each byte of the text a bounded number of
//! times, so its work grows with the text's length and never with its square.
//!
//! The types are `ekr.observe`'s: `ekr-observe` re-exports this module as
//! `ekr_observe::redaction`.
//! The code sits here, needing only `std`, because `ekr-sdk` links no workspace crate but this
//! one, and a consumer redacts in process before it hands a text to a model. A shape is a
//! credential format, not a domain or graph concept (invariant 8).

use std::fmt;

/// A credential shape a model-facing projection detects and replaces:
/// `ekr.observe.CredentialShape`. The shapes are generic; a source's own secrets are instance
/// policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CredentialShape {
    /// An AWS access key id: `AKIA` or `ASIA` and sixteen upper-case letters or digits.
    AwsAccessKeyId,
    /// A GitHub token: `ghp_`, `gho_`, `ghu_`, `ghs_` or `ghr_` and at least 36 letters or digits,
    /// or `github_pat_` and at least 22 letters, digits or underscores.
    GitHubToken,
    /// A Slack token: `xoxa-`, `xoxb-`, `xoxp-`, `xoxr-` or `xoxs-` and at least ten letters,
    /// digits or hyphens.
    SlackToken,
    /// A JSON Web Token: three base64url segments joined by `.`, the first two opening with `eyJ`
    /// (an encoded `{"`) and at least ten characters each; the third is empty for an unsecured
    /// token, whose claims are no less readable for being unsigned.
    JsonWebToken,
    /// The credential of an HTTP bearer authorisation: the at least sixteen token characters after
    /// the scheme word `bearer`, in any letter case, and one or more spaces, tabs or
    /// percent-encoded spaces (`%20`, as in a URL query); RFC 9110 § 11.1, § 11.4. The scheme word
    /// stays visible; only the token is replaced.
    BearerToken,
    /// An armoured private key, PEM (a `BEGIN … PRIVATE KEY` line) or OpenPGP (a
    /// `BEGIN PGP PRIVATE KEY BLOCK` line), each between runs of five hyphens, from its begin line
    /// through its matching end line, or to the end of the text when the end line is missing.
    PrivateKeyBlock,
    /// An AWS secret access key: the forty letters, digits, `/` or `+` assigned to the name
    /// `aws_secret_access_key`, in any letter case, by `=` or `:`, with any spaces, tabs or quote
    /// around the sign. The name stays visible; only the key is replaced.
    AwsSecretAccessKey,
    /// The password of a URL's userinfo, `scheme://user:password@`: the at least one byte between
    /// the first `:` after `://` and the `@`. The scheme, user and host stay visible.
    UrlPassword,
}

impl CredentialShape {
    /// Every shape, in the order `observe.yaml` declares them.
    pub const ALL: [Self; 8] = [
        Self::AwsAccessKeyId,
        Self::GitHubToken,
        Self::SlackToken,
        Self::JsonWebToken,
        Self::BearerToken,
        Self::PrivateKeyBlock,
        Self::AwsSecretAccessKey,
        Self::UrlPassword,
    ];

    /// The shape's name as `observe.yaml` declares it, which is also what its marker names.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AwsAccessKeyId => "AwsAccessKeyId",
            Self::GitHubToken => "GitHubToken",
            Self::SlackToken => "SlackToken",
            Self::JsonWebToken => "JsonWebToken",
            Self::BearerToken => "BearerToken",
            Self::PrivateKeyBlock => "PrivateKeyBlock",
            Self::AwsSecretAccessKey => "AwsSecretAccessKey",
            Self::UrlPassword => "UrlPassword",
        }
    }

    /// The text that stands in the model-facing projection where a credential of this shape was:
    /// `[REDACTED:<name>]`.
    #[must_use]
    pub fn marker(self) -> String {
        format!("[REDACTED:{}]", self.name())
    }
}

impl fmt::Display for CredentialShape {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where a credential sat in the observed text: `ekr.observe.RedactionSpan`, the byte offset of
/// its first byte and its length in bytes. Only [`redact_for_model`] makes one, so its length is
/// never zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RedactionSpan {
    start: usize,
    length: usize,
}

impl RedactionSpan {
    /// The byte offset, in the text as observed, of the credential's first byte.
    #[must_use]
    pub const fn start(self) -> usize {
        self.start
    }

    /// The credential's length in bytes; at least one.
    #[must_use]
    pub const fn length(self) -> usize {
        self.length
    }

    /// The byte offset just past the credential's last byte.
    #[must_use]
    pub const fn end(self) -> usize {
        self.start + self.length
    }

    /// The span as a range of byte offsets into the text as observed.
    #[must_use]
    pub const fn range(self) -> std::ops::Range<usize> {
        self.start..self.end()
    }
}

/// One credential replaced: `ekr.observe.Redaction`. Its shape and its span, and nothing else; the
/// credential's value is never recorded, hashed or echoed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Redaction {
    /// The credential's shape.
    pub shape: CredentialShape,
    /// Where it sat in the text as observed.
    pub span: RedactionSpan,
}

/// The model-facing projection of an observed text: `ekr.observe.ModelInput`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModelInput {
    /// The observed text with every detected credential replaced by its shape's
    /// [marker](CredentialShape::marker).
    pub text: String,
    /// One redaction per replacement, in span order; spans never overlap.
    pub redactions: Vec<Redaction>,
}

/// The model-facing projection of `observed`.
///
/// Left to right, each credential of a [`CredentialShape`] is replaced by its shape's marker and
/// recorded as a [`Redaction`]. A text with no credential comes back unchanged, byte for byte,
/// with no redactions. `observed` is borrowed and never changed.
#[must_use]
pub fn redact_for_model(observed: &str) -> ModelInput {
    let bytes = observed.as_bytes();
    let mut text = String::with_capacity(observed.len());
    let mut redactions = Vec::new();
    let mut quiet = Quiet::default();
    let mut kept = 0;
    let mut at = 0;
    while at < bytes.len() {
        examined(1);
        match detect(bytes, at, &mut quiet) {
            Some((shape, span)) => {
                // Every detector starts and ends on an ASCII byte, so both are char boundaries.
                text.push_str(&observed[kept..span.start]);
                text.push_str(&shape.marker());
                redactions.push(Redaction { shape, span });
                kept = span.end();
                at = kept;
            }
            None => at += 1,
        }
    }
    text.push_str(&observed[kept..]);
    ModelInput { text, redactions }
}

/// Positions before which a shape is known not to start, learnt from an attempt that failed.
///
/// A Slack token's body and a JSON Web Token's first segment are runs that admit a non-word byte
/// (`-`), so a fresh start can sit inside a run an earlier attempt already scanned. Every such
/// later start sees the same run end and the same bytes after it, with less before it, so it
/// fails wherever the earlier attempt failed. Skipping those starts is what keeps the work
/// linear: without it, `xoxb-` repeated and followed by `_` is rescanned from every repetition.
#[derive(Default)]
struct Quiet {
    slack: usize,
    json_web_token: usize,
}

/// The credential that begins at `at`, if one does.
fn detect(bytes: &[u8], at: usize, quiet: &mut Quiet) -> Option<(CredentialShape, RedactionSpan)> {
    let rest = &bytes[at..];
    let mut found = None;
    if starts_word(bytes, at) {
        found = aws_access_key_id(rest)
            .map(|length| (CredentialShape::AwsAccessKeyId, 0, length))
            .or_else(|| github_token(rest).map(|length| (CredentialShape::GitHubToken, 0, length)))
            .or_else(|| {
                if at < quiet.slack {
                    return None;
                }
                match slack_token(rest) {
                    Ok(length) => Some((CredentialShape::SlackToken, 0, length)),
                    Err(clear) => {
                        quiet.slack = at + clear;
                        None
                    }
                }
            })
            .or_else(|| {
                if at < quiet.json_web_token {
                    return None;
                }
                match json_web_token(rest) {
                    Ok(length) => Some((CredentialShape::JsonWebToken, 0, length)),
                    Err(clear) => {
                        quiet.json_web_token = at + clear;
                        None
                    }
                }
            })
            .or_else(|| {
                bearer_token(rest)
                    .map(|(offset, length)| (CredentialShape::BearerToken, offset, length))
            });
    }
    // An assignment's name is often prefixed (`TF_VAR_aws_secret_access_key`), so an underscore
    // before it does not hide it.
    if found.is_none() && (starts_word(bytes, at) || !bytes[at - 1].is_ascii_alphanumeric()) {
        found = aws_secret_access_key(rest)
            .map(|(offset, length)| (CredentialShape::AwsSecretAccessKey, offset, length));
    }
    let found = found
        .or_else(|| {
            private_key_block(rest).map(|length| (CredentialShape::PrivateKeyBlock, 0, length))
        })
        .or_else(|| {
            url_password(bytes, at)
                .map(|(offset, length)| (CredentialShape::UrlPassword, offset, length))
        });
    found.map(|(shape, offset, length)| {
        (
            shape,
            RedactionSpan {
                start: at + offset,
                length,
            },
        )
    })
}

/// `true` when a self-delimiting shape may begin at `at`: the start of the text, a byte before it
/// that does not continue a word, or an escape before it that the text carries as characters.
/// So a shape's prefix inside a longer word is not a credential, and `\nAKIA…` in a serialised
/// record or `%3DAKIA…` in a URL is.
fn starts_word(bytes: &[u8], at: usize) -> bool {
    at == 0 || !is_word(bytes[at - 1]) || follows_escape(&bytes[..at])
}

/// `true` when `before` ends with a character escape: a backslash and one of `b f n r t`, a
/// backslash, `u` and four hexadecimal digits, or a percent-encoded byte.
fn follows_escape(before: &[u8]) -> bool {
    match before {
        [.., b'\\', b'u', a, b, c, d] if [a, b, c, d].iter().all(|x| x.is_ascii_hexdigit()) => true,
        [.., b'\\', letter] if b"bfnrt".contains(letter) => true,
        [.., b'%', high, low] => high.is_ascii_hexdigit() && low.is_ascii_hexdigit(),
        _ => false,
    }
}

const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

const fn is_base64url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// The length of the run at the start of `bytes` whose every byte satisfies `admits`.
fn run(bytes: &[u8], admits: impl Fn(u8) -> bool) -> usize {
    let length = bytes.iter().take_while(|byte| admits(**byte)).count();
    examined(length + 1);
    length
}

/// The armour line that opens a key block. A token whose run admits `-` stops before it, so a
/// private key written directly after a token is still found and replaced.
const ARMOUR_BEGIN: &[u8] = b"-----BEGIN ";

/// [`run`] for a token, stopping before an [`ARMOUR_BEGIN`].
fn token_run(bytes: &[u8], admits: impl Fn(u8) -> bool) -> usize {
    let mut length = 0;
    while let Some(&byte) = bytes.get(length) {
        if !admits(byte) || (byte == b'-' && bytes[length..].starts_with(ARMOUR_BEGIN)) {
            break;
        }
        length += 1;
    }
    examined(length + 1);
    length
}

/// `true` when the byte at `index` does not continue a word: the end of the text or a non-word
/// byte.
fn ends_word(bytes: &[u8], index: usize) -> bool {
    bytes.get(index).is_none_or(|byte| !is_word(*byte))
}

fn aws_access_key_id(bytes: &[u8]) -> Option<usize> {
    let body = [b"AKIA".as_slice(), b"ASIA"]
        .iter()
        .find_map(|prefix| bytes.strip_prefix(*prefix))?;
    let length = run(body, |byte| {
        byte.is_ascii_uppercase() || byte.is_ascii_digit()
    });
    (length == 16 && ends_word(body, length)).then_some(4 + length)
}

fn github_token(bytes: &[u8]) -> Option<usize> {
    for prefix in [b"ghp_".as_slice(), b"gho_", b"ghu_", b"ghs_", b"ghr_"] {
        if let Some(body) = bytes.strip_prefix(prefix) {
            let length = run(body, |byte| byte.is_ascii_alphanumeric());
            return (length >= 36 && ends_word(body, length)).then_some(prefix.len() + length);
        }
    }
    let prefix = b"github_pat_";
    let body = bytes.strip_prefix(prefix.as_slice())?;
    let length = run(body, is_word);
    (length >= 22).then_some(prefix.len() + length)
}

/// A Slack token's length, or, when there is none, how many bytes from here no Slack token can
/// start in (see [`Quiet`]).
fn slack_token(bytes: &[u8]) -> Result<usize, usize> {
    let Some((kind, body)) = bytes.strip_prefix(b"xox").and_then(<[u8]>::split_first) else {
        return Err(1);
    };
    let Some(body) = body.strip_prefix(b"-") else {
        return Err(1);
    };
    if !b"abprs".contains(kind) {
        return Err(1);
    }
    let length = token_run(body, |byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if length >= 10 && ends_word(body, length) {
        Ok(5 + length)
    } else {
        // A later start whose five-byte prefix lies inside this run has a shorter body ending at
        // the same byte, so it fails too.
        Err(length + 1)
    }
}

/// A JSON Web Token's length, or, when there is none, how many bytes from here no token can
/// start in (see [`Quiet`]).
fn json_web_token(bytes: &[u8]) -> Result<usize, usize> {
    if !bytes.starts_with(b"eyJ") {
        return Err(1);
    }
    let first = token_run(bytes, is_base64url);
    // A later start inside the first segment ends that segment at the same byte and sees the
    // same bytes after it, with a shorter first segment: it fails wherever this one does.
    let miss = Err(first);
    if first < 10 {
        return miss;
    }
    let mut length = first;
    for segment in 1..3 {
        if bytes.get(length) != Some(&b'.') {
            return miss;
        }
        length += 1;
        let rest = &bytes[length..];
        if segment == 1 && !rest.starts_with(b"eyJ") {
            return miss;
        }
        let part = token_run(rest, is_base64url);
        if segment == 1 && part < 10 {
            return miss;
        }
        length += part;
    }
    if ends_word(bytes, length) {
        Ok(length)
    } else {
        miss
    }
}

/// The token after the scheme word `bearer` (any letter case) and one or more spaces, tabs or
/// `%20`, as its offset from the scheme word and its length.
fn bearer_token(bytes: &[u8]) -> Option<(usize, usize)> {
    let scheme = b"bearer";
    if !bytes.get(..scheme.len())?.eq_ignore_ascii_case(scheme) {
        return None;
    }
    let mut offset = scheme.len();
    loop {
        let rest = &bytes[offset..];
        examined(1);
        if rest.starts_with(b" ") || rest.starts_with(b"\t") {
            offset += 1;
        } else if rest.starts_with(b"%20") {
            offset += 3;
        } else {
            break;
        }
    }
    if offset == scheme.len() {
        return None;
    }
    let length = token_run(&bytes[offset..], |byte| {
        byte.is_ascii_alphanumeric() || b"-._~+/=".contains(&byte)
    });
    (length >= 16).then_some((offset, length))
}

/// The forty-character key assigned to `aws_secret_access_key` (any letter case) by `=` or `:`,
/// as its offset from the name and its length.
fn aws_secret_access_key(bytes: &[u8]) -> Option<(usize, usize)> {
    let name = b"aws_secret_access_key";
    if !bytes.get(..name.len())?.eq_ignore_ascii_case(name) {
        return None;
    }
    let mut at = name.len();
    let quote = |byte: u8| byte == b'"' || byte == b'\'';
    let blank = |byte: u8| byte == b' ' || byte == b'\t';
    at += run(&bytes[at..], quote).min(1);
    at += run(&bytes[at..], blank);
    if !matches!(bytes.get(at), Some(b'=' | b':')) {
        return None;
    }
    at += 1;
    at += run(&bytes[at..], blank);
    at += run(&bytes[at..], quote).min(1);
    let is_key = |byte: u8| byte.is_ascii_alphanumeric() || byte == b'/' || byte == b'+';
    let key = &bytes[at..];
    let length = key
        .iter()
        .take(41)
        .take_while(|byte| is_key(**byte))
        .count();
    examined(length + 1);
    (length == 40 && key.get(40).is_none_or(|byte| *byte != b'=')).then_some((at, 40))
}

/// A byte of a URL's userinfo other than the `:` that separates user from password.
const fn is_userinfo(byte: u8) -> bool {
    match byte {
        b':' | b'@' | b'/' | b'?' | b'#' | b'"' | b'\'' | b'<' | b'>' | b'\\' | b'`' => false,
        _ => byte >= 0x80 || byte.is_ascii_graphic(),
    }
}

/// At a `:` that is the first after a URL's `://`, the password that runs from the next byte to
/// the `@` ending the userinfo, as its offset from `at` and its length.
fn url_password(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    if bytes[at] != b':' {
        return None;
    }
    // Back over the user; it stops at the previous `:`, `/` or `@`, so no byte is walked twice.
    let user = bytes[..at]
        .iter()
        .rev()
        .take_while(|byte| is_userinfo(**byte))
        .count();
    examined(user + 1);
    let user_start = at - user;
    let scheme_end = user_start.checked_sub(3)?;
    if &bytes[scheme_end..user_start] != b"://"
        || !scheme_end
            .checked_sub(1)
            .is_some_and(|last| bytes[last].is_ascii_alphanumeric())
    {
        return None;
    }
    let length = run(&bytes[at + 1..], |byte| byte == b':' || is_userinfo(byte));
    (length >= 1 && bytes.get(at + 1 + length) == Some(&b'@')).then_some((1, length))
}

/// From a `-----BEGIN … PRIVATE KEY-----` line through the next `-----END … PRIVATE KEY-----`
/// line, or to the end of the text when there is none.
fn private_key_block(bytes: &[u8]) -> Option<usize> {
    let header = armour_line(bytes, b"-----BEGIN ")?;
    let mut at = header;
    while at < bytes.len() {
        examined(1);
        if let Some(footer) = armour_line(&bytes[at..], b"-----END ") {
            return Some(at + footer);
        }
        at += 1;
    }
    Some(bytes.len())
}

/// The length of an armour line opening `bytes` that names a private key: `opening`, any
/// upper-case words, then `PRIVATE KEY-----` (PEM) or `PRIVATE KEY BLOCK-----` (OpenPGP).
fn armour_line(bytes: &[u8], opening: &[u8]) -> Option<usize> {
    let body = bytes.strip_prefix(opening)?;
    let label = run(body, |byte| byte.is_ascii_uppercase() || byte == b' ');
    let names_private_key = [b"PRIVATE KEY".as_slice(), b"PRIVATE KEY BLOCK"]
        .iter()
        .any(|ending| body[..label].ends_with(ending));
    (names_private_key && body[label..].starts_with(b"-----")).then_some(opening.len() + label + 5)
}

#[cfg(test)]
thread_local! {
    /// Bytes the detectors have examined on this thread: the work a test counts.
    static EXAMINED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Counts `bytes` examined bytes; does nothing outside this module's tests.
#[cfg_attr(not(test), allow(unused_variables))]
#[inline]
fn examined(bytes: usize) {
    #[cfg(test)]
    EXAMINED.with(|count| count.set(count.get() + bytes));
}

#[cfg(test)]
mod tests {
    use super::{redact_for_model, EXAMINED};

    /// The bytes the detectors examine to project `text`, counted, never timed.
    fn work(text: &str) -> usize {
        EXAMINED.with(|count| count.set(0));
        let _ = redact_for_model(text);
        EXAMINED.with(std::cell::Cell::get)
    }

    /// Each unit repeats a shape's opening so that every repetition is a fresh place a detector
    /// may start, and the run it scans from there reaches the end of the text. Projection must
    /// examine each byte a bounded number of times: the count stays within a fixed multiple of the
    /// text's length at every size, so doubling the text cannot quadruple the work.
    #[test]
    fn detection_examines_each_byte_a_bounded_number_of_times() {
        let units: [(&str, String, &str); 13] = [
            (
                "slack opening before an underscore",
                ["xo", "xb-"].concat(),
                "_",
            ),
            ("token segment openings", ["-e", "yJ"].concat(), ""),
            (
                "token segments without a third",
                ["ey", "J0123456789."].concat(),
                "_",
            ),
            ("bearer schemes", "bearer ".to_owned(), ""),
            ("bearer schemes with dashes", "Bearer -".to_owned(), ""),
            ("encoded bearer separators", "bearer%20%20".to_owned(), ""),
            (
                "secret key names",
                ["aws_secret", "_access_key="].concat(),
                "",
            ),
            ("URL schemes", "a://b:".to_owned(), ""),
            ("colons", "x:".to_owned(), ""),
            ("armour openings", ["-----", "BEGIN "].concat(), ""),
            (
                "public key armour",
                ["-----", "BEGIN PUBLIC KEY-----"].concat(),
                "",
            ),
            ("escapes", "\\n%3D".to_owned(), ""),
            ("one letter", "a".to_owned(), ""),
        ];
        let mut over = Vec::new();
        for (name, unit, tail) in &units {
            for bytes in [10_000, 20_000] {
                let text = format!("{}{tail}", unit.repeat(bytes / unit.len()));
                let examined = work(&text);
                if examined > 8 * text.len() {
                    over.push(format!(
                        "{name}: {examined} examined for {} bytes",
                        text.len()
                    ));
                }
            }
        }
        assert!(over.is_empty(), "detection rescans the text: {over:?}");
    }
}
