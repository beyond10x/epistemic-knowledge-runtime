//! Adversary pass on `task:protocol-error-keeps-the-answer`: the `answer` a
//! `TransportError::Protocol` keeps, driven against the field documentation in `transport.rs`
//! and the `Protocol` row of `docs/sdk.md` ("Failure, the latch and cancellation").
//!
//! Every child is a stand-in shell script that answers the binary handshake (`--version`). Where
//! the exact bytes matter it copies `answer.bin`, which the test wrote beside it.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{OneShotReader, ReadError};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport as _, TransportError, ANSWER_BYTES, ANSWER_CUT};

const HANDSHAKE: &str = "#!/bin/sh\ncase \"$1\" in --version) echo 'ekr 0.0.20'; exit 0 ;; esac\n";

/// A one-shot stand-in: reads its input to the end, prints `answer.bin` beside it, exits 0.
const ONE_SHOT_PRINTS_THE_FILE: &str = "cat >/dev/null\ncat \"$(dirname \"$0\")/answer.bin\"\n";

/// A session stand-in: reads one request line, answers with `answer.bin` beside it.
const SESSION_ANSWERS_THE_FILE: &str =
    "read line\ncat \"$(dirname \"$0\")/answer.bin\"\nexec cat >/dev/null\n";

fn stand_in(directory: &Path, body: &str) -> EkrBinary {
    let path = directory.join("ekr");
    std::fs::write(&path, format!("{HANDSHAKE}{body}")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    EkrBinary::open(&path).expect("the stand-in answers the handshake")
}

fn store(directory: &Path) -> StoreConfig {
    StoreConfig {
        host: directory.join("host.json"),
        store: directory.join("store"),
        backend: Backend::File,
    }
}

/// The `answer` and message of a `Protocol` error, or a panic naming what came instead.
fn protocol(error: &TransportError) -> (String, String) {
    match error {
        TransportError::Protocol { answer, .. } => (answer.clone(), error.to_string()),
        other => panic!("expected Protocol, got {other:?}"),
    }
}

/// What a one-shot read keeps of `printed`.
fn one_shot_answer(printed: &[u8]) -> (String, String) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("answer.bin"), printed).unwrap();
    let binary = stand_in(directory.path(), ONE_SHOT_PRINTS_THE_FILE);
    let mut reader =
        OneShotReader::new(&binary, store(directory.path()), SessionOptions::default());
    match reader.head() {
        Err(ReadError::Transport(error)) => protocol(&error),
        other => panic!("expected a transport error, got {other:?}"),
    }
}

/// What a session keeps of the answer line `printed` (newline included by the caller).
fn session_answer(printed: &[u8]) -> (String, String) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("answer.bin"), printed).unwrap();
    let binary = stand_in(directory.path(), SESSION_ANSWERS_THE_FILE);
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let error = session
        .request(&Request::new(["ontology"]))
        .expect_err("the stand-in's line is not a reply");
    protocol(&error)
}

/// Boundary sweep: a character of every UTF-8 width (1 to 4 bytes) placed at every offset
/// around the cut. The kept prefix is the longest one that ends on a character boundary and
/// leaves room for the marker, and the whole is at most `ANSWER_BYTES`.
#[test]
fn every_char_width_at_every_offset_around_the_cut_is_cut_on_a_boundary() {
    let room = ANSWER_BYTES - ANSWER_CUT.len();
    for character in ['a', 'é', '€', '𝄞'] {
        let width = character.len_utf8();
        for back in 0..=width {
            let head = "x".repeat(room - back);
            let long: String = format!("{head}{}", character.to_string().repeat(ANSWER_BYTES));
            let mut expected_end = room;
            while !long.is_char_boundary(expected_end) {
                expected_end -= 1;
            }
            let expected = format!("{}{ANSWER_CUT}", &long[..expected_end]);
            let (answer, message) = one_shot_answer(format!("{long}\n").as_bytes());
            assert!(
                answer.len() <= ANSWER_BYTES,
                "width {width}, back {back}: {} bytes",
                answer.len()
            );
            assert_eq!(answer, expected, "width {width}, back {back}");
            assert!(message.contains(&answer), "width {width}, back {back}");
        }
    }
}

/// An answer that is only a newline: nothing was printed but the line end, so the answer is
/// empty and the message still says so.
#[test]
fn a_session_answer_that_is_only_a_newline_keeps_an_empty_answer() {
    let (answer, message) = session_answer(b"\n");
    assert_eq!(answer, "");
    assert!(message.contains(": \"\"; stderr tail"), "{message}");
}

/// CRLF: the session drops `\n`, the answer drops the `\r` before it; the middle of a one-shot's
/// output keeps its line ends.
#[test]
fn crlf_line_ends_are_dropped_only_at_the_end() {
    let (answer, _) = session_answer(b"not json\r\n");
    assert_eq!(answer, "not json");
    let (answer, _) = one_shot_answer(b"not\r\njson\r\n");
    assert_eq!(answer, "not\r\njson");
}

/// A NUL byte is kept in `answer` and escaped, not dropped, in the message.
#[test]
fn a_nul_byte_is_kept_in_the_answer() {
    let (answer, message) = session_answer(b"a\0b\n");
    assert_eq!(answer, "a\0b");
    assert!(message.contains(r#""a\0b""#), "{message}");
}

/// `transport.rs` documents the answer as "at most ANSWER_BYTES, cut back to a character
/// boundary and ended with ANSWER_CUT when it was longer"; `docs/sdk.md` the same. A one-shot
/// that printed 200 bytes, none of them UTF-8, did not print more than 400 bytes. The lossy
/// decode turns each byte into a three-byte U+FFFD before the length is measured, so the answer
/// is cut and marked `[cut]` though nothing the process printed was over the bound.
#[test]
#[ignore = "finding: an answer of 134..=400 bytes that is not UTF-8 is marked [cut] although it was not longer than ANSWER_BYTES"]
fn a_short_answer_that_is_not_utf8_is_not_marked_cut() {
    let printed = [0xFF_u8; 200];
    let (answer, _) = one_shot_answer(&printed);
    assert!(
        !answer.ends_with(ANSWER_CUT),
        "200 printed bytes were not longer than {ANSWER_BYTES}, yet the answer reads as cut: \
         {} bytes, {} U+FFFD",
        answer.len(),
        answer.chars().filter(|c| *c == '\u{FFFD}').count()
    );
}

/// `transport.rs`: "the start of what the process printed, without its final line end". A
/// one-shot whose output ends in a blank line has one final line end; every trailing CR and LF
/// is dropped instead.
#[test]
#[ignore = "finding: every trailing CR/LF is dropped, not only the final line end the field documentation names"]
fn only_the_final_line_end_is_dropped() {
    let (answer, _) = one_shot_answer(b"not json\n\n");
    assert_eq!(answer, "not json\n");
}

/// The message shows the answer through `{answer:?}`, so an answer with a double quote or a
/// backslash does not appear in the message as printed; a consumer that matches its own copy of
/// the line against the message (the acceptance's "whose message contains it") misses it.
#[test]
#[ignore = "finding: the message Debug-escapes the answer, so a line with a quote or backslash is not contained in it"]
fn a_session_message_contains_an_answer_with_a_quote() {
    let line = r#"error: "store" at C:\ekr not found"#;
    let (answer, message) = session_answer(format!("{line}\n").as_bytes());
    assert_eq!(answer, line);
    assert!(message.contains(line), "{message}");
}

/// Request content echoed into the error message. `WireReply` is `deny_unknown_fields`, and
/// `EkrBinary` checks only a minimum version, so a newer `ekr session` whose answer carries one
/// more field fails every reply as `Protocol`. The answer is then the start of that reply line,
/// whose `stderr` for a usage error is clap's message naming the offending argument as the
/// request gave it. Before this unit the error carried only serde's detail; now the argument
/// sits in the `Protocol` message and in the `Latched` cause of every later call.
#[test]
#[ignore = "finding: an answer that is JSON but not a reply puts request content (an argv value) into the error message and latch cause"]
fn a_reply_the_sdk_cannot_read_does_not_echo_the_request_into_the_message() {
    const SENTINEL: &str = "s3cr3t-value-from-the-request";
    let directory = tempfile::tempdir().unwrap();
    // A newer session: clap refuses the unknown argument and quotes it back, and the answer line
    // carries a field this SDK does not know.
    let binary = stand_in(
        directory.path(),
        "read line\n\
         arg=$(printf '%s' \"$line\" | sed 's/.*\"\\(--token=[^\"]*\\)\".*/\\1/')\n\
         printf '{\"exit\":2,\"stdout\":null,\"stderr\":\"error: unexpected argument %s found\\\\n\",\"elapsed_ms\":3}\\n' \"'$arg'\"\n\
         exec cat >/dev/null\n",
    );
    let mut session =
        ProcessSession::start(&binary, store(directory.path()), SessionOptions::default()).unwrap();
    let error = session
        .request(&Request::new(["ontology", &format!("--token={SENTINEL}")]))
        .expect_err("a reply with an unknown field is not read");
    let (answer, message) = protocol(&error);
    assert!(
        answer.contains("elapsed_ms"),
        "the stand-in answered: {answer:?}"
    );
    assert!(
        !message.contains(SENTINEL),
        "the request's argument is in the error message: {message}"
    );
    match session.request(&Request::new(["ontology"])) {
        Err(TransportError::Latched { cause, .. }) => {
            assert!(
                !cause.contains(SENTINEL),
                "and in every later call's cause: {cause}"
            );
        }
        other => panic!("expected Latched, got {other:?}"),
    }
}
