//! The read servers' operational records (`story:readers-emit-metrics-and-request-logs`;
//! `views.yaml`, `ekr.views.ReaderMetricName` and `ekr.views.RequestLogV1`).
//!
//! `ekr view` and `ekr mcp-http` each hold one [`Telemetry`]: the metrics registry `GET /metrics`
//! answers when the server runs with `--metrics`, and the request log `--request-log` names.
//! Both count a read at their own dispatch by its wire name ([`Wire`]) and its outcome
//! ([`Outcome`]), so one read through either server moves the same series. A label value is a
//! wire name or an outcome, never request text; a log line carries the request's ids, the wire
//! name, the outcome, the status, the duration and the result size, and nothing a request or an
//! answer holds (invariant 9).
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::Write;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;

use crate::exit::Failure;

/// The content type of the exposition `GET /metrics` answers.
pub(super) const EXPOSITION: &str = "text/plain; version=0.0.4; charset=utf-8";

/// The upper bounds, in seconds, of `ekr_read_duration_seconds`'s buckets; `+Inf` follows.
const DURATION_BUCKETS: [f64; 13] = [
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, 30.0,
];
/// The upper bounds of `ekr_read_result_bytes`'s buckets; `+Inf` follows.
const BYTE_BUCKETS: [f64; 10] = [
    64.0, 256.0, 1024.0, 4096.0, 16384.0, 65536.0, 262_144.0, 1_048_576.0, 4_194_304.0,
    16_777_216.0,
];
/// The longest `X-Request-Id` kept as it came.
const REQUEST_ID_LIMIT: usize = 128;

/// A read's wire name, `ekr.views.ReadWireName`: the closed set of label values of `tool`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Wire {
    Head,
    Overview,
    Search,
    DescribeNode,
    Expand,
    Timeline,
    ChangesSince,
    Explain,
    Resolve,
    DescribeEvidence,
    Projection,
    Roles,
    Find,
}

impl Wire {
    /// Every wire name, in the order `views.yaml` declares them.
    pub(super) const ALL: [Self; 13] = [
        Self::Head,
        Self::Overview,
        Self::Search,
        Self::DescribeNode,
        Self::Expand,
        Self::Timeline,
        Self::ChangesSince,
        Self::Explain,
        Self::Resolve,
        Self::DescribeEvidence,
        Self::Projection,
        Self::Roles,
        Self::Find,
    ];

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Head => "head",
            Self::Overview => "overview",
            Self::Search => "search",
            Self::DescribeNode => "describe_node",
            Self::Expand => "expand",
            Self::Timeline => "timeline",
            Self::ChangesSince => "changes_since",
            Self::Explain => "explain",
            Self::Resolve => "resolve",
            Self::DescribeEvidence => "describe_evidence",
            Self::Projection => "projection",
            Self::Roles => "roles",
            Self::Find => "find",
        }
    }

    /// The wire name an MCP `tools/call` names, `None` for a name outside the set.
    pub(super) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|wire| wire.name() == name)
    }
}

/// A read's outcome, `ekr.views.ReadOutcome`: the closed set of label values of `outcome`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Outcome {
    Ok,
    InvalidInput,
    NotSeeded,
    NotFound,
    Unavailable,
    StoreError,
}

impl Outcome {
    pub(super) const ALL: [Self; 6] = [
        Self::Ok,
        Self::InvalidInput,
        Self::NotSeeded,
        Self::NotFound,
        Self::Unavailable,
        Self::StoreError,
    ];

    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::InvalidInput => "invalid_input",
            Self::NotSeeded => "not_seeded",
            Self::NotFound => "not_found",
            Self::Unavailable => "unavailable",
            Self::StoreError => "store_error",
        }
    }

    /// The class of the named refusal `name`, as `views.yaml` assigns it: `not_seeded`,
    /// `not_found` for every `…NotFound` and `ekr.views.NodeIsAType`, `unavailable` for
    /// `store-replaced`, and `invalid_input` for every other.
    pub(super) fn of_refusal(name: &str) -> Self {
        match name {
            "ekr.views.NotSeeded" | "ekr.kernel.NotSeeded" => Self::NotSeeded,
            "ekr.views.NodeIsAType" => Self::NotFound,
            super::session::STORE_REPLACED => Self::Unavailable,
            name if name.ends_with("NotFound") => Self::NotFound,
            _ => Self::InvalidInput,
        }
    }
}

/// One read a server dispatched: what [`Telemetry::finish`] counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Call {
    pub(super) wire: Wire,
    pub(super) outcome: Outcome,
    /// The bytes of the document the read answered; 0 when it answered none.
    pub(super) bytes: u64,
}

/// Where an id of a log line came from, `ekr.views.RequestIdSource`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Source {
    Incoming,
    Generated,
    Replaced,
}

/// One request header as it arrived: absent, once, or more than once.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum Header {
    #[default]
    Absent,
    One(String),
    Many,
}

impl Header {
    /// What a request's `header` lookup answered: `Err` for a header given more than once.
    pub(super) fn of(found: Result<Option<&str>, ()>) -> Self {
        match found {
            Ok(None) => Self::Absent,
            Ok(Some(value)) => Self::One(value.to_owned()),
            Err(()) => Self::Many,
        }
    }
}

/// A request's incoming `X-Request-Id` and `traceparent`, unchecked.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Incoming {
    pub(super) request_id: Header,
    pub(super) traceparent: Header,
}

/// The ids a request's log line carries: each the incoming one when it is well formed, else a
/// generated one, and where it came from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Ids {
    pub(super) request_id: String,
    pub(super) request_id_source: Source,
    pub(super) traceparent: String,
    pub(super) traceparent_source: Source,
}

impl Ids {
    /// The ids of a request that carried `incoming`: a malformed, oversized or repeated header
    /// is replaced, never kept.
    pub(super) fn of(incoming: &Incoming) -> Self {
        let (request_id, request_id_source) =
            kept(&incoming.request_id, request_id_admitted, generated_request_id);
        let (traceparent, traceparent_source) =
            kept(&incoming.traceparent, traceparent_admitted, generated_traceparent);
        Self {
            request_id,
            request_id_source,
            traceparent,
            traceparent_source,
        }
    }
}

fn kept(header: &Header, admitted: fn(&str) -> bool, generate: fn() -> String) -> (String, Source) {
    match header {
        Header::One(value) if admitted(value) => (value.clone(), Source::Incoming),
        Header::Absent => (generate(), Source::Generated),
        Header::One(_) | Header::Many => (generate(), Source::Replaced),
    }
}

/// 1 to 128 bytes of ASCII letters, digits and `-._:`.
fn request_id_admitted(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= REQUEST_ID_LIMIT
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-._:".contains(&b))
}

/// A W3C trace context version 00 `traceparent`: `00-<32 hex>-<16 hex>-<2 hex>`, lowercase, with
/// neither id all zeros.
fn traceparent_admitted(value: &str) -> bool {
    let hex = |part: &str, length: usize| {
        part.len() == length
            && part
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    let parts: Vec<&str> = value.split('-').collect();
    value.len() == 55
        && matches!(parts.as_slice(), [version, trace, parent, flags]
            if *version == "00"
                && hex(trace, 32)
                && hex(parent, 16)
                && hex(flags, 2)
                && trace.bytes().any(|b| b != b'0')
                && parent.bytes().any(|b| b != b'0'))
}

fn generated_request_id() -> String {
    uuid::Uuid::now_v7().as_hyphenated().to_string()
}

/// A fresh `traceparent`: a UUIDv7's 128 bits as the trace id, and the low 64 of another — never
/// zero, since a UUID's variant bits are among them — as the parent id, not sampled.
fn generated_traceparent() -> String {
    let trace = uuid::Uuid::now_v7().as_u128();
    let parent = uuid::Uuid::now_v7().as_u128() as u64;
    format!("00-{trace:032x}-{parent:016x}-00")
}

/// A histogram's per-bucket counts (not cumulative; the last is `+Inf`), sum and count.
#[derive(Clone, Debug, Default)]
struct Histogram {
    buckets: Vec<u64>,
    sum: f64,
    count: u64,
}

impl Histogram {
    fn observe(&mut self, bounds: &[f64], value: f64) {
        if self.buckets.is_empty() {
            self.buckets = vec![0; bounds.len() + 1];
        }
        let at = bounds
            .iter()
            .position(|bound| value <= *bound)
            .unwrap_or(bounds.len());
        self.buckets[at] += 1;
        self.sum += value;
        self.count += 1;
    }
}

#[derive(Debug, Default)]
struct Registry {
    requests: BTreeMap<(Wire, Outcome), u64>,
    durations: BTreeMap<Wire, Histogram>,
    bytes: BTreeMap<Wire, Histogram>,
    errors: BTreeMap<Wire, u64>,
}

/// The metrics of one server.
#[derive(Debug, Default)]
struct Metrics {
    registry: Mutex<Registry>,
    in_flight: AtomicI64,
    /// The head revision plus one; 0 while none was read.
    head: AtomicU64,
}

/// One request log line, `ekr.views.RequestLogV1`, its keys in the declared order.
#[derive(Serialize)]
struct Line<'a> {
    format: &'static str,
    server: &'static str,
    request_id: &'a str,
    request_id_source: Source,
    traceparent: &'a str,
    traceparent_source: Source,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool: Option<Wire>,
    #[serde(skip_serializing_if = "Option::is_none")]
    outcome: Option<Outcome>,
    status: u16,
    duration_seconds: f64,
    result_bytes: u64,
}

/// A server's metrics registry and request log, each present only when its option asked for it.
pub(super) struct Telemetry {
    server: &'static str,
    metrics: Option<Metrics>,
    log: Option<Mutex<Box<dyn Write + Send>>>,
}

impl std::fmt::Debug for Telemetry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Telemetry")
            .field("server", &self.server)
            .field("metrics", &self.metrics.is_some())
            .field("log", &self.log.is_some())
            .finish()
    }
}

/// Decrements the in-flight gauge when the request it counts is answered, however it ends.
pub(super) struct Waiting<'a>(Option<&'a Metrics>);

impl Drop for Waiting<'_> {
    fn drop(&mut self) {
        if let Some(metrics) = self.0 {
            metrics.in_flight.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

impl Telemetry {
    /// `server`'s telemetry: `/metrics` served when `metrics`, and a line per request written to
    /// `log` when it is given.
    pub(super) fn new(
        server: &'static str,
        metrics: bool,
        log: Option<Box<dyn Write + Send>>,
    ) -> Self {
        Self {
            server,
            metrics: metrics.then(Metrics::default),
            log: log.map(Mutex::new),
        }
    }

    /// No metrics and no log: what a server run without either option holds.
    #[cfg(test)]
    pub(super) fn off(server: &'static str) -> Self {
        Self::new(server, false, None)
    }

    /// `server`'s telemetry as its options name it: `--metrics`, and `--request-log`'s
    /// destination, `-` for standard error or a file appended to.
    ///
    /// # Errors
    ///
    /// The log file does not open for appending.
    pub(super) fn open(
        server: &'static str,
        metrics: bool,
        request_log: Option<&std::path::Path>,
    ) -> Result<Self, Failure> {
        let log: Option<Box<dyn Write + Send>> = match request_log {
            None => None,
            Some(path) if path.as_os_str() == "-" => Some(Box::new(std::io::stderr())),
            Some(path) => Some(Box::new(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|error| {
                        Failure::fault(format!(
                            "opening the request log {}: {error}",
                            path.display()
                        ))
                    })?,
            )),
        };
        Ok(Self::new(server, metrics, log))
    }

    /// Whether `GET /metrics` is served.
    pub(super) fn serves_metrics(&self) -> bool {
        self.metrics.is_some()
    }

    /// Whether anything is recorded: a server that records nothing skips the work of measuring.
    pub(super) fn records(&self) -> bool {
        self.metrics.is_some() || self.log.is_some()
    }

    /// Counts one request handed to the store's thread until the guard drops.
    pub(super) fn waiting(&self) -> Waiting<'_> {
        if let Some(metrics) = &self.metrics {
            metrics.in_flight.fetch_add(1, Ordering::SeqCst);
        }
        Waiting(self.metrics.as_ref())
    }

    /// Records the head revision the server's store holds now.
    pub(super) fn head(&self, revision: u64) {
        if let Some(metrics) = &self.metrics {
            metrics
                .head
                .store(revision.saturating_add(1), Ordering::SeqCst);
        }
    }

    /// Records one answered request: its read, if it was one, in the metrics, and its line in
    /// the log.
    pub(super) fn finish(&self, ids: &Ids, call: Option<Call>, status: u16, elapsed: Duration) {
        let seconds = elapsed.as_secs_f64();
        if let (Some(metrics), Some(call)) = (&self.metrics, call) {
            let mut registry = metrics
                .registry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            *registry
                .requests
                .entry((call.wire, call.outcome))
                .or_default() += 1;
            registry
                .durations
                .entry(call.wire)
                .or_default()
                .observe(&DURATION_BUCKETS, seconds);
            #[allow(clippy::cast_precision_loss)]
            registry
                .bytes
                .entry(call.wire)
                .or_default()
                .observe(&BYTE_BUCKETS, call.bytes as f64);
            if call.outcome == Outcome::StoreError {
                *registry.errors.entry(call.wire).or_default() += 1;
            }
        }
        if let Some(log) = &self.log {
            let line = Line {
                format: "ekr.request-log/1",
                server: self.server,
                request_id: &ids.request_id,
                request_id_source: ids.request_id_source,
                traceparent: &ids.traceparent,
                traceparent_source: ids.traceparent_source,
                tool: call.map(|call| call.wire),
                outcome: call.map(|call| call.outcome),
                status,
                duration_seconds: seconds,
                result_bytes: call.map_or(0, |call| call.bytes),
            };
            if let Ok(mut text) = serde_json::to_vec(&line) {
                text.push(b'\n');
                let mut out = log.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                // A log that cannot be written loses its line, never the answer.
                let _ = out.write_all(&text).and_then(|()| out.flush());
            }
        }
    }

    /// The Prometheus text exposition of every series that moved, each family with its HELP and
    /// TYPE lines; `ekr_read_in_flight` always, and `ekr_store_head_revision` once a head was read.
    pub(super) fn exposition(&self) -> String {
        let Some(metrics) = &self.metrics else {
            return String::new();
        };
        let registry = metrics
            .registry
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut out = String::new();
        family(
            &mut out,
            "ekr_read_requests_total",
            "counter",
            "Reads dispatched, by wire name and outcome.",
        );
        for ((wire, outcome), count) in &registry.requests {
            let _ = writeln!(
                out,
                "ekr_read_requests_total{{tool=\"{}\",outcome=\"{}\"}} {count}",
                wire.name(),
                outcome.name()
            );
        }
        family(
            &mut out,
            "ekr_read_duration_seconds",
            "histogram",
            "Seconds from a read's accept to its answer, by wire name.",
        );
        for (wire, histogram) in &registry.durations {
            series(
                &mut out,
                "ekr_read_duration_seconds",
                *wire,
                &DURATION_BUCKETS,
                histogram,
            );
        }
        family(
            &mut out,
            "ekr_read_result_bytes",
            "histogram",
            "Bytes of the document a read answered, by wire name.",
        );
        for (wire, histogram) in &registry.bytes {
            series(
                &mut out,
                "ekr_read_result_bytes",
                *wire,
                &BYTE_BUCKETS,
                histogram,
            );
        }
        family(
            &mut out,
            "ekr_read_in_flight",
            "gauge",
            "Requests handed to the store's thread and not yet answered.",
        );
        let _ = writeln!(
            out,
            "ekr_read_in_flight {}",
            metrics.in_flight.load(Ordering::SeqCst)
        );
        family(
            &mut out,
            "ekr_store_head_revision",
            "gauge",
            "The head revision of the store the reader serves.",
        );
        if let Some(head) = metrics.head.load(Ordering::SeqCst).checked_sub(1) {
            let _ = writeln!(out, "ekr_store_head_revision {head}");
        }
        family(
            &mut out,
            "ekr_store_query_errors_total",
            "counter",
            "Reads that failed on the store, by wire name; refusals are not counted.",
        );
        for (wire, count) in &registry.errors {
            let _ = writeln!(
                out,
                "ekr_store_query_errors_total{{tool=\"{}\"}} {count}",
                wire.name()
            );
        }
        out
    }
}

fn family(out: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(out, "# HELP {name} {help}\n# TYPE {name} {kind}");
}

fn series(out: &mut String, name: &str, wire: Wire, bounds: &[f64], histogram: &Histogram) {
    let tool = wire.name();
    let mut cumulative = 0;
    for (at, count) in histogram.buckets.iter().enumerate() {
        cumulative += count;
        let bound = bounds
            .get(at)
            .map_or_else(|| "+Inf".to_owned(), ToString::to_string);
        let _ = writeln!(
            out,
            "{name}_bucket{{tool=\"{tool}\",le=\"{bound}\"}} {cumulative}"
        );
    }
    let _ = writeln!(out, "{name}_sum{{tool=\"{tool}\"}} {}", histogram.sum);
    let _ = writeln!(
        out,
        "{name}_count{{tool=\"{tool}\"}} {}",
        histogram.count
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_well_formed_incoming_id_is_kept_and_any_other_replaced() {
        let trace = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";
        let ids = Ids::of(&Incoming {
            request_id: Header::One("req-1.a:b_c".into()),
            traceparent: Header::One(trace.into()),
        });
        assert_eq!(ids.request_id, "req-1.a:b_c");
        assert_eq!(ids.request_id_source, Source::Incoming);
        assert_eq!(ids.traceparent, trace);
        assert_eq!(ids.traceparent_source, Source::Incoming);
        for bad in [
            "",
            "has space",
            "quote\"",
            &"x".repeat(REQUEST_ID_LIMIT + 1),
        ] {
            let ids = Ids::of(&Incoming {
                request_id: Header::One(bad.into()),
                traceparent: Header::Absent,
            });
            assert_eq!(ids.request_id_source, Source::Replaced, "{bad:?}");
            assert_ne!(ids.request_id, bad);
            assert_eq!(ids.traceparent_source, Source::Generated);
            assert!(traceparent_admitted(&ids.traceparent), "{}", ids.traceparent);
            assert!(request_id_admitted(&ids.request_id));
        }
        for bad in [
            "00-00000000000000000000000000000000-b7ad6b7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-0000000000000000-01",
            "00-0AF7651916CD43DD8448EB211C80319C-b7ad6b7169203331-01",
            "ff-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01-extra",
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b716920333-001",
        ] {
            let ids = Ids::of(&Incoming {
                request_id: Header::Many,
                traceparent: Header::One(bad.into()),
            });
            assert_eq!(ids.traceparent_source, Source::Replaced, "{bad}");
            assert_ne!(ids.traceparent, bad);
            assert_eq!(ids.request_id_source, Source::Replaced);
        }
    }

    #[test]
    fn a_refusal_is_classed_by_its_name() {
        for (name, outcome) in [
            ("ekr.views.NotSeeded", Outcome::NotSeeded),
            ("ekr.views.RevisionNotFound", Outcome::NotFound),
            ("ekr.views.NodeNotFound", Outcome::NotFound),
            ("ekr.views.NodeIsAType", Outcome::NotFound),
            ("ekr.views.NodeTypeNotFound", Outcome::NotFound),
            ("ekr.kernel.AssertionNotFound", Outcome::NotFound),
            ("ekr.kernel.RevisionNotFound", Outcome::NotFound),
            ("store-replaced", Outcome::Unavailable),
            ("ekr.views.LimitExceeded", Outcome::InvalidInput),
            ("ekr.views.SinceMalformed", Outcome::InvalidInput),
            ("reference-without-identity", Outcome::InvalidInput),
        ] {
            assert_eq!(Outcome::of_refusal(name), outcome, "{name}");
        }
    }
}
