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

Current stage: accepted, before implementation. Independent review and full gate pending.

## Required verification

Real PostgreSQL with verified TLS and distinct schema/application authority; preserving source
snapshot, all logical revision roots and evidence bytes; interrupted copy refusal; credential
redaction; compatible File/SQLite and SDK behavior. Exact case names are in the story.
Run the full repository gate before integration; capture each step's exit and report skips.
