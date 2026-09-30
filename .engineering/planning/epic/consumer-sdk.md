---
format: aep.planning-md/3
id: epic:consumer-sdk
kind: epic
status: active
title: A consumer drives an EKR store through one library and writes only its own domain code
relations:
- serves: vision:o5
- decomposes: initiative:epistemic-knowledge-runtime
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T12:21:07Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-09-30T12:21:07Z", actor: "human:timo", revision: 4}
---
## Context

A consumer instance that drives an EKR store through the `ekr` binary re-implements, around the
binary, what every consumer needs (review of 2026-09-29, `wc -l` counts): a session client with
timeouts and a failure latch (408 lines), a tag-aware YAML writer for transaction documents
(86 lines), seed and schema documents built from name→id maps (~400 lines), a resolve-once cache
with alias invalidation, `Stale` retry and bisection of a rejected batch (~550 lines), and
per-message evidence kept outside the store. None of it is instance policy. The session protocol
(`docs/cli.md`, held by `crates/ekr/tests/docs_cli.rs`) is the specified, tested surface it all
talks to.

## Outcome

A Rust consumer seeds, grows, reads and checks a store through one library crate, `ekr-sdk`, over
`ekr session`, and writes only its own domain code: no process spawning, no YAML text, no id
minting or hashing requests, no resolve cache and no batch bisection of its own. Every write it
makes is a propose→validate→commit trio (or the one seed under `--create`).

## Acceptance

- `crates/ekr-sdk/examples/fixture_consumer.rs` builds a store from a neutral fixture, on both
  providers, with the node, edge and assertion counts of the reference path.
- The example contains no `std::process`, no YAML text and no `mint` or `hash` request; a
  recording transport proves it.
- A recording transport shows every write request is a propose, validate or commit, or the seed.
- The SDK refuses, by name, an `ekr` older than its minimum version or one missing a required
  capability.
- `crates/ekr-sdk/Cargo.toml` declares no `ekr-kernel`, `ekr-store` or `ekr-graph` dependency, and a
  `crates/ekr/tests/story_contract.rs` case enforces this.

## Shape

- A library crate in this workspace, released in lockstep with the `ekr` binary. It depends on
  `ekr-core` only (`Id::mint()`, the `ekr.payload.v1` hash) and talks to a store only through a
  child `ekr session --create`, behind a `Transport` trait.
- No in-process linking of the kernel for consumers: the kernel facade is neither ESS-specified
  nor docs-tested; a child process lets one consumer drive any installed `ekr` across persisted
  format changes (`story:seed-envelope-v3-references-payloads`); `ekr-kernel` does not build as a
  dependency outside this workspace (`serde_yaml_ng::observation` exists only in the vendored copy
  applied through `[patch.crates-io]`); and the pipe costs 4.4 ms for a whole resolve on a 1.4 MB
  store, against 20–34 ms per resolve and 58–91 ms per write verb in the kernel.
- The one in-process transport lives inside crate `ekr` and is not exported:
  `story:extraction-verb-shares-the-sdk-path`.

## Decisions taken by the coordinator (2026-09-29)

| question | decision |
|---|---|
| views reads | added to `ekr session` as verbs (a `docs/cli.md` change, no new noun); one process per store |
| where extraction-apply lives | the engine verb reuses the SDK over an in-process transport (S8) |
| version skew | a minimum version plus a capability probe through `ekr operations`; no exact match; no writer-version stamp in the store for now |
| local minting | `ekr-core`'s `Id::mint()` is the function `ekr mint` runs; `AGENTS.md` line 44 is amended in `story:sdk-typed-documents` to say so |
| transaction documents | the SDK keeps a tag-aware YAML writer; readers keep refusing the one-key JSON form of a tag; no format change |
| replacing a store with a verified candidate | not filed: with `AddEvidence` (wave ingest-02) a delta lands in the current store and promote-by-copy is no longer needed |
| distribution | by git tag, like the binary; crates.io (which means publishing `ekr-core` too) is a later milestone |

## ESS first (engine nouns and wire formats, owned by their own stories)

`AddEvidence` (`story:add-evidence-operation`); the extraction document and its apply report,
including the ontology-by-name section that `OntologySpec` projects
(`story:extraction-document-applies-to-a-store`); the quality report
(`story:store-quality-report`); the code-names finding
(`story:store-reading-code-names-no-contents`); the sample draw and judged rate
(`story:fact-quality-by-judged-sample`); the source-adapter contract
(`story:source-adapter-contract`); `ChangesSince` (`story:changes-since-read`).

The SDK itself adds no noun. Its transport, typed replies, YAML writer, local mint and hash,
resolve cache, chunking, `Stale` retry, bisection and name→id map are client plumbing, held by
drift tests against `docs/cli.md`, `ekr schema` and the kernel's readers.

## Consumer comments on the draft (2026-09-29)

The reviewed consumer instance accepted the shape (child session, minimum version plus capability
probe, git tag) and asked for the following, now in the stories:

| ask | story |
|---|---|
| blocking API, no `tokio` in any feature | S1 |
| explicit binary path; never search `PATH` | S1 |
| cleared child environment, or an exact list | S1 |
| cancel from a signal handler stops the child, same failure latch | S1 |
| a recording transport that replays, so tests need no `ekr` | S1 |
| rejections reported per operation, with batch, group and issues; commits with ids | S3 |
| typed one-shot `head`, `ontology`, `snapshot`, `transactions`, `explain` | S5 |
| applying an extraction document without the engine starting any agent | S8 |

## Stories

| id | wave | depends on |
|---|---|---|
| `story:sdk-session-transport` (S1) | A | implemented session stories |
| `story:sdk-typed-documents` (S2) | A | implemented schema and identity stories |
| `story:sdk-resolve-and-batch` (S3) | B | S1, S2 |
| `story:sdk-read-helpers` (S5) | B | S1, `story:changes-since-read` for its last read |
| `story:sdk-evidence-attachment` (S4) | C | S3, `story:add-evidence-operation` |
| `story:sdk-store-checks` (S6) | C | S1 and the three check stories, one wrapper each |
| `story:extraction-verb-shares-the-sdk-path` (S8) | D | S3, S4, `story:extraction-document-applies-to-a-store` |
| `story:sdk-source-adapter-runner` (S7) | D | `story:source-adapter-contract`, `story:observations-are-retained` |

S1 and S2 both touch `crates/ekr-sdk/{Cargo.toml,src/lib.rs}` and the root `Cargo.toml`; the
coordinator owns those files.

## Non-goals

In-process kernel linking for consumers; third-party connectors (ADR 0012); any model or judge
invocation; an async API (ADR 0006: a Tokio consumer wraps calls in `spawn_blocking`); installing
binaries; run directories and promotion; instance document formats; MCP write tools; other
languages (they use the session protocol); retrying a refused write beyond `Stale`; opening store
files.

## Expected effect on the reviewed consumer (estimates, not measurements)

About 1,900 of 8,769 source lines removed with the SDK alone, about 2,550 once S8 lands. Most of
the removal in its health, code-names and quality modules comes from the three engine check
stories; the SDK's share there is the call.
