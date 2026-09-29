//! The `ekr` binary a consumer names: its version and capabilities (`story:sdk-session-transport`).
//!
//! The consumer chooses the binary and passes its path; the SDK never searches `PATH`. Opening it
//! runs `ekr --version` and refuses a binary below the minimum, and
//! [`EkrBinary::require_operations`] probes `ekr operations` for the operation kinds a consumer
//! writes.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::str::FromStr;
use std::time::{Duration, Instant};

/// The oldest `ekr` this SDK drives: 0.0.14 is the first whose `ekr session --create` serves `seed`.
pub const MINIMUM_VERSION: Version = Version::new(0, 0, 14);

/// How long `ekr --version` and `ekr operations` may take before the binary is killed and
/// refused, unless [`EkrBinary::open_with`] sets another bound.
pub const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

/// A release version, `major.minor.patch`. A pre-release or build suffix (`-rc.1`, `+abc`) is
/// read past and not compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Version {
    /// The major version.
    pub major: u64,
    /// The minor version.
    pub minor: u64,
    /// The patch version.
    pub patch: u64,
}

impl Version {
    /// `major.minor.patch`.
    pub const fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// A version that is not `major.minor.patch`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0:?} is not a version: expected major.minor.patch")]
pub struct VersionError(String);

impl FromStr for Version {
    type Err = VersionError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let core = text.split(['-', '+']).next().unwrap_or("");
        let parts: Vec<&str> = core.split('.').collect();
        let number = |part: &str| {
            (!part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                .then(|| part.parse::<u64>().ok())
                .flatten()
        };
        match parts.as_slice() {
            [major, minor, patch] => match (number(major), number(minor), number(patch)) {
                (Some(major), Some(minor), Some(patch)) => Ok(Self::new(major, minor, patch)),
                _ => Err(VersionError(text.to_owned())),
            },
            _ => Err(VersionError(text.to_owned())),
        }
    }
}

/// Why a binary was refused.
#[derive(Debug, thiserror::Error)]
pub enum BinaryError {
    /// No path was given.
    #[error(
        "no ekr binary path was given; the SDK runs the binary it is given and never searches PATH"
    )]
    NoPath,
    /// A bare name was given, which starting a process would search `PATH` for.
    #[error("{name:?} is a bare name that would be searched for on PATH; give the path of the ekr binary, such as ./{name}")]
    BareName {
        /// The name given.
        name: String,
    },
    /// The binary could not be run.
    #[error("running {path} {what}: {source}")]
    Run {
        /// The binary.
        path: PathBuf,
        /// The arguments it was run with.
        what: String,
        /// The operating system's error.
        source: std::io::Error,
    },
    /// The binary did not answer within the probe timeout; it was killed.
    #[error("{path} {what} did not answer within {timeout:?}, so it was killed")]
    TimedOut {
        /// The binary.
        path: PathBuf,
        /// The arguments it was run with.
        what: String,
        /// The probe timeout.
        timeout: Duration,
    },
    /// The binary ran and failed, or printed what `ekr` does not.
    #[error("{path} {what} did not answer as ekr does: {detail}")]
    Unexpected {
        /// The binary.
        path: PathBuf,
        /// The arguments it was run with.
        what: String,
        /// Its exit status and output.
        detail: String,
    },
    /// The binary is older than the minimum.
    #[error("{path} is ekr {found}, below the minimum version {minimum}")]
    TooOld {
        /// The binary.
        path: PathBuf,
        /// The version it reports.
        found: Version,
        /// The minimum it was held to.
        minimum: Version,
    },
    /// The binary does not know an operation kind the consumer requires.
    #[error("{path} (ekr {version}) lacks the operation kinds {missing:?}")]
    MissingOperations {
        /// The binary.
        path: PathBuf,
        /// The version it reports.
        version: Version,
        /// The required kinds `ekr operations` does not list.
        missing: Vec<String>,
    },
}

/// An `ekr` binary at a path the consumer chose, whose version has been checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EkrBinary {
    path: PathBuf,
    version: Version,
    probe_timeout: Duration,
}

impl EkrBinary {
    /// The binary at `path`, held to [`MINIMUM_VERSION`].
    pub fn open(path: impl AsRef<Path>) -> Result<Self, BinaryError> {
        Self::open_with_minimum(path, MINIMUM_VERSION)
    }

    /// The binary at `path`, held to `minimum`: `ekr --version` must report at least that.
    ///
    /// An empty path and a bare name (`ekr`) are refused before anything runs, because starting
    /// a process by a bare name searches `PATH`. A relative path is taken from the current
    /// directory, now. The version probe runs with an empty environment.
    pub fn open_with_minimum(
        path: impl AsRef<Path>,
        minimum: Version,
    ) -> Result<Self, BinaryError> {
        Self::open_with(path, minimum, PROBE_TIMEOUT)
    }

    /// [`EkrBinary::open_with_minimum`], with `probe_timeout` bounding `--version` and every
    /// later `operations` probe: a binary that has not answered by then is killed and refused.
    pub fn open_with(
        path: impl AsRef<Path>,
        minimum: Version,
        probe_timeout: Duration,
    ) -> Result<Self, BinaryError> {
        let path = path.as_ref();
        if path.as_os_str().is_empty() {
            return Err(BinaryError::NoPath);
        }
        if !path.is_absolute() && path.components().count() == 1 && !path.starts_with(".") {
            return Err(BinaryError::BareName {
                name: path.to_string_lossy().into_owned(),
            });
        }
        let path = std::path::absolute(path).map_err(|source| BinaryError::Run {
            path: path.to_path_buf(),
            what: "--version".to_owned(),
            source,
        })?;
        let printed = run(&path, &["--version"], probe_timeout)?;
        let found = printed
            .trim()
            .strip_prefix("ekr ")
            .and_then(|version| version.parse::<Version>().ok())
            .ok_or_else(|| BinaryError::Unexpected {
                path: path.clone(),
                what: "--version".to_owned(),
                detail: format!("printed {printed:?}, not `ekr <version>`"),
            })?;
        if found < minimum {
            return Err(BinaryError::TooOld {
                path,
                found,
                minimum,
            });
        }
        Ok(Self {
            path,
            version: found,
            probe_timeout,
        })
    }

    /// The binary's absolute path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The version `ekr --version` reported.
    pub fn version(&self) -> Version {
        self.version
    }

    /// The operation kinds `ekr operations` lists, in its order.
    pub fn operations(&self) -> Result<Vec<String>, BinaryError> {
        let printed = run(&self.path, &["operations"], self.probe_timeout)?;
        Ok(printed
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .map(str::to_owned)
            .collect())
    }

    /// Refuse this binary unless `ekr operations` lists every kind in `required`.
    pub fn require_operations(&self, required: &[&str]) -> Result<(), BinaryError> {
        let listed = self.operations()?;
        let missing: Vec<String> = required
            .iter()
            .filter(|kind| !listed.iter().any(|listed| listed == *kind))
            .map(|kind| (*kind).to_owned())
            .collect();
        if missing.is_empty() {
            Ok(())
        } else {
            Err(BinaryError::MissingOperations {
                path: self.path.clone(),
                version: self.version,
                missing,
            })
        }
    }
}

/// The waits before each retry of a start refused as busy: 630 ms in all.
const BUSY_RETRY_WAITS: [Duration; 6] = [
    Duration::from_millis(10),
    Duration::from_millis(20),
    Duration::from_millis(40),
    Duration::from_millis(80),
    Duration::from_millis(160),
    Duration::from_millis(320),
];

/// Start `command`, retrying while the binary is busy being written.
///
/// Linux refuses to execute a file that any process holds open for writing (`ETXTBSY`). A binary
/// just written is busy for as long as that descriptor lives, including in a child another thread
/// of the consumer forked before it execs. That refusal is retried after each of
/// [`BUSY_RETRY_WAITS`]; any other error, and the busy refusal after the last wait, is returned.
pub(crate) fn spawn(command: &mut Command) -> std::io::Result<Child> {
    let mut waits = BUSY_RETRY_WAITS.iter();
    loop {
        match command.spawn() {
            Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                match waits.next() {
                    Some(wait) => std::thread::sleep(*wait),
                    None => return Err(error),
                }
            }
            started => return started,
        }
    }
}

/// `path args…` with an empty environment and nothing on stdin: its stdout, on exit 0. A process
/// still running after `timeout` is killed and refused.
fn run(path: &Path, args: &[&str], timeout: Duration) -> Result<String, BinaryError> {
    let what = args.join(" ");
    let run_error = |source| BinaryError::Run {
        path: path.to_path_buf(),
        what: what.clone(),
        source,
    };
    let timed_out = || BinaryError::TimedOut {
        path: path.to_path_buf(),
        what: what.clone(),
        timeout,
    };
    let mut command = Command::new(path);
    command
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = spawn(&mut command).map_err(run_error)?;
    let read_all = |mut pipe: Box<dyn std::io::Read + Send>| {
        let (sent, received) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            let _ = sent.send(bytes);
        });
        received
    };
    let stdout = read_all(Box::new(child.stdout.take().expect("stdout is piped")));
    let stderr = read_all(Box::new(child.stderr.take().expect("stderr is piped")));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(timed_out());
            }
            Err(source) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(run_error(source));
            }
        }
    };
    let remaining = || {
        deadline
            .saturating_duration_since(Instant::now())
            .max(Duration::from_secs(1))
    };
    let stdout = stdout.recv_timeout(remaining()).map_err(|_| timed_out())?;
    if !status.success() {
        let stderr = stderr.recv_timeout(remaining()).unwrap_or_default();
        return Err(BinaryError::Unexpected {
            path: path.to_path_buf(),
            what,
            detail: format!("{status}; stderr {:?}", String::from_utf8_lossy(&stderr)),
        });
    }
    String::from_utf8(stdout).map_err(|_| BinaryError::Unexpected {
        path: path.to_path_buf(),
        what,
        detail: "printed text that is not UTF-8".to_owned(),
    })
}
