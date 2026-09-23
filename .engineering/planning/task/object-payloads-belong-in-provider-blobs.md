---
format: aep.planning-md/1
id: task:object-payloads-belong-in-provider-blobs
kind: task
status: draft
title: Move object payloads out of event bodies while preserving atomic publication
relations:
- derived_from: story:version-persisted-contracts
- serves: vision:o2
revision: 4
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

## Verified provider source adoption, 2026-09-22

Eventlog main4ee3dc23f0d02a5726a0e41d097477791f09efe2 implements strict
read-only File/SQLite inspection and native atomic blob publication across all
three providers. PR11 and its exact-source required production, comparative
and restart checks passed:
https://github.com/beyond10x/eventlog/pull/11
https://github.com/beyond10x/eventlog/actions/runs/35688825492

The coordinator advanced all three Cargo.toml selectors from tag0.2.1 to that
immutable revision, regenerated Cargo.lock, and updated the existing dependency
qualifier guard and workspace story together. The provider's SQLite inspection
dependency adds nix and cfg_aliases. Consumer tests and the final integrated gate
remain the acceptance of this dependency update.

This supplies the provider capability. EKR still needs metadata-only schema2
ObjectStored publication, versioned readers, actual durable application and
the preserving migration. No existing runtime event format has been changed
by this dependency selection alone.

## Wave p1-12 rescope

Rescoped in wave p1-12 (2026-09-23) after an evidence audit and the vectors unit's cases.

**Done, with the case that executes it:**

- New object payloads are provider blobs, and their events carry schema-2 metadata only
  (`crates/ekr-store/src/eventlog.rs:427`, `:556-561`):
  `crates/ekr-store/tests/durable_objects.rs::new_object_events_are_schema_two_metadata_with_verified_native_blobs`.
- Reads verify blob hash and length (`eventlog.rs:250-263`).
- A schema-1 inline object is refused on read and write, with its bytes left untouched:
  `crates/ekr-store/tests/current_legacy_object_refusal.rs`.

**Still open, and the only thing this task now covers:** the preserving migration of legacy
schema-1 inline objects to schema-2 metadata plus a blob (design §89, §91.6). No migration code
exists: `eventlog.rs:250-252` refuses them with "legacy inline records require migration".

This no longer blocks `story:commit-and-revision-lineage`. That story's acceptance is about
current-format persistence, and it passes (wave p1-12 gate: 680 passed). The migration belongs with
the P7 import path; that placement is a coordinator judgement, not a recorded decision.
