//! Adversary pass on wave sdk-01 unit H: `task:readers-reopen-a-replaced-store` and
//! `task:mcp-serves-the-head`.
//!
//! Every case drives the built binary: `ekr mcp` and `ekr session` held open across requests, with
//! other processes acting on the store path between two requests, as a host promoting a store
//! does.

use std::io::{BufRead as _, BufReader, Write as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Output, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

use serde_json::{json, Value};

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const NODE_A: &str = "00000000-0000-4000-8000-00000000c901";
const TX_A: &str = "00000000-0000-4000-8000-00000000c902";
const NODE_B: &str = "00000000-0000-4000-8000-00000000d901";
const TX_B: &str = "00000000-0000-4000-8000-00000000d902";
const NODE_C: &str = "00000000-0000-4000-8000-00000000e901";
const TX_C: &str = "00000000-0000-4000-8000-00000000e902";

const WITHIN: Duration = Duration::from_secs(120);

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A `CreateNode` of an Organization named `name`, in transaction `tx`.
fn create(tx: &str, node: &str, name: &str) -> String {
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {tx}\n  proposer: \
         {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: {name}\n    properties: {{}}\n    \
         aliases: [{name}]\n  evidence: []\n"
    )
}

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let path = entry.unwrap().path();
        let target = to.join(path.file_name().unwrap());
        if path.is_dir() {
            copy_tree(&path, &target);
        } else {
            std::fs::copy(&path, &target).unwrap();
        }
    }
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_owned();
    text.push(suffix);
    PathBuf::from(text)
}

/// Moves what is at `store` to `aside` and `from` into its place, each by rename, a SQLite
/// database with its `-wal` and `-shm` files.
fn replace(store: &Path, aside: &Path, from: &Path) {
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(store, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(aside, suffix)).unwrap();
        }
    }
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(from, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(store, suffix)).unwrap();
        }
    }
}

fn inode(path: &Path) -> (u64, u64) {
    let metadata = std::fs::metadata(path).unwrap();
    (metadata.dev(), metadata.ino())
}

/// Deletes the file store at `store` and creates a copy of `from` there, as
/// `rm -rf store && cp -r from store` does, in a root directory holding the inode the deleted
/// one freed. `cp -r` gets that inode when it is the lowest free one of the group, which on ext4
/// it often is (measured: the first run of these cases); this makes it certain, by creating
/// directories until one is handed the freed inode and renaming that one into place, which
/// keeps its inode.
///
/// Returns `false`, with nothing at `store`, when no directory is handed the freed inode: another
/// process on the same filesystem took it first, which a machine running other builds and tests
/// does (measured: a full `cargo test` in parallel). The inode is then gone for good, and the
/// replacement this helper exists to stage cannot be made.
fn recreate_on_the_freed_inode(store: &Path, from: &Path) -> bool {
    let freed = inode(store);
    std::fs::remove_dir_all(store).unwrap();
    let parent = store.parent().unwrap();
    let mut spare = Vec::new();
    let mut found = None;
    for n in 0..20_000 {
        let candidate = parent.join(format!(".inode-{n}"));
        std::fs::create_dir(&candidate).unwrap();
        if inode(&candidate) == freed {
            found = Some(candidate);
            break;
        }
        spare.push(candidate);
    }
    for candidate in spare {
        std::fs::remove_dir(candidate).unwrap();
    }
    let Some(found) = found else {
        return false;
    };
    std::fs::rename(&found, store).unwrap();
    copy_tree(from, store);
    assert_eq!(
        inode(store),
        freed,
        "precondition: the same device and inode"
    );
    true
}

/// [`recreate_on_the_freed_inode`], or `false` after writing why the variant is skipped to the
/// test output. The line goes to the process's standard error directly rather than through
/// `eprintln!`, which the test harness captures and shows only for a failing case.
fn recreate_on_the_freed_inode_or_skip(store: &Path, from: &Path) -> bool {
    if recreate_on_the_freed_inode(store, from) {
        return true;
    }
    let mut stderr = std::io::stderr().lock();
    writeln!(
        stderr,
        "skipped: adversary_h_mcp_answers_from_a_file_store_replaced_under_the_same_inode_\
         without_a_restart, variant \"deleted and created again\": no directory created in {} \
         was handed the inode the deleted store freed (another process on the filesystem took \
         it), so a store replaced under the same inode could not be staged",
        store.parent().unwrap().display()
    )
    .ok();
    false
}

#[path = "support/inode_directory.rs"]
mod inode_directory;

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self {
            directory: inode_directory::directory(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.file("a.yaml", &create(TX_A, NODE_A, "Alphaco"));
        world.file("b.yaml", &create(TX_B, NODE_B, "Betaco"));
        world.file("c.yaml", &create(TX_C, NODE_C, "Gammaco"));
        world.ok(&["seed", "seed.yaml"]);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).stdin(Stdio::null()).output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Proposes, validates and commits `file`, each as its own process.
    fn commit(&self, file: &str, tx: &str) {
        self.ok(&["propose", file]);
        assert_eq!(self.ok(&["validate", tx])["kind"], "Validated");
        assert_eq!(self.ok(&["commit", tx])["kind"], "Committed");
    }

    fn head(&self) -> u64 {
        self.ok(&["head"])["revision"].as_u64().unwrap()
    }

    fn spawn(&self, verb: &[&str]) -> Lines {
        Lines::spawn(self.command(verb))
    }
}

/// A process answering one line per line written.
struct Lines {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
}

impl Lines {
    fn spawn(mut command: std::process::Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if send.send(line).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            stdin: Some(stdin),
            lines,
        }
    }

    fn line(&mut self, message: &Value) -> Value {
        let input = self.stdin.as_mut().unwrap();
        input.write_all(format!("{message}\n").as_bytes()).unwrap();
        input.flush().unwrap();
        let line = self
            .lines
            .recv_timeout(WITHIN)
            .unwrap_or_else(|error| panic!("{message}: no answer: {error}"));
        serde_json::from_str(&line).unwrap_or_else(|error| panic!("{error}: {line}"))
    }

    /// A session request: the whole answer line.
    fn ask(&mut self, argv: &[&str], stdin: Option<&str>) -> Value {
        match stdin {
            Some(stdin) => self.line(&json!({"argv": argv, "stdin": stdin})),
            None => self.line(&json!({"argv": argv})),
        }
    }

    /// A session request that must exit 0: its document.
    fn ok(&mut self, argv: &[&str], stdin: Option<&str>) -> Value {
        let answer = self.ask(argv, stdin);
        assert_eq!(answer["exit"], 0, "{argv:?}: {answer}");
        answer["stdout"].clone()
    }

    fn initialize(&mut self) {
        let answer = self.line(&json!({"jsonrpc": "2.0", "id": 0, "method": "initialize",
            "params": {"protocolVersion": "2025-11-25", "capabilities": {},
                       "clientInfo": {"name": "adversary", "version": "0"}}}));
        assert!(answer["result"].is_object(), "{answer}");
        let input = self.stdin.as_mut().unwrap();
        input
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        input.flush().unwrap();
    }

    /// A tool call: its result, or the whole response when it is an error.
    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        let answer = self.line(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": tool, "arguments": arguments}}));
        match answer.get("result") {
            Some(result) => result.clone(),
            None => answer,
        }
    }

    fn close(mut self) -> Output {
        drop(self.stdin.take());
        self.child.wait_with_output().unwrap()
    }
}

/// `ekr mcp` started and initialized on `world`'s store.
fn mcp(world: &World) -> Lines {
    let mut server = world.spawn(&["mcp"]);
    server.initialize();
    server
}

/// docs/cli.md § `ekr session`: "A session holding a transaction it proposed that is neither
/// committed nor rejected does not follow a replacement". A transaction the session proposed
/// and another process then validated and committed is committed: the session holds nothing
/// open, and the next store verb must be answered from the store now at the path.
#[test]
fn adversary_h_a_session_follows_a_replacement_once_another_process_committed_its_proposal() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        let mut session = world.spawn(&["session"]);
        assert_eq!(session.ok(&["head"], None)["revision"], 0, "{backend}");
        let proposed = session.ok(&["propose", "a.yaml"], None);
        assert_eq!(proposed["transaction_id"], TX_A, "{backend}: {proposed}");

        // Another process finishes the session's proposal.
        assert_eq!(world.ok(&["validate", TX_A])["kind"], "Validated");
        assert_eq!(world.ok(&["commit", TX_A])["kind"], "Committed");
        assert_eq!(session.ok(&["head"], None)["revision"], 1, "{backend}");

        // The store is replaced by one with two commits.
        let replacement = World::seeded(backend);
        replacement.commit("b.yaml", TX_B);
        replacement.commit("c.yaml", TX_C);
        replace(
            &world.store(),
            &world.directory.path().join("aside"),
            &replacement.store(),
        );

        let head = session.ask(&["head"], None);
        assert_eq!(
            head["exit"], 0,
            "{backend}: a session whose only proposal is committed refuses the replacement: {head}"
        );
        assert_eq!(head["stdout"]["revision"], 2, "{backend}: {head}");
        session.close();
    }
}

/// Deletes every entry of the file store directory `store` and copies `from`'s into it, as
/// `rsync -a --delete from/ store/` does: the root directory, and so its device and inode, stay.
/// Always stages the replacement.
fn replace_the_files_inside(store: &Path, from: &Path) -> bool {
    for entry in std::fs::read_dir(store).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path).unwrap();
        } else {
            std::fs::remove_file(&path).unwrap();
        }
    }
    copy_tree(from, store);
    true
}

/// task:readers-reopen-a-replaced-store, Context: the reader kept answering from the replaced
/// store "until it was restarted", and the unit exists so that no restart is needed. A file
/// store replaced at its path while its root directory keeps its device and inode — deleted and
/// created again (`rm -rf store && cp -r new store`, which ext4 hands the freed inode), or its
/// files replaced inside it (`rsync --delete`) — is not seen by the identity check. The server
/// must answer, by the third call at the latest, what a server started on the store now answers.
///
/// The first variant needs the filesystem to hand the freed inode back. Where another process
/// took it first, that variant is skipped and says so in the test output; when the inode comes
/// back, the reopen is asserted as before. The second variant keeps its inode and always runs.
#[test]
fn adversary_h_mcp_answers_from_a_file_store_replaced_under_the_same_inode_without_a_restart() {
    for (how, recreate) in [
        (
            "deleted and created again",
            recreate_on_the_freed_inode_or_skip as fn(&Path, &Path) -> bool,
        ),
        ("files replaced inside", replace_the_files_inside),
    ] {
        let world = World::seeded("file");
        world.commit("a.yaml", TX_A);
        let mut server = mcp(&world);
        let described = server.call("describe_node", json!({"node": NODE_A}));
        assert_eq!(described["isError"], false, "{how}: {described}");

        let replacement = World::seeded("file");
        replacement.commit("b.yaml", TX_B);
        replacement.commit("c.yaml", TX_C);
        if !recreate(&world.store(), &replacement.store()) {
            server.close();
            continue;
        }

        let mut fresh = mcp(&world);
        let expected_head = fresh.call("head", json!({}));
        let expected_node = fresh.call("describe_node", json!({"node": NODE_A}));
        fresh.close();
        assert_eq!(
            expected_head["structuredContent"]["head"], 2,
            "{expected_head}"
        );
        assert_eq!(expected_node["isError"], true, "{expected_node}");

        let mut answers = Vec::new();
        for _ in 0..3 {
            answers.push((
                server.call("head", json!({})),
                server.call("describe_node", json!({"node": NODE_A})),
            ));
        }
        server.close();
        let (head, node) = answers.pop().unwrap();
        assert_eq!(head, expected_head, "{how}: head, third call");
        assert_eq!(node, expected_node, "{how}: describe_node, third call");
    }
}

/// GUARD (green: the suspicion is ruled out). The implementor's suspicion: SQLite deletes `<path>-wal` by name when the last connection of a
/// database closes, so a reader dropping the replaced store's connection could unlink the WAL of
/// the database now at the path while another process writes it. The WAL at the path must
/// survive the reader's reopen, and every commit the writer made must stay readable.
#[test]
fn adversary_h_closing_the_replaced_sqlite_store_leaves_the_wal_at_the_path_alone() {
    let world = World::seeded("sqlite");
    let mut server = mcp(&world);
    let head = server.call("head", json!({}));
    assert_eq!(head["structuredContent"]["head"], 0, "{head}");

    let replacement = World::seeded("sqlite");
    replace(
        &world.store(),
        &world.directory.path().join("aside.db"),
        &replacement.store(),
    );

    // Another process writes the database now at the path, and keeps it open.
    let mut writer = world.spawn(&["session"]);
    writer.ok(&["propose", "b.yaml"], None);
    writer.ok(&["validate", TX_B], None);
    assert_eq!(writer.ok(&["commit", TX_B], None)["kind"], "Committed");
    let wal = suffixed(&world.store(), "-wal");
    let wal_before = inode(&wal);

    // The reader follows the replacement: it closes its connection to the replaced database.
    let head = server.call("head", json!({}));
    assert_eq!(head["structuredContent"]["head"], 1, "{head}");
    assert_eq!(
        std::fs::metadata(&wal)
            .map(|metadata| (metadata.dev(), metadata.ino()))
            .ok(),
        Some(wal_before),
        "the WAL of the database at the path after the reader closed the replaced one"
    );

    writer.ok(&["propose", "c.yaml"], None);
    writer.ok(&["validate", TX_C], None);
    assert_eq!(writer.ok(&["commit", TX_C], None)["kind"], "Committed");
    assert_eq!(
        world.head(),
        2,
        "a one-shot reader while the writer is open"
    );
    assert_eq!(writer.close().status.code(), Some(0));
    assert_eq!(world.head(), 2, "a one-shot reader after the writer closed");
    let head = server.call("head", json!({}));
    assert_eq!(head["structuredContent"]["head"], 2, "{head}");
    server.close();
    assert_eq!(
        head_at(&world.directory.path().join("aside.db")),
        0,
        "the replaced store still opens at its new path"
    );
}

/// The head of the store at `path`, opened by a one-shot process under the same host.
fn head_at(path: &Path) -> u64 {
    let directory = path.parent().unwrap();
    let output = ekr()
        .current_dir(directory)
        .arg("--host")
        .arg(directory.join("host.json"))
        .arg("--store")
        .arg(path)
        .args(["--backend", "sqlite", "head"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let head: Value = serde_json::from_slice(&output.stdout).unwrap();
    head["revision"].as_u64().unwrap()
}
