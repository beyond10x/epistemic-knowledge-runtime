//! The clap surface of the `ekr` binary and its thin dispatch to the kernel's `Runtime`.
//!
//! Verbs carry the `ekr.kernel` ESS wire names. Each store verb opens the configured provider
//! through `Runtime::file` or `Runtime::sqlite` under the trusted host document and calls exactly
//! one kernel handler or read; nothing here applies, validates or persists anything itself. The
//! agent verbs — `guide`, `operations`, `example`, `schema`, `mint`, `hash` — print static, tested
//! text, a generated JSON Schema, a fresh id or a payload's content hash, and open no provider.

mod agent;
mod commit;
mod explain;
mod hash;
mod head;
mod input;
mod ontology;
mod propose;
mod schema;
mod seed;
mod snapshot;
mod transactions;
mod validate;

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use ekr_core::Timestamp;
use ekr_kernel::Runtime;
use serde::Serialize;

pub use agent::{ExampleFormat, IdKind, OperationKind};
pub use transactions::StateFilter;

use crate::exit::Failure;
use crate::host::CliHostConfigurationV1;

/// Every verb's help ends here, so an agent that reads any one of them finds the rest.
const SEE: &str = "Start with `ekr guide`. Documents: `ekr example ekr.transaction-document/1`, \
`ekr example ekr-seed/2`, `ekr example ekr.cli-host/1`; their JSON Schemas: `ekr schema <format>`; \
operation kinds: `ekr operations`.";

/// The command-line surface of the Epistemic Knowledge Runtime.
#[derive(Debug, Parser)]
#[command(
    name = "ekr",
    version,
    about,
    long_about = "The Epistemic Knowledge Runtime. Agents propose transactions; the kernel \
validates and commits them. Run `ekr guide` for the workflow.",
    after_help = SEE
)]
pub struct Cli {
    /// Trusted host configuration: an `ekr.cli-host/1` JSON document (`ekr example ekr.cli-host/1`).
    /// Store verbs only; `EKR_HOST` when absent.
    #[arg(long, global = true)]
    pub host: Option<PathBuf>,
    /// The provider location: a directory for `file`, a database file for `sqlite`.
    /// Store verbs only; `EKR_STORE` when absent.
    #[arg(long, global = true)]
    pub store: Option<PathBuf>,
    /// The native provider. Store verbs only; `EKR_BACKEND` (`file` or `sqlite`) when absent.
    #[arg(long, value_enum, global = true)]
    pub backend: Option<Backend>,
    /// The command.
    #[command(subcommand)]
    pub command: Command,
}

/// The configured local provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    /// The file provider.
    File,
    /// The SQLite provider.
    Sqlite,
}

/// The kernel verbs, by their ESS wire names, and the agent verbs that describe them.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load the seed (`ekr.kernel.Seed`).
    ///
    /// A store verb: writes revision 0 under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Seed {
        /// An `ekr-seed/2` YAML document, or `-` for stdin (`ekr example ekr-seed/2`).
        document: PathBuf,
    },
    /// Propose a transaction (`ekr.kernel.Propose`) as the host operator.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Propose {
        /// An `ekr.transaction-document/1` YAML document, or `-` for stdin
        /// (`ekr example ekr.transaction-document/1`, `ekr operations`).
        document: PathBuf,
    },
    /// Validate a transaction (`ekr.kernel.Validate`) as the host's profile validator.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Validate {
        /// The retained transaction's id, as `ekr propose` printed it.
        transaction_id: ekr_core::TransactionId,
        /// The committed revision to validate against; the head (`ekr head`) when absent.
        #[arg(long)]
        against: Option<u64>,
    },
    /// Commit a validated transaction (`ekr.kernel.Commit`) as the host operator.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Commit {
        /// The retained transaction's id, as `ekr propose` printed it, after `ekr validate`.
        transaction_id: ekr_core::TransactionId,
    },
    /// Read a snapshot (`ekr.kernel.Snapshot`) of one verified revision.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Snapshot {
        /// The committed revision to read; the newest (`ekr head`) when absent.
        #[arg(long)]
        at: Option<u64>,
        /// Select the assertions valid at this instant, into `matching_assertions`: decimal
        /// milliseconds or `YYYY-MM-DD` (midnight UTC).
        #[arg(long, allow_negative_numbers = true, value_parser = crate::host::parse_valid_at)]
        valid_at: Option<Timestamp>,
    },
    /// Explain an assertion (`ekr.kernel.Explain`) at the newest verified revision.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Explain {
        /// The assertion's id, as `ekr snapshot` prints it.
        assertion_id: ekr_core::AssertionId,
    },
    /// Print the workflow: roles, propose → validate → commit, exit codes, where ids come from.
    #[command(after_help = SEE)]
    Guide,
    /// List the `ekr.transaction-document/1` operation kinds, or print one kind's fields and
    /// an example operation.
    #[command(after_help = SEE)]
    Operations {
        /// The operation kind, as its YAML tag without `!`.
        kind: Option<OperationKind>,
    },
    /// Print a complete example document of one input format.
    #[command(after_help = SEE)]
    Example {
        /// The format.
        format: ExampleFormat,
    },
    /// Print the JSON Schema (draft 2020-12) of one input format, generated from the types its
    /// reader decodes.
    ///
    /// Validates a document before `ekr seed` or `ekr propose` reads it. A YAML format's schema
    /// applies to the document read as plain YAML and written as JSON, where a tag `!Kind value`
    /// is the one-key object {"!Kind": value}; its description says so. Needs no store
    /// configuration.
    #[command(after_help = SEE)]
    Schema {
        /// The format: `ekr.transaction-document/1`, `ekr-seed/2` or `ekr.cli-host/1`.
        format: ExampleFormat,
    },
    /// Print a fresh id of one kind as JSON.
    #[command(after_help = SEE)]
    Mint {
        /// The id kind.
        kind: IdKind,
    },
    /// Print a payload's content hash as JSON: the `content_hash` of an `ekr-seed/2` evidence
    /// entry and the key of its `evidence_payloads` entry.
    ///
    /// The hash is sha256("ekr.payload.v1" || bytes), over the bytes exactly as given (a trailing
    /// newline counts); `payload_yaml` is the `evidence_payloads` value to paste. Needs no store
    /// configuration.
    #[command(after_help = SEE)]
    Hash {
        /// The payload file, or `-` for stdin.
        payload: PathBuf,
    },
    /// Print the head revision number and root as JSON.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Head,
    /// List retained transactions: id, state and proposer.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST).
    #[command(after_help = SEE)]
    Transactions {
        /// Only transactions in this state.
        #[arg(long, value_enum, ignore_case = true)]
        state: Option<StateFilter>,
    },
    /// Print node types, edge types and properties, by name and id, at the head.
    ///
    /// A store verb, under the `ekr.cli-host/1` host (--host or EKR_HOST). The ids `ekr ontology`
    /// prints are the `type_id`, `predicate: !Relation` and property ids a document uses.
    #[command(after_help = SEE)]
    Ontology,
}

/// The system clock in milliseconds since the Unix epoch, for a new decision only.
#[must_use]
pub fn system_time() -> Timestamp {
    let now = std::time::SystemTime::now();
    let millis = match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_millis()).map_or(i64::MIN, |m| -m),
    };
    Timestamp::from_millis(millis)
}

/// Parses `argv` and executes it. The host seam: `now` is the clock, `stdin` is `-`.
///
/// # Errors
///
/// A clap usage refusal, a named kernel refusal or an operational/configuration fault.
pub fn run<I, T>(
    argv: I,
    now: &dyn Fn() -> Timestamp,
    stdin: &mut dyn Read,
) -> Result<String, Failure>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        // The binary exits 0 for these through `Cli::parse`; the seam reports them the same way.
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return Ok(error.render().to_string());
        }
        Err(error) => {
            return Err(Failure::Usage {
                message: error.render().to_string(),
            })
        }
    };
    execute(cli, now, stdin)
}

/// A store verb's missing configuration: a usage error naming the flag and its variable.
fn required<T>(value: Option<T>, flag: &str, var: &str, verb: &str) -> Result<T, Failure> {
    value.ok_or_else(|| Failure::Usage {
        message: format!("ekr: `{verb}` needs {flag} or {var} (see `ekr guide`)"),
    })
}

/// Executes one parsed command and renders its actual result.
///
/// # Errors
///
/// A usage error, a named kernel refusal or an operational/configuration fault.
pub fn execute(
    cli: Cli,
    now: &dyn Fn() -> Timestamp,
    stdin: &mut dyn Read,
) -> Result<String, Failure> {
    let configured = Configured {
        host: cli.host,
        store: cli.store,
        backend: cli.backend,
    };
    match cli.command {
        Command::Guide => Ok(agent::GUIDE.to_owned()),
        Command::Operations { kind: None } => Ok(agent::operation_list()),
        Command::Operations { kind: Some(kind) } => Ok(agent::operation(kind)),
        Command::Example { format } => Ok(agent::example(format).to_owned()),
        Command::Schema { format } => schema::run(format),
        Command::Mint { kind } => render(&agent::mint(kind)),
        Command::Hash { payload } => render(&hash::run(&payload, stdin)?),
        Command::Seed { document } => {
            let store = configured.resolve("seed")?;
            render(&seed::run(&document, stdin, || store.open(), now)?)
        }
        Command::Propose { document } => {
            let store = configured.resolve("propose")?;
            let operator = store.host.context.operator;
            render(&propose::run(
                &document,
                stdin,
                || store.open(),
                operator,
                now,
            )?)
        }
        Command::Validate {
            transaction_id,
            against,
        } => {
            let runtime = configured.resolve("validate")?.open()?;
            let against = match against {
                Some(against) => against,
                None => head::root(&runtime)?.revision.get(),
            };
            render(&validate::run(runtime, transaction_id, against, now)?)
        }
        Command::Commit { transaction_id } => {
            let store = configured.resolve("commit")?;
            let operator = store.host.context.operator;
            render(&commit::run(store.open()?, transaction_id, operator, now)?)
        }
        Command::Snapshot { at, valid_at } => {
            let runtime = configured.resolve("snapshot")?.open()?;
            render(&snapshot::run(runtime, at, valid_at)?)
        }
        Command::Explain { assertion_id } => {
            let runtime = configured.resolve("explain")?.open()?;
            render(&explain::run(&runtime, assertion_id)?)
        }
        Command::Head => render(&head::run(&configured.resolve("head")?.open()?)?),
        Command::Transactions { state } => {
            let runtime = configured.resolve("transactions")?.open()?;
            render(&transactions::run(&runtime, state)?)
        }
        Command::Ontology => render(&ontology::run(&configured.resolve("ontology")?.open()?)?),
    }
}

/// The provider configuration as given, before a store verb needs it.
struct Configured {
    host: Option<PathBuf>,
    store: Option<PathBuf>,
    backend: Option<Backend>,
}

/// A store verb's resolved configuration: the trusted host document, read and checked.
struct Store {
    host: CliHostConfigurationV1,
    store: PathBuf,
    backend: Backend,
}

/// A flag's value, else its environment variable's; an empty value of either is unset. Read
/// only when a store verb resolves its configuration, so no other verb sees the environment.
fn flag_or_var(flag: Option<PathBuf>, var: &str) -> Option<PathBuf> {
    flag.filter(|path| !path.as_os_str().is_empty())
        .or_else(|| {
            std::env::var_os(var)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

impl Configured {
    fn resolve(self, verb: &str) -> Result<Store, Failure> {
        let host = required(
            flag_or_var(self.host, "EKR_HOST"),
            "--host",
            "EKR_HOST",
            verb,
        )?;
        let store = required(
            flag_or_var(self.store, "EKR_STORE"),
            "--store",
            "EKR_STORE",
            verb,
        )?;
        let backend = match self.backend {
            Some(backend) => Some(backend),
            None => match std::env::var("EKR_BACKEND") {
                Ok(value) if !value.is_empty() => Some(Backend::from_str(&value, false).map_err(
                    |_| Failure::Usage {
                        message: format!(
                            "ekr: EKR_BACKEND={value:?} is not a backend (`file` or `sqlite`, \
                             lowercase, as for --backend) for `{verb}`"
                        ),
                    },
                )?),
                Ok(_) | Err(std::env::VarError::NotPresent) => None,
                Err(std::env::VarError::NotUnicode(_)) => {
                    return Err(Failure::Usage {
                        message: format!(
                            "ekr: EKR_BACKEND is not text; `{verb}` needs --backend or EKR_BACKEND"
                        ),
                    })
                }
            },
        };
        let backend = required(backend, "--backend", "EKR_BACKEND", verb)?;
        let host = std::fs::read(&host)
            .map_err(|error| Failure::fault(format!("reading host {}: {error}", host.display())))
            .and_then(|bytes| {
                CliHostConfigurationV1::from_json(&bytes)
                    .map_err(|error| Failure::fault(format!("host configuration: {error}")))
            })?;
        Ok(Store {
            host,
            store,
            backend,
        })
    }
}

impl Store {
    fn open(&self) -> Result<Runtime, Failure> {
        let CliHostConfigurationV1 {
            tenant,
            context,
            authority,
            ..
        } = self.host.clone();
        match self.backend {
            Backend::File => Runtime::file(&self.store, &tenant, context, authority),
            Backend::Sqlite => Runtime::sqlite(&self.store, &tenant, context, authority),
        }
        .map_err(|error| Failure::fault(format!("opening the provider: {error}")))
    }
}

/// One JSON document. The kernel's byte strings serialise as number arrays; the CLI prints each
/// as one standard padded base64 string instead, and the guide says so. Kernel types are not
/// changed; `crates/ekr/tests/agent_cli.rs` holds every verb's output free of number arrays.
fn render(result: &impl Serialize) -> Result<String, Failure> {
    let mut value = serde_json::to_value(result).map_err(Failure::fault)?;
    bytes_as_base64(&mut value);
    let mut text = serde_json::to_string_pretty(&value).map_err(Failure::fault)?;
    text.push('\n');
    Ok(text)
}

/// The one byte-string field any verb's result carries: `ProposalRecordV1.document_bytes`,
/// wherever a result nests a proposal record (propose, commit and explain).
fn bytes_as_base64(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                match (key.as_str(), item) {
                    ("document_bytes", item) => encode(item),
                    (_, item) => bytes_as_base64(item),
                }
            }
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(bytes_as_base64),
        _ => {}
    }
}

/// Replaces a number array of bytes with its base64 string; anything else is left as it is.
fn encode(value: &mut serde_json::Value) {
    let serde_json::Value::Array(items) = value else {
        return;
    };
    let bytes: Option<Vec<u8>> = items
        .iter()
        .map(|item| item.as_u64().and_then(|n| u8::try_from(n).ok()))
        .collect();
    if let Some(bytes) = bytes {
        *value = serde_json::Value::String(base64(&bytes));
    }
}

/// Standard padded base64 (RFC 4648 § 4).
pub(super) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let triple = chunk.iter().enumerate().fold(0_u32, |acc, (at, byte)| {
            acc | (u32::from(*byte) << (16 - 8 * at))
        });
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((triple >> (18 - 6 * position)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}
