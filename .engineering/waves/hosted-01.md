# Hosted snapshot foundation

AEP implementation skill 0.19.1. Operator authorized implementing and publishing the missing
generic upstream capabilities. One unit, `story:hosted-postgres-snapshot`, serves O2 and O5.
Authorization covers its implementation commits, reviewed corrections, integration, closing
evidence and publication. A verified source release follows as a separate authorized procedure.

## Selection and scope

Read-only story-scoper found existing generic preserving migration and eventlog PostgreSQL APIs.
This unit adds backend/configuration and consistent initial copy together because they share
runtime and CLI dispatch. Network serving follows sequentially. Incremental atomic suffix
publication and search-first entry design remain separate work.

The selection command returned wave 1: `story:hosted-postgres-snapshot`,
`story:observations-are-retained`; wave 2: `story:property-constraints-are-enforced`,
`story:sdk-source-adapter-runner`. Its collision was hosted-postgres-snapshot against
property-constraints-are-enforced at docs/cli.md. Its unassessed set was:
credential-redaction-before-model-input, facts-migrate-across-schema-versions,
poll-health-proves-its-window, reads-served-from-a-persisted-read-model,
schema-transaction-cites-evidence, source-adapter-contract, store-equivalence-check,
store-provider-migration, types-merge-and-split (all story ids). No cycles.
Only the hosted snapshot candidate is selected; the other stories are outside this delivery.
The overlapping existing provider-migration story remains open for File/SQLite directions.

## Execution and storage

Integration: `agent/hosted-runtime`, managed id `ekr-hosted-runtime`.
Unit: `impl/hosted-postgres-snapshot`, managed id `ekr-hosted-postgres`.
Trees: `<worktrees>/b10x/epistemic-knowledge-runtime/<id>`.
Unit build: `/dev/shm/ekr-hosted-postgres-target`; scratch: `<cache>/ekr-hosted-runtime/postgres`.
Only one build at a time; jobs 2, no incremental or debug information. Preflight found 8.5 GiB
free on the filesystem and 24 GiB free in executable shared memory, so disposable build output
uses the latter. Retained evidence always goes to disk. Existing active contract-adoption and
knowledge-inbox worktrees belong to other work and remain untouched.

Codex native agents run the `aep:implementor` and `aep:adversary` reference procedures; the host
does not expose plugin-specific subagent types. Root is the sole AEP writer. All committed
running code is Rust. No consumer identity, data or operational hostname enters this tree.

Current stage: PostgreSQL implementation under real-provider tests and independent review.
Initial CLI lane passed three cases; kernel lane passed six of seven and exposed a destination
emptiness observation issue. Review exposed fixed completion-marker/evidence collision.
Corrections and final gates remain pending; neither result is completed acceptance.

## HTTP unit preparation

The authorized `story:hosted-read-serving` is active. Read-only scoping established the CLI,
session refusal and viewer compatibility surfaces. Its integration remains sequential after
the PostgreSQL unit because `mod.rs`, `session.rs` and `docs/cli.md` overlap. Preparation may
run in its own tree now, restricted to the new `http.rs`, new transport tests and existing
viewer/MCP modules that the PostgreSQL unit does not edit. Do not wire the overlapping CLI
or documentation until the PostgreSQL result is integrated. This staged ownership replaces
idle waiting without pretending the complete stories are disjoint.

HTTP unit: managed id `ekr-hosted-http`, branch `impl/hosted-read-serving`, initially based on
the integration head recorded in its brief. Build `/dev/shm/ekr-hosted-http-target`; scratch
`<cache>/ekr-hosted-runtime/serving`. Preparation adds no second large build while the provider
gate is running. Preflight: 29 GiB filesystem and 19 GiB shared-memory capacity free. Native
implementor and independent adversary roles follow the same procedures. Root retains AEP,
architecture, integration and release ownership. All accepted evidence and remaining failures
are recorded before source publication.

## Required verification

Real PostgreSQL with verified TLS and distinct schema/application authority; preserving source
snapshot, all logical revision roots and evidence bytes; interrupted copy refusal; credential
redaction; compatible File/SQLite and SDK behavior. Exact case names are in the story.
Run the full repository gate on combined integration before merging to the base; capture each
step's exit and report skips. Individual units first pass their affected-package gates and review.

## Search entry unit

`story:search-first-viewer-entry` delivers the already requested search-oriented browser entry
as Rust-rendered `/find` with an ordinary GET form, bounded results and evidence/graph links.
It reuses existing search semantics and adds no scripts or external graph dependency to the
entry page. This avoids treating transport delivery as fulfillment of the separate UI request.
The full graph viewer and JSON search API remain compatible.

New renderer/test files may be prepared independently; `view.rs` and CLI docs integrate only
after the HTTP unit. The active-scope selection output is retained at
`<cache>/ekr-hosted-runtime/serving/active-waves.json`; these three hosted units deliberately
share integration surfaces and are sequenced, not asserted disjoint. Other backlog items remain
out of scope. Search unit: managed id `ekr-search-entry`, branch `impl/search-first-entry`,
base `3fef957a5b`; target `/dev/shm/ekr-search-entry-target`; scratch
`<cache>/ekr-hosted-runtime/search`. The provider implementor may prepare its new renderer
after freezing the PostgreSQL candidate, with only one of its builds active at once. Route
wiring and a complete gate wait for HTTP integration. Preflight: 20 GiB filesystem free and
18 GiB shared-memory free; all targets remain checkout-specific.
