//! Replies to session requests, typed: outcomes, refusals and faults (`story:sdk-session-transport`).
//!
//! A [`Reply`] is what `ekr` answered one request with: the exit status, the JSON document it
//! printed and what it wrote to stderr, exactly as `docs/cli.md` § Exit codes and output defines
//! them. [`Reply::answer`] reads that into an [`Answer`].

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One request's answer: what the one-shot verb would have exited with and printed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    /// The exit status: 0 an outcome, 1 a fault, 2 a named refusal or a usage error.
    pub exit: i32,
    /// The JSON document the verb printed on stdout, or `None` when it printed nothing.
    pub document: Option<Value>,
    /// The text the verb wrote to stderr, newline included; empty when it wrote nothing.
    pub stderr: String,
}

/// A [`Reply`] read by its exit status.
#[derive(Clone, Debug, PartialEq)]
pub enum Answer {
    /// Exit 0: a declared outcome.
    Outcome(Outcome),
    /// Exit 2 with `ekr: <code>: <reason>` on stderr: a named refusal. Nothing was recorded.
    Refusal(Refusal),
    /// Exit 2 with anything else on stderr: clap's usage message, whole.
    Usage(String),
    /// Exit 1, or any status other than 0 and 2: a fault.
    Fault(Fault),
}

/// An exit-0 document, by its `kind` where the kind is one a write verb declares.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// `validate`: the transaction may be committed.
    Validated(Value),
    /// `validate`: the transaction is refused for good; the document lists its `issues`.
    Rejected(Value),
    /// `commit`: a new revision was published, `result.revision`.
    Committed(Value),
    /// `commit`: the head moved after validation and nothing was applied.
    Stale(Value),
    /// Any other document: a read, a proposal record, a seed result, a minted id, a hash.
    Other(Value),
}

/// A named refusal: `ekr: <code>: <reason>` on exit 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refusal {
    /// The refusal's name, such as `ekr.kernel.TransactionNotFound` or `session-request-malformed`.
    pub code: String,
    /// The text after the code, without its trailing newline.
    pub reason: String,
}

/// A fault: the verb could not answer (a provider, verification, input or configuration failure).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// The exit status, 1 for every fault `ekr` declares.
    pub exit: i32,
    /// What the verb wrote to stderr, without its trailing newline.
    pub message: String,
}

impl Reply {
    /// This reply, typed by its exit status and, on exit 0, by the document's `kind`.
    ///
    /// An exit 0 that printed nothing is an [`Outcome::Other`] holding JSON `null`.
    pub fn answer(&self) -> Answer {
        let stderr = self.stderr.strip_suffix('\n').unwrap_or(&self.stderr);
        match self.exit {
            0 => {
                let document = self.document.clone().unwrap_or(Value::Null);
                Answer::Outcome(match document.get("kind").and_then(Value::as_str) {
                    Some("Validated") => Outcome::Validated(document),
                    Some("Rejected") => Outcome::Rejected(document),
                    Some("Committed") => Outcome::Committed(document),
                    Some("Stale") => Outcome::Stale(document),
                    _ => Outcome::Other(document),
                })
            }
            2 => match refusal(stderr) {
                Some(refusal) => Answer::Refusal(refusal),
                None => Answer::Usage(self.stderr.clone()),
            },
            exit => Answer::Fault(Fault {
                exit,
                message: stderr.to_owned(),
            }),
        }
    }
}

impl Outcome {
    /// The document this outcome was read from.
    pub fn document(&self) -> &Value {
        match self {
            Outcome::Validated(document)
            | Outcome::Rejected(document)
            | Outcome::Committed(document)
            | Outcome::Stale(document)
            | Outcome::Other(document) => document,
        }
    }
}

/// `ekr: <code>: <reason>`, where the code is one word.
fn refusal(stderr: &str) -> Option<Refusal> {
    let (code, reason) = stderr.strip_prefix("ekr: ")?.split_once(": ")?;
    let named = !code.is_empty() && !code.contains(char::is_whitespace);
    named.then(|| Refusal {
        code: code.to_owned(),
        reason: reason.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(exit: i32, stderr: &str) -> Reply {
        Reply {
            exit,
            document: None,
            stderr: stderr.to_owned(),
        }
    }

    #[test]
    fn a_refusal_is_one_word_after_the_prefix() {
        assert_eq!(
            reply(2, "ekr: session-request-malformed: not a request: x\n").answer(),
            Answer::Refusal(Refusal {
                code: "session-request-malformed".into(),
                reason: "not a request: x".into(),
            })
        );
        let usage = "error: invalid value 'zero' for '--at <AT>'\n";
        assert_eq!(reply(2, usage).answer(), Answer::Usage(usage.into()));
        assert_eq!(
            reply(2, "ekr: opening the provider: invalid\n").answer(),
            Answer::Usage("ekr: opening the provider: invalid\n".into())
        );
    }

    #[test]
    fn every_status_but_zero_and_two_is_a_fault() {
        for exit in [1, 3, -1] {
            assert!(
                matches!(reply(exit, "ekr: x\n").answer(), Answer::Fault(Fault { exit: e, .. }) if e == exit)
            );
        }
        assert_eq!(
            reply(0, "").answer(),
            Answer::Outcome(Outcome::Other(Value::Null))
        );
    }
}
