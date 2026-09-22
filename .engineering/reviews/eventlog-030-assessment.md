# Eventlog 0.3.0 adoption assessment

Assessment date: 2026-09-22. This report is source inspection only: zero builds, zero runtime tests, zero operator-store accesses. A separately authorized synthetic integrity probe follows this assessment.

## Verdict

The released revision is source-compatible with EKR's currently consumed provider interfaces. No inspected change requires a kernel decision or Entity Runtime adoption. The minimal upgrade is the three Eventlog dependency revisions, regenerated lockfile, and the current dependency qualifier record/test, followed by consumer qualification. It must remain separate from activating V2 persistence or the writer.

Two boundaries need explicit treatment: SQLite refuses a populated predecessor blob table without an explicit trust migration, and File now has two similarly named atomic blob APIs with different retry semantics. A concrete SQLite integrity-reuse concern needs an executable probe before qualifying future blob-backed publication; source inspection alone does not establish a reproduced defect.

Owners: the coordinator owns dependency adoption, governed records, execution gates, publication and durable activation. This reviewer owns this read-only assessment. Any upstream integrity behavior remains upstream provider ownership; EKR must qualify the provider boundary it intends to use.

## Exact compared inputs

- EKR coordinator HEAD observed: `c52a9a0fd25ef141a2436e9623fc8e367f6046b8`.
- Current Eventlog pin: `4ee3dc23f0d02a5726a0e41d097477791f09efe2`.
- Released Eventlog 0.3.0 commit: `ac6b1731654329d32f1e3c9cf164fefad6a5b46a`.
- Read EKR `AGENTS.md`, `.engineering/waves/COORDINATOR.md`, and `.engineering/waves/p1-durable-activation-brief.md`.
- Upstream citations below refer to the released commit; EKR citations refer to the observed coordinator source.

No primary checkout or parser worktree was changed. Local Git objects supplied the upstream comparison; no remote metadata claim is inferred from the primary checkout's current branch.

## Current consumer and minimal adoption

EKR imports `EventStore`, `AtomicEventStore`, `AppendGroup`, `Expected`, `NewEvent` and both concrete providers (`crates/ekr-store/src/eventlog.rs:33`). Its ordinary reads use `read_stream` (`:293`), ordinary writes use `append` (`:529` onward), and initialization uses `AtomicEventStore::append_group` (`:604`, `:687`). Those interfaces remain present. The current production consumer does not yet call blob or strict-inspection APIs. Its objects still carry inline bytes.

EKR SQLite opens through `SqliteEventStore::open` (`eventlog.rs:150`); File opens through `FileEventStore::open` (`:166`). The initializer preserves `Expected::NoStream` for the lineage across object-contention retries (`:620–696`). Nothing inspected requires altering that initializer for the pin upgrade.

The three revisions are in EKR `Cargo.toml:28–30`. The literal dependency qualification roster in `crates/ekr/tests/story_contract.rs:489–515` and current scope in `.engineering/planning/story/workspace-crate-skeleton.md:91` must move together under coordinator ownership. The older adoption entry in `version-persisted-contracts.md:353` is historical evidence, not a reason to rewrite that past record. Regenerate `Cargo.lock`, retaining a coherent Eventlog bundle. Upstream now pins `rusqlite =0.40.2`; EKR has no direct rusqlite dependency in its manifests, so inspection found no immediate native-links conflict, but dependency resolution has not run.

Entity Runtime is not an EKR runtime dependency. Roadmap D1 remains the runtime's own kernel with Eventlog persistence (`docs/roadmap.md:30–44`). A separate upstream Entity Runtime release does not reopen D1.

## SQLite migration and preservation

Released `eventlog-sqlite/src/lib.rs:73` makes ordinary `open` use `LegacyBlobMigration::RefusePopulated`. The schema migration uses an immediate transaction (`:379`): an empty predecessor blob table can upgrade, while a populated predecessor table refuses by default (`:389–403`). Explicit `TrustObservedBytes` validates metadata, computes hashes over observed predecessor bytes and records the upgrade (`:404–488`). It does not prove those predecessor bytes were historically uncorrupted. Do not silently enable this trust option in EKR merely to obtain a successful open.

EKR's current inline-object implementation gives reason to expect its provider blob tables to be empty, but no live store was inspected and this is not a claim about operator data. A populated predecessor table is an operational cutover condition requiring deliberate treatment.

Ordinary SQLite open sets WAL mode before schema admission (`:183`). A refused ordinary open therefore is not an assertion that all database/journal bytes were untouched. The new `open_existing` refuses creation and predecessor migration (`:84`, `:153–159`), but still opens read/write; it is not the strict read-only inspector. Likewise File `open_existing` can recover a committed intent (`eventlog-file/src/lib.rs:111–137`). Preservation-first inspection must continue to use the dedicated inspector.

## Atomic blob API selection and retry ownership

The original `eventlog-core/src/atomic_blob.rs` is unchanged between the two compared revisions. `AtomicBlobEventStore::append_group_with_blobs(&BlobAppendGroup)` fingerprints group and exact blob bytes (`:26–46`, `:52–67`). An exact retained receipt retry returns its original result without invoking the guard or restoring erased blobs.

File 0.3.0 additionally exposes an inherent `append_group_with_blobs(&AppendGroup, &[(String, Vec<u8>)])` (`eventlog-file/src/lib.rs:248–269`). This is a different capability: its batch-bearing retry re-runs its admission guard before comparing recorded batch identities (`:288–316`). The similarly new `AtomicEventStore::append_group_guarded_with_blobs` defaults to a named unsupported refusal and is implemented by File, not a portable replacement across EKR's two providers (`eventlog-core/src/atomic_group.rs:125–171`; `eventlog-file/src/lib.rs:1107`).

The future EKR writer should explicitly select `AtomicBlobEventStore::append_group_with_blobs(&store, &request)` or its guarded counterpart. This avoids inherent-method resolution and preserves the already adopted portable receipt semantics. `FileEventStore`'s AtomicBlobEventStore implementation still checks its retained fingerprint before blob binding and callback execution (`eventlog-file/src/lib.rs:1117–1163`).

SQLite's atomic implementation likewise returns an exact prior group receipt before binding or callbacks (`eventlog-sqlite/src/atomic_group.rs:126–161`). A commit acknowledgement failure remains `UnknownCommit` (`:93–108`). File similarly distinguishes uncertain publication from pre-publication refusal and does not treat uncertainty as cleanup permission (`eventlog-file/src/lib.rs:160–223`). EKR's present `StoreError` conversion flattens Eventlog errors into a backend string (`crates/ekr-store/src/lib.rs:191–202`); that is source-compatible but the future writer must classify unknown outcome before flattening, as required by the activation brief (`:83–93`).

New `BlobMigrationCommitUnknown` and `BlobMigrationCompleted` variants do not create an exhaustive-match break in EKR's current catch-all conversion. Their actual production emission is in the PostgreSQL migration path, which EKR does not consume (`eventlog-postgres/src/schema.rs:667`, `src/lib.rs:176`). They must not be confused with proof of migration rollback if that API is adopted elsewhere.

## Concrete source concern requiring a probe

When a **new** SQLite atomic blob group reuses an existing blob, `eventlog-sqlite/src/atomic_group.rs:163–176` selects only `bytes` and compares those bytes to the request. It does not load or validate the existing `byte_count`, `integrity_sha256` and `integrity_v1` values. In contrast, ordinary `get_blob`/`put_blob` and guarded callback reads validate the full record (`eventlog-sqlite/src/lib.rs:2099–2173`, `:2609–2638`; `blob_integrity.rs:46–67`). Current-schema open does not unconditionally validate every existing row (`lib.rs:485`).

This creates a specific test hypothesis: an existing blob whose bytes are unchanged but whose stored integrity hash is corrupt may be accepted by an unguarded **fresh** atomic group, producing events that refer to a blob ordinary reads reject. That is distinct from an already committed exact receipt retry, which intentionally does not revisit content. This report has not executed that hypothesis. Qualify it with a synthetic current-schema database and exact trait call before activating the durable writer.

## Strict inspection and storage compatibility

Core inspection and File inspection source files are unchanged between the compared revisions. SQLite inspection changes its SQL count decoding from `u64` to `i64` for rusqlite 0.40 (`eventlog-sqlite/src/inspection.rs:347–350`). Its Linux cold rollback admission, process-lifetime lock-preservation registry and strict event/schema checks remain the same boundary (`:46`, `:79`, `:309–350`). History inspection remains event-history inspection, not a complete blob/archive backup.

File's group record gains a defaulted, omitted-when-empty `blobs` field (`eventlog-file/src/state.rs:37–56`), keeping the old empty-batch representation on the inspected encoding path. New native journal/cache/open behavior still warrants consumer reopen and preservation checks; unchanged inspector entry points do not establish runtime compatibility by themselves. EKR's frozen legacy DTOs/encoders are not changed by this dependency pin.

## Recommended targeted execution

1. Resolve the exact released pin and updated qualification roster, then run `cargo test --locked -p ekr --test story_contract` and `cargo test --locked -p ekr-store -p ekr-kernel`. Run the full EKR gate before integration. This report ran none of them.
2. Use synthetic File and SQLite stores written with the previous pin to check real-kernel seed/reopen, retained full ontology/evidence, identical roots and unchanged legacy event payloads under 0.3.0. Cover empty predecessor blob upgrade and default populated-predecessor refusal separately; no implicit trust migration.
3. Before activating blobs, use the exact AtomicBlobEventStore trait on both providers for tentative-read/rollback, independent-handle contention, equal-length unequal-byte rejection, exact receipt retry without callback, retry after erasure without restoration, altered-payload mismatch and unknown-outcome resolution. Do not substitute the new File-only method.
4. Execute the focused corrupt-integrity reuse hypothesis above, observing event/group-receipt/blob state before and after each attempt and distinguishing fresh publication from an existing receipt retry.
5. Retain the dedicated strict-inspection preservation controls, including File journal state and SQLite supported cold rollback versus unsupported WAL, without replacing inspection with an ordinary opener.

The upcoming activation brief already separates format activation and the real writer from this pin change (`p1-durable-activation-brief.md:42–61`, `:83–111`). No source inspection here establishes its end-to-end runtime acceptance.

## Read commands and evidence limits

Used local `git cat-file -t`, `git show <revision>:<path>`, `git diff <old> <new> -- <paths>`, `git grep <pattern> <revision> -- <paths>`, plus line-numbered reads and `rg` in the coordinator tree. Both compared objects were available as commits. The inspected unchanged-file comparison covered core atomic_blob/inspection and File inspection; SQLite inspection had only the integer decoder adaptations described above. No Rust program, cargo command, migration, provider opener or inspector was executed during this assessment.
