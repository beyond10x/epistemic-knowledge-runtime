//! `task:protocol-error-keeps-the-answer`: a `TransportError::Protocol` keeps the start of what
//! the `ekr` process printed, so a consumer's error says what the answer was.
//!
//! Every child here is a stand-in shell script that answers the binary handshake (`--version`).
//! Where the exact bytes matter it copies `answer.bin`, which the test wrote beside it. The script
//! is written and closed before [`EkrBinary::open`] runs it, and that probe retries a binary that
//! is still busy (`ETXTBSY`), as `adversary_session_close.rs` relies on.

#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{OneShotReader, ReadError};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig, LINE_CAP};
use ekr_sdk::transport::{Request, Transport as _, TransportError, ANSWER_BYTES, ANSWER_CUT};

const HANDSHAKE: &str = "#!/bin/sh\ncase \"$1\" in --version) echo 'ekr 0.0.20'; exit 0 ;; esac\n";

const NOT_JSON: &str = "this is not a JSON answer";

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

fn start(directory: &Path, body: &str) -> ProcessSession {
    let binary = stand_in(directory, body);
    ProcessSession::start(&binary, store(directory), SessionOptions::default()).unwrap()
}

/// The stand-in session reads one request line and answers with `answer.bin` beside it.
const SESSION_ANSWERS_THE_FILE: &str =
    "read line\ncat \"$(dirname \"$0\")/answer.bin\"\nexec cat >/dev/null\n";

/// The `answer` and message of a `Protocol` error, or a panic naming what came instead.
fn protocol(error: &TransportError) -> (String, String) {
    match error {
        TransportError::Protocol { answer, .. } => (answer.clone(), error.to_string()),
        other => panic!("expected Protocol, got {other:?}"),
    }
}

#[test]
fn a_session_answer_that_is_not_json_is_kept_in_the_protocol_error() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("answer.bin"), format!("{NOT_JSON}\n")).unwrap();
    let mut session = start(directory.path(), SESSION_ANSWERS_THE_FILE);
    let error = session
        .request(&Request::new(["ontology"]))
        .expect_err("a line that is not JSON is not a reply");
    let (answer, message) = protocol(&error);
    assert_eq!(answer, NOT_JSON);
    assert!(message.contains(NOT_JSON), "{message}");
    // The latch reports the original failure, answer included.
    match session.request(&Request::new(["ontology"])) {
        Err(TransportError::Latched { cause, .. }) => {
            assert!(cause.contains(NOT_JSON), "{cause}");
        }
        other => panic!("expected Latched, got {other:?}"),
    }
}

#[test]
fn a_one_shot_read_answer_that_is_not_json_is_kept_in_the_protocol_error() {
    let directory = tempfile::tempdir().unwrap();
    let binary = stand_in(
        directory.path(),
        &format!("cat >/dev/null\necho '{NOT_JSON}'\n"),
    );
    let mut reader =
        OneShotReader::new(&binary, store(directory.path()), SessionOptions::default());
    match reader.head() {
        Err(ReadError::Transport(error)) => {
            let (answer, message) = protocol(&error);
            assert_eq!(answer, NOT_JSON);
            assert!(message.contains(NOT_JSON), "{message}");
            assert!(
                ReadError::Transport(error).to_string().contains(NOT_JSON),
                "the read error shows the transport's message"
            );
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
}

/// A request line over [`LINE_CAP`] whose argv reads no `-` runs as its own one-shot process
/// beside the session; that process printing something that is not JSON keeps its answer too.
#[test]
fn a_session_one_shot_answer_that_is_not_json_is_kept_in_the_protocol_error() {
    let directory = tempfile::tempdir().unwrap();
    let mut session = start(
        directory.path(),
        &format!(
            "case \" $* \" in *' session --create '*) exec cat >/dev/null ;; esac\n\
             cat >/dev/null\necho '{NOT_JSON}'\n"
        ),
    );
    let request = Request::new(["hash"]).with_stdin("x".repeat(LINE_CAP + 1));
    let error = session
        .request(&request)
        .expect_err("a one-shot answer that is not JSON is not a reply");
    let (answer, message) = protocol(&error);
    assert_eq!(answer, NOT_JSON);
    assert!(message.contains(NOT_JSON), "{message}");
}

/// A long answer is cut on a character boundary: a three-byte character straddles the byte where
/// the cut would fall, so a byte-wise cut would split it.
#[test]
fn a_long_answer_is_cut_to_at_most_its_bound_on_a_character_boundary_and_marked() {
    let room = ANSWER_BYTES - ANSWER_CUT.len();
    // `€` is three bytes and starts one byte before the cut, so it would be split by it.
    let head = "a".repeat(room - 1);
    let long = format!("{head}€{}", "b".repeat(ANSWER_BYTES));
    assert!(!long.is_char_boundary(room), "the cut falls inside `€`");

    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("answer.bin"), format!("{long}\n")).unwrap();
    let mut session = start(directory.path(), SESSION_ANSWERS_THE_FILE);
    let error = session
        .request(&Request::new(["ontology"]))
        .expect_err("a line that is not JSON is not a reply");
    let (answer, message) = protocol(&error);
    assert!(answer.len() <= ANSWER_BYTES, "{} bytes", answer.len());
    assert!(answer.ends_with(ANSWER_CUT), "{answer:?}");
    assert_eq!(answer, format!("{head}{ANSWER_CUT}"));
    assert!(message.contains(&answer), "{message}");
}

/// An answer that fits is kept whole and carries no marker.
#[test]
fn an_answer_of_exactly_the_bound_is_kept_whole() {
    let exact = format!("{}é", "a".repeat(ANSWER_BYTES - 2));
    assert_eq!(exact.len(), ANSWER_BYTES);

    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("answer.bin"), format!("{exact}\n")).unwrap();
    let mut session = start(directory.path(), SESSION_ANSWERS_THE_FILE);
    let error = session
        .request(&Request::new(["ontology"]))
        .expect_err("a line that is not JSON is not a reply");
    let (answer, _) = protocol(&error);
    assert_eq!(answer, exact);
}
