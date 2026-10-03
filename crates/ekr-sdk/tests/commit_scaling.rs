//! `story:commit-cost-flat-with-store-size`: the cost of one transaction does not grow with the
//! evidence already in the store.
//!
//! A consumer measured a 2,000-operation transaction costing about twice as much in an
//! 80,000-fact delta as in a 10,000-fact delta over the same base store. This harness builds a
//! synthetic base store through the SDK, copies it, applies a small and a large delta of the same
//! shape to the two copies through one child `ekr session` each, and times every `propose`,
//! `validate` and `commit` the [`Batcher`] makes. The acceptance is that a transaction at the end
//! of the large delta costs at most [`BOUND`] times one at the end of the small delta, and that
//! the last transactions of the large delta cost at most [`BOUND`] times its first.
//! At the default sizes on SQLite, validation and commit also separately require first/last
//! 15-transaction medians within 1.2x (`task:validate-cost-flat-with-store-size`).
//!
//! Ignored: it measures time, which a loaded machine distorts, and at the acceptance's sizes it
//! takes about an hour ([`Sizes`]). Run it in release, where it builds a release `ekr`:
//!
//! ```text
//! cargo test --release -p ekr-sdk --test commit_scaling -- --ignored --nocapture
//! EKR_SCALING_QUICK=1 cargo test --release -p ekr-sdk --test commit_scaling -- --ignored --nocapture
//! EKR_SCALING_PROVIDER=sqlite cargo test --release -p ekr-sdk --test commit_scaling -- --ignored --nocapture
//! ```
//!
//! Synthetic data only: invented organisations and messages.

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use ekr_sdk::batch::Batcher;
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, Assertion, GraphRootId, NodeDraft, Object, Operation, Predicate, PropertyId, Subject,
    Timestamp, TypeId, Value,
};
use ekr_sdk::evidence::{EvidenceItem, EvidenceSet};
use ekr_sdk::reply::{Answer, Reply};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport, TransportError};
use serde_json::Value as Json;

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";

/// Operations in one transaction: 666 facts of three operations each, the most that fit in the
/// 2,000 a consumer's transactions carry.
const OPERATIONS: usize = 1_998;
/// Facts in one transaction. Each fact is one organisation, one assertion of its legal name and
/// the evidence it cites: three operations.
const FACTS_PER_TRANSACTION: usize = OPERATIONS / 3;
/// Transactions averaged at each end of a delta. The first transaction of a session is left out
/// of both: it verifies the whole history the session opened on, once. So is a last transaction
/// that carries fewer facts than the others.
const WINDOW: usize = 3;

/// Facts in the base store and in the two deltas.
///
/// The acceptance's sizes by default: a 10,000- and an 80,000-fact delta over a 20,000-fact base
/// store, about an hour on both providers. `EKR_SCALING_QUICK=1` selects whole-transaction sizes
/// that run in minutes (9, 4 and 24 transactions); `EKR_SCALING_BASE`, `EKR_SCALING_SMALL` and
/// `EKR_SCALING_LARGE` set any of the three in facts, over either.
#[derive(Clone, Copy, Debug)]
struct Sizes {
    base: usize,
    small: usize,
    large: usize,
}

impl Sizes {
    fn from_environment() -> Self {
        let quick = std::env::var("EKR_SCALING_QUICK").is_ok_and(|value| value == "1");
        let defaults = if quick {
            Self {
                base: 9 * FACTS_PER_TRANSACTION,
                small: 4 * FACTS_PER_TRANSACTION,
                large: 24 * FACTS_PER_TRANSACTION,
            }
        } else {
            Self {
                base: 20_000,
                small: 10_000,
                large: 80_000,
            }
        };
        let read = |name: &str, default: usize| {
            std::env::var(name).map_or(default, |value| {
                value
                    .parse()
                    .unwrap_or_else(|_| panic!("{name}={value} is not a number of facts"))
            })
        };
        let sizes = Self {
            base: read("EKR_SCALING_BASE", defaults.base),
            small: read("EKR_SCALING_SMALL", defaults.small),
            large: read("EKR_SCALING_LARGE", defaults.large),
        };
        for (name, facts) in [("small", sizes.small), ("large", sizes.large)] {
            assert!(
                facts / FACTS_PER_TRANSACTION > WINDOW,
                "the {name} delta of {facts} facts has too few full transactions to time: more \
                 than {WINDOW} of {FACTS_PER_TRANSACTION} facts each are needed"
            );
        }
        sizes
    }
}

/// How much more a transaction may cost at the larger store.
const BOUND: f64 = 1.5;

/// The workspace root, found at run time from the directory holding `Cargo.lock`: never
/// `env!("CARGO_MANIFEST_DIR")` (`AGENTS.md` § The gate).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// The `ekr` binary of this checkout, built once per test process through cargo, in release when
/// this test is.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let mut build = std::process::Command::new(cargo);
        build
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(Stdio::inherit());
        if !cfg!(debug_assertions) {
            build.arg("--release");
        }
        let output = build.output().expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Json>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

fn root() -> GraphRootId {
    id(ROOT)
}

fn operator() -> AgentId {
    id(OPERATOR)
}

/// `ekr <args>` from the built binary with no store: exit 0 and stdout as text.
fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// The store under `directory` on `backend`, with the example host beside it.
fn store(directory: &Path, backend: Backend) -> StoreConfig {
    StoreConfig {
        host: directory.join("host.json"),
        store: match backend {
            Backend::File => directory.join("store"),
            Backend::Sqlite => directory.join("state.db"),
            Backend::Postgres => panic!("local fixture requires a filesystem backend"),
        },
        backend,
    }
}

fn session(directory: &Path, backend: Backend) -> ProcessSession {
    let binary = EkrBinary::open(ekr_path()).unwrap();
    let options = SessionOptions {
        current_dir: Some(directory.to_path_buf()),
        timeout: Duration::from_secs(3_600),
        ..SessionOptions::default()
    };
    ProcessSession::start(&binary, store(directory, backend), options).unwrap()
}

/// Copies `from` into `to`, recursively: a closed store, as it lies on disk.
fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// Fact `n`: a new organisation and one assertion of its legal name, citing a message of its own.
fn fact(n: usize, evidence: &mut EvidenceSet) -> Vec<Operation> {
    let name = format!("org-{n}");
    let item = EvidenceItem::new(
        format!("reader-{}", n % 7),
        Timestamp::from_millis(1_790_000_000_000 + i64::try_from(n).unwrap()),
        format!("Message {n}: {name}'s legal name was read off its letterhead.\n").into_bytes(),
    );
    let cites = evidence.cite(item);
    let node = NodeDraft::new(root(), id::<TypeId>(ORGANIZATION), &name).with_alias(&name);
    let claim = Assertion::new(
        root(),
        Subject::Node(node.id),
        Predicate::Property(id::<PropertyId>(LEGAL_NAME)),
        Object::Value(Value::String(name.clone())),
        operator(),
    )
    .citing(cites);
    vec![node.into(), claim.into()]
}

/// What one transaction cost, verb by verb.
#[derive(Clone, Copy, Default)]
struct Cost {
    propose: Duration,
    validate: Duration,
    commit: Duration,
}

impl Cost {
    fn total(&self) -> Duration {
        self.propose + self.validate + self.commit
    }
}

/// A transport that times every `propose`, `validate` and `commit`; a `propose` opens the next
/// transaction.
struct Timed<'s> {
    session: &'s mut ProcessSession,
    costs: Vec<Cost>,
    input_bytes: Vec<usize>,
}

impl Transport for Timed<'_> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let started = Instant::now();
        let reply = self.session.request(request);
        let took = started.elapsed();
        match request.verb() {
            "propose" => {
                self.costs.push(Cost {
                    propose: took,
                    ..Cost::default()
                });
                self.input_bytes
                    .push(request.stdin.as_ref().map_or(0, String::len));
            }
            "validate" => {
                if let Some(cost) = self.costs.last_mut() {
                    cost.validate += took;
                }
            }
            "commit" => {
                if let Some(cost) = self.costs.last_mut() {
                    cost.commit += took;
                }
            }
            _ => {}
        }
        reply
    }
}

/// Commits facts `first..first + count` into the store under `directory` through one session,
/// and returns what each transaction cost.
fn apply(directory: &Path, backend: Backend, first: usize, count: usize) -> Vec<Cost> {
    let mut session = session(directory, backend);
    let mut evidence = EvidenceSet::new(operator());
    let groups: Vec<Vec<Operation>> = (first..first + count)
        .map(|n| fact(n, &mut evidence))
        .collect();
    let mut timed = Timed {
        session: &mut session,
        costs: Vec::new(),
        input_bytes: Vec::new(),
    };
    let report = Batcher::new(operator())
        .with_limits(OPERATIONS, 8 << 20)
        .commit_with_evidence(&mut timed, &groups, &mut evidence)
        .unwrap();
    assert!(report.rejected.is_empty(), "{:?}", report.rejected);
    assert!(report.refused.is_none(), "{:?}", report.refused);
    let costs = timed.costs;
    let input_bytes = timed.input_bytes;
    session.close().unwrap();
    for (index, bytes) in input_bytes.into_iter().enumerate() {
        println!(
            "facts {first}..{} transaction {index}: proposal_input_bytes={bytes}",
            first + count
        );
    }
    costs
}

fn mean(costs: &[Cost]) -> Cost {
    let n = u32::try_from(costs.len().max(1)).unwrap();
    let sum = costs.iter().fold(Cost::default(), |sum, cost| Cost {
        propose: sum.propose + cost.propose,
        validate: sum.validate + cost.validate,
        commit: sum.commit + cost.commit,
    });
    Cost {
        propose: sum.propose / n,
        validate: sum.validate / n,
        commit: sum.commit / n,
    }
}

/// Independent medians of the verbs, for the validate/commit scaling task's 15-transaction
/// measurement. These task assertions supplement the older whole-transaction assertions.
fn median(costs: &[Cost]) -> Cost {
    let field = |get: fn(&Cost) -> Duration| {
        let mut values: Vec<_> = costs.iter().map(get).collect();
        values.sort_unstable();
        values[values.len() / 2]
    };
    Cost {
        propose: field(|cost| cost.propose),
        validate: field(|cost| cost.validate),
        commit: field(|cost| cost.commit),
    }
}

fn line(label: &str, cost: Cost) -> String {
    format!(
        "{label:<34} propose {:>8.1} ms  validate {:>8.1} ms  commit {:>8.1} ms  total {:>8.1} ms",
        cost.propose.as_secs_f64() * 1e3,
        cost.validate.as_secs_f64() * 1e3,
        cost.commit.as_secs_f64() * 1e3,
        cost.total().as_secs_f64() * 1e3,
    )
}

/// Builds the base store, then applies the small and the large delta to copies of it, and returns
/// what went wrong against the bounds.
fn measure(backend: Backend, sizes: Sizes) -> Vec<String> {
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path().join("base");
    std::fs::create_dir_all(&base).unwrap();
    std::fs::write(
        base.join("host.json"),
        ekr_text(&["example", "ekr.cli-host/1"]),
    )
    .unwrap();
    std::fs::write(base.join("seed.yaml"), ekr_text(&["example", "ekr-seed/2"])).unwrap();
    {
        let mut session = session(&base, backend);
        let seeded = session
            .request(&Request::new(["seed", "seed.yaml"]))
            .unwrap();
        assert!(
            matches!(seeded.answer(), Answer::Outcome(_)),
            "{backend:?}: the seed is refused: {seeded:?}"
        );
        session.close().unwrap();
    }
    let started = Instant::now();
    let base_costs = apply(&base, backend, 0, sizes.base);
    println!(
        "{backend:?}: base store of {} facts in {} transactions, {:.1} s",
        sizes.base,
        base_costs.len(),
        started.elapsed().as_secs_f64()
    );

    let mut deltas = Vec::new();
    for (label, count) in [("small", sizes.small), ("large", sizes.large)] {
        let copy = directory.path().join(label);
        copy_tree(&base, &copy);
        let started = Instant::now();
        let costs = apply(&copy, backend, sizes.base, count);
        assert_eq!(
            costs.len(),
            count.div_ceil(FACTS_PER_TRANSACTION),
            "{backend:?}: the {label} delta is not one proposal per transaction"
        );
        println!(
            "{backend:?}: {label} delta of {count} facts in {} transactions, {:.1} s",
            costs.len(),
            started.elapsed().as_secs_f64()
        );
        for (at, cost) in costs.iter().enumerate() {
            println!("{}", line(&format!("  {label} transaction {at}"), *cost));
        }
        deltas.push(costs);
    }

    // Every transaction timed carries the same operations: the first of each session and a short
    // last one are left out.
    let timed = |costs: &[Cost], facts: usize| costs[1..facts / FACTS_PER_TRANSACTION].to_vec();
    let (small, large) = (
        timed(&deltas[0], sizes.small),
        timed(&deltas[1], sizes.large),
    );
    let mut wrong = Vec::new();
    if large.len() >= 30 {
        let first = median(&large[..15]);
        let last = median(&large[large.len() - 15..]);
        println!("{}", line(&format!("{backend:?} first 15 medians"), first));
        println!("{}", line(&format!("{backend:?} last 15 medians"), last));
        println!(
            "{backend:?} median ratios: validate {:.3}x; commit {:.3}x; load {}",
            last.validate.as_secs_f64() / first.validate.as_secs_f64(),
            last.commit.as_secs_f64() / first.commit.as_secs_f64(),
            std::fs::read_to_string("/proc/loadavg")
                .unwrap_or_else(|_| "unavailable".into())
                .trim(),
        );
        if matches!(backend, Backend::Sqlite)
            && (sizes.base, sizes.small, sizes.large) == (20_000, 10_000, 80_000)
        {
            for (verb, first, last) in [
                ("validate", first.validate, last.validate),
                ("commit", first.commit, last.commit),
            ] {
                let ratio = last.as_secs_f64() / first.as_secs_f64();
                if ratio > 1.2 {
                    wrong.push(format!(
                        "{backend:?}: {verb} last/first 15-transaction median {ratio:.3}x exceeds 1.2x"
                    ));
                }
            }
        }
    }
    let small_end = mean(&small[small.len() - WINDOW..]);
    let large_start = mean(&large[..WINDOW]);
    let large_end = mean(&large[large.len() - WINDOW..]);
    println!(
        "{}",
        line(
            &format!("{backend:?} small delta, last {WINDOW}"),
            small_end
        )
    );
    println!(
        "{}",
        line(
            &format!("{backend:?} large delta, first {WINDOW}"),
            large_start
        )
    );
    println!(
        "{}",
        line(
            &format!("{backend:?} large delta, last {WINDOW}"),
            large_end
        )
    );
    let across = large_end.total().as_secs_f64() / small_end.total().as_secs_f64();
    let within = large_end.total().as_secs_f64() / large_start.total().as_secs_f64();
    println!("{backend:?}: large/small {across:.2}x, large last/first {within:.2}x");

    if across > BOUND {
        wrong.push(format!(
            "{backend:?}: a transaction at the end of the large delta costs {across:.2}x one at the \
             end of the small delta"
        ));
    }
    if within > BOUND {
        wrong.push(format!(
            "{backend:?}: the last transactions of the large delta cost {within:.2}x its first"
        ));
    }
    wrong
}

/// Acceptance, on both providers: see the module documentation.
#[test]
#[ignore = "measures time; run in release with --ignored --nocapture"]
fn a_transaction_costs_the_same_in_a_large_delta_as_in_a_small_one() {
    let sizes = Sizes::from_environment();
    println!("{sizes:?} facts");
    let mut wrong = Vec::new();
    let providers = match std::env::var("EKR_SCALING_PROVIDER").as_deref() {
        Ok("sqlite") => vec![Backend::Sqlite],
        Ok("file") => vec![Backend::File],
        Err(_) => vec![Backend::Sqlite, Backend::File],
        Ok(other) => panic!("EKR_SCALING_PROVIDER={other}: expected sqlite or file"),
    };
    println!("Providers: {providers:?}");
    for backend in providers {
        wrong.extend(measure(backend, sizes));
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
