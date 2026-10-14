//! Credential redaction before model input (predecessors A6, invariant 9).
//!
//! [`redact`] is the model-facing projection of an observed text: `ekr.observe.ModelInput` of
//! `systems/ekr/domains/observe.yaml`. Every credential of a [`CredentialShape`] the text holds is
//! replaced by a marker naming its shape, and each replacement is recorded as a [`Redaction`]: its
//! shape and its [`RedactionSpan`] in the text as observed, never its value.
//!
//! It is a pure function over the text it is given. It borrows that text and returns a new one, so
//! the observation and the bytes it was observed from are left exactly as they were; a text with
//! no credential projects to itself, byte for byte, with no redactions. Which fields of which
//! source reach a model is instance policy and is not decided here, and nothing here names the
//! pipeline step, adapter or checkpoint a projection runs at: that waits on
//! `decision-blocker:observation-retention-path`.
//!
//! The types are `ekr.observe`'s, and the observation layer is where they are used: `ekr-observe`
//! re-exports this module as `ekr_observe::redact`. The code sits here, needing only `std`, because
//! `ekr-sdk` links no workspace crate but this one, and a consumer redacts in process before it
//! hands a text to a model. A shape is a credential format, not a domain or graph concept
//! (invariant 8).

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
    /// (an encoded `{"`), each at least ten characters.
    JsonWebToken,
    /// The credential of an HTTP bearer authorisation: the at least sixteen token characters after
    /// `Bearer ` (or `bearer `). The scheme word stays visible; only the token is replaced.
    BearerToken,
    /// A PEM private key, from its `-----BEGIN … PRIVATE KEY-----` line through its matching end
    /// line, or to the end of the text when the end line is missing.
    PrivateKeyBlock,
}

impl CredentialShape {
    /// Every shape, in the order `observe.yaml` declares them.
    pub const ALL: [Self; 6] = [
        Self::AwsAccessKeyId,
        Self::GitHubToken,
        Self::SlackToken,
        Self::JsonWebToken,
        Self::BearerToken,
        Self::PrivateKeyBlock,
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
/// its first byte and its length in bytes. Only [`redact`] makes one, so its length is never zero.
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
pub fn redact(observed: &str) -> ModelInput {
    let bytes = observed.as_bytes();
    let mut text = String::with_capacity(observed.len());
    let mut redactions = Vec::new();
    let mut kept = 0;
    let mut at = 0;
    while at < bytes.len() {
        match detect(bytes, at) {
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

/// The credential that begins at `at`, if one does. A detector anchors only where the byte before
/// is not part of a word, so a shape's prefix inside a longer word is not a credential.
fn detect(bytes: &[u8], at: usize) -> Option<(CredentialShape, RedactionSpan)> {
    if at > 0 && is_word(bytes[at - 1]) {
        return None;
    }
    let rest = &bytes[at..];
    let found = aws_access_key_id(rest)
        .map(|length| (CredentialShape::AwsAccessKeyId, 0, length))
        .or_else(|| github_token(rest).map(|length| (CredentialShape::GitHubToken, 0, length)))
        .or_else(|| slack_token(rest).map(|length| (CredentialShape::SlackToken, 0, length)))
        .or_else(|| json_web_token(rest).map(|length| (CredentialShape::JsonWebToken, 0, length)))
        .or_else(|| {
            bearer_token(rest)
                .map(|(offset, length)| (CredentialShape::BearerToken, offset, length))
        })
        .or_else(|| {
            private_key_block(rest).map(|length| (CredentialShape::PrivateKeyBlock, 0, length))
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

const fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

const fn is_base64url(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'
}

/// The length of the run at the start of `bytes` whose every byte satisfies `admits`.
fn run(bytes: &[u8], admits: impl Fn(u8) -> bool) -> usize {
    bytes.iter().take_while(|byte| admits(**byte)).count()
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

fn slack_token(bytes: &[u8]) -> Option<usize> {
    let body = bytes.strip_prefix(b"xox")?;
    let (kind, body) = body.split_first()?;
    let body = body.strip_prefix(b"-")?;
    if !b"abprs".contains(kind) {
        return None;
    }
    let length = run(body, |byte| byte.is_ascii_alphanumeric() || byte == b'-');
    (length >= 10 && ends_word(body, length)).then_some(5 + length)
}

fn json_web_token(bytes: &[u8]) -> Option<usize> {
    let mut length = 0;
    for segment in 0..3 {
        if segment > 0 {
            if bytes.get(length) != Some(&b'.') {
                return None;
            }
            length += 1;
        }
        let rest = &bytes[length..];
        if segment < 2 && !rest.starts_with(b"eyJ") {
            return None;
        }
        let part = run(rest, is_base64url);
        if part < 10 {
            return None;
        }
        length += part;
    }
    ends_word(bytes, length).then_some(length)
}

/// The token after `Bearer ` or `bearer `, as its offset from the scheme word and its length.
fn bearer_token(bytes: &[u8]) -> Option<(usize, usize)> {
    let body = [b"Bearer ".as_slice(), b"bearer "]
        .iter()
        .find_map(|prefix| bytes.strip_prefix(*prefix))?;
    let length = run(body, |byte| {
        byte.is_ascii_alphanumeric() || b"-._~+/=".contains(&byte)
    });
    (length >= 16).then_some((7, length))
}

/// From a `-----BEGIN … PRIVATE KEY-----` line through the next `-----END … PRIVATE KEY-----`
/// line, or to the end of the text when there is none.
fn private_key_block(bytes: &[u8]) -> Option<usize> {
    let header = armour_line(bytes, b"-----BEGIN ")?;
    let mut at = header;
    while at < bytes.len() {
        if let Some(footer) = armour_line(&bytes[at..], b"-----END ") {
            return Some(at + footer);
        }
        at += 1;
    }
    Some(bytes.len())
}

/// The length of a PEM armour line opening `bytes` that names a private key: `opening`, any
/// upper-case words, then `PRIVATE KEY-----`.
fn armour_line(bytes: &[u8], opening: &[u8]) -> Option<usize> {
    let body = bytes.strip_prefix(opening)?;
    let label = run(body, |byte| byte.is_ascii_uppercase() || byte == b' ');
    (body[..label].ends_with(b"PRIVATE KEY") && body[label..].starts_with(b"-----"))
        .then_some(opening.len() + label + 5)
}
