---
format: aep.planning-md/1
id: task:object-payloads-belong-in-provider-blobs
kind: task
status: draft
title: Move object payloads out of event bodies while preserving atomic publication
relations:
- derived_from: story:version-persisted-contracts
- blocks: story:commit-and-revision-lineage
revision: 2
---
## Confirmed persistence-contract mismatch

The upstream scope report in
`.engineering/waves/strict-history-inspection-scope.md` of the Eventlog dependency
unit identifies an existing concrete blob API and the RFC 0020 / Eventlog AGENTS
invariant that event bodies contain references instead of payload bytes. In this
runtime, `crates/ekr-store/src/eventlog.rs` stores `ObjectRecord.bytes` as a JSON
array in the ObjectStored event. Encoding bytes as integers does not meet that
boundary. The coordinator verified `EventStore::put_blob/get_blob/delete_blob`
and `AppendGroup` in the exact published provider source. The latter groups event
appends but declares no blob operation.

## Required correction before durable writer completion

Move newly persisted object payloads, including seed/evidence input, to the
provider-owned erasable blob boundary; events carry verified metadata and content
addresses. Preserve legacy bytes, metadata and hashes through explicit version
dispatch and a preserving migration. Never delete or reinterpret an old object
merely to satisfy the new representation.

Settle and implement atomic publication before changing code: the seed story's
invalid/already-seeded refusal must not leave a losing object or change retention.
A separate put_blob followed by a conditional append is not automatically that
guarantee. If staged unreferenced bytes need a changed lifecycle contract, make
that decision explicit and reviewable rather than weakening an existing test.
Do not duplicate provider storage or hide content in another JSON event field.

The scope and any necessary additive provider capability are under read-only
assessment. This task records a pre-existing integration mismatch, not a claim
that the seed unit introduced it or that the correction already executes.

## Scoped preserving implementation route

The bounded source assessment is retained at
`.engineering/waves/p1-object-blob-scope.md`. Its read-only trace found no public
atomic blob-write plus event-append capability in the current provider. Preserve
the existing no-losing-object contract: implement and independently verify the
small additive provider capability before EKR changes payload placement. Do not
choose unconditional pre-staging or delete-on-failure cleanup as an implicit
substitute. The report distinguishes private crash staging from durable public
blob bindings; freeze those guarantees in the provider design before dispatch.

The upstream scope report's references to a release are sequencing shorthand,
not a new release requirement. A verified published main commit is sufficient
under the workspace source-composition rule. Advance all EKR provider selectors
and the lockfile together. Provider schema compatibility, required checks and
exact source provenance remain mandatory.

ObjectStored v2 should retain verified metadata and addresses while bytes live
behind the provider blob API. Versioned legacy decoding must preserve source
history and refuse missing evidence; it must never recover erased v2 content
from an archived inline copy. Retain independent checks of raw blob bindings as
well as canonical metadata when proving invalid or losing publication writes
nothing visible. Full design, implementation and migration remain unexecuted.
