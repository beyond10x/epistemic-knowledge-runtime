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

PostgreSQL unit `bf94be06bc` is integrated at `37763e2ed2`. Its implementor report records
119 passing focused cases across 14 targets, zero failures or ignored cases, including 13 real
PostgreSQL cases and required previous-release compatibility. Final touched-package clippy and
format checks passed. Independent review is `review-result:hosted-postgres-final-2`; its earlier
direct provider rerun executed 12 cases, and its final correction review inspected the later logs.
The broader collection was stopped at a completed target boundary with exit 143 after 712 passes,
five failures and five existing ignores. All discovered failures have focused green corrections;
that partial run is not a full gate. The combined workspace gate remains required before landing.

The copy corrections bind completion to a fresh seed claim and prevent checkpoint shortcuts from
admitting incomplete copies or letting older readers bypass the new format. Regression corrections
verify logical content and physical hash mappings separately. The previously ignored marker-shaped
evidence case now executes and passes. HTTP integration may proceed on this reviewed foundation.

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

HTTP unit `a3a4f23079` is integrated at `45e6a2e143`. The implementor's final report records
180 affected tests passing with no failures or ignored cases, plus four final queue tests after
the private type-alias correction. Format and all-target package clippy passed. Independent
`review-result:hosted-http-final-2` records a direct 13-case process run and closes both original
contract findings, with the admission-order regression's red and green runs retained.
Subscription-backed Codex called `head` on a synthetic loopback HTTP store; Claude Code's
connection health check succeeded without a model request. Hosted consumer acceptance is pending.

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

## Combined gate findings and remaining acceptance

The search unit is integrated at f878be7af. Its independent review is
`review-result:search-entry-final-1`. Full gate attempts retained under
`<cache>/ekr-hosted-runtime/release-gate` found a read-only cleanup test assuming eager
admission, a queue regression against the unchanged 64-stream viewer case, and the SDK
returning a URL for an absent store after lazy admission. The first two corrections passed
their focused tests and reviews. SDK startup compatibility is still being corrected; the
original missing-store case remains the regression contract.

Checking the remaining packages exposed two public-entry-point guards that had not placed
the new PostgreSQL constructors. The commit-authority guard classifies these as functions
that publish no occurrence. The runtime-context guard now calls both constructors, including
read and write opens, with absent configuration references in entered and running Tokio
contexts, proving refusal before configuration or provider access. Reviews are
`review-result:hosted-store-guard-final-1` and `review-result:hosted-context-guard-final-1`;
both disclose the reviewer's earlier provider implementation authorship.

The corrected remaining-package run at cc1c43db6 reports 359 passed, 0 failed and 3 ignored across 84 completed runners.
Workspace doc tests, benchmark compilation, rustdoc, vendor checks, specification validation,
conformance freshness, planning validation and site validation also exited zero. These are
partial integration results; a full combined run after the SDK correction and required checks
on the published head remain mandatory before release.

The current combined build target is `<cache>/ekr-hosted-runtime/postgres-target`, on disk
after shared-memory quota failures. Earlier HTTP/search targets were reclaimed only after
their owners confirmed all builds and reviews had stopped. Source trees and retained evidence
remain managed and intact. The SDK correction uses its own bounded target.


## Final acceptance and release

The full local gate on 70761eaed64e3d5893a93d8ae3d5552c874a9987 passed: 2189 passed, 0 failed, 13 ignored across 358 completed runner summaries. All 10 recorded gate steps exited zero. Required real PostgreSQL and previous-release prerequisites were enabled; these scoped acceptance cases executed. Logs remain at `<cache>/ekr-hosted-runtime/release-gate/`. Existing ignored helper cases are not acceptance passes.

Exact-head GitHub Repository correctness and common / Security and privacy checks passed; correctness job https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37096359423/job/111126998509 completed successfully. Remote main and annotated 0.0.28 tag were verified at this commit. Release https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.28 was read back as published at 2026-10-03T04:56:40Z. This fulfills the scoped source-release contract, not a consumer deployment claim.

The three scoped stories are implemented. Their acceptance mappings are retained in their final sections. Earlier failed and partial runs remain historical evidence; the final complete gate supersedes pending local-gate wording above. The unchanged SDK startup and viewer concurrency regressions pass after reviewed corrections.

The parent hosted requirement stays open for atomic suffix publication and inline retained-evidence previews; search currently offers labeled evidence links. Consumer deployment and external access control remain consumer-owned acceptance. HTTP and search worktrees and disposable build targets were retired only after publication, review and exact-id managed cleanup; retained logs remain. The PostgreSQL and integration trees remain for closing verification.
