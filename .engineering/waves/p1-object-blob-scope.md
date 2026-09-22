# Object blobs: bounded source assessment, 2026-09-22

Recommendation: do not replace ObjectRecord.bytes with an unconditional put_blob followed by
append_group and call that the same seed contract. The pinned kit has no public atomic blob-write
plus append capability. Preserving the current no-losing-object contract while moving payloads out
of events needs a small coordinated upstream capability, its provider proof, and an EKR pin change.
If staging is chosen instead, it is a visible contract change requiring an explicit decision.

This assessment made no builds, source changes, live-store reads/writes, AEP changes, commits or
upstream changes. Its only output is this file. Seed tree was read only after handback.

## Evidence and exact pinned capabilities

Pinned tree: <cargo-checkouts>/eventlog-3661f9771fa6366c/77cda08
(published eventlog 0.2.1, commit prefix 77cda080).

- AGENTS.md:52, invariant 8: payload bytes/free personal text do not enter event bodies.
  RFC0020, <predecessor>/architecture/rfcs/0020-state-is-a-fold-over-an-event-log.md:
  235–237 separates uploaded bytes from events; 352–357 requires digest references and bounded
  erasure; its schema-evolution section immediately afterward preserves historical event schemas.
- eventlog-core/src/lib.rs:980–1002 declares independent put_blob(tenant,digest,bytes),
  get_blob(tenant,digest) and delete_blob(tenant,digest). put_blob returns (), not ownership of a
  newly inserted blob or a conditional lease.
- eventlog-core/src/atomic_group.rs:10–23: AppendGroup contains tenant, StreamAppend entries,
  CommandMeta; entries contain StreamId, Expected, NewEvent only. It has no blob member.
  AtomicEventStore:99–114 supports append_group and append_group_guarded, with no append-loop
  fallback.
- eventlog-core/src/projection.rs:61 supplies transaction-local get_blob only. There is no
  transaction-local put_blob/delete_blob and no public transaction escape hatch.
  docs/design/atomic-append-groups.md:44 expressly prohibits re-entering the outer store from a
  callback that already holds its transaction.
- SQLite lib.rs:1391–1436 executes blob insert/get/delete through its connection mutex separately
  from atomic_group.rs:18–117's BEGIN IMMEDIATE group transaction.
- File lib.rs:787–833 puts bytes in blobs/<object-id>, fsyncs the file/directory, then commits a
  tenant/digest binding through its own transaction. append_group_guarded:461–517 starts a
  different transaction. Its transaction method:85–141 and private Transaction state are not a
  public capability EKR can compose.
- PostgreSQL lib.rs:846–918 likewise exposes independent blob statements. Its atomic append port
  does not carry blob writes either.

Provider nuance: SQLite (and PostgreSQL) put_blob uses ON CONFLICT DO NOTHING, without comparing
the old bytes. File refuses a digest rebound to different bytes. EKR must verify exact read-back
bytes/content hash/length after put and before admitting metadata; it must not assume all providers
verify the caller's digest. File internally computes another hash to protect its physical blob,
which does not remove EKR's content-address obligation.

## Present EKR crossing

In the handed-back seed tree, crates/ekr-store/src/eventlog.rs:106–116 puts bytes: Vec<u8> inside
ObjectRecord, and NewEvent::new(OBJECT_STORED,1,body) appends the whole record. This is pre-existing
for ordinary ObjectStore::put and is now also the mechanism initialize:567 uses to put a complete
validated seed envelope into the same atomic group as Seeded.

The seed envelope itself contains full ontology, graph, provenance attribution and HumanStatement
evidence_payloads. Keeping that envelope intact as an erasable content-addressed blob is the
smallest storage migration. Do not split evidence payloads into many blobs merely to repair event
body placement: that adds a second atomicity/retention graph without being required here.

Distinguish these three things:

1. Raw blob bytes/binding in the provider blob namespace.
2. Canonical object metadata: ObjectStored/retention events establishing EKR's addressed retained
   object and its policy.
3. Revision publication: Seeded linking that object to revision zero.

With put_blob then append_group, (2) and (3) can commit together and a losing seed can leave neither
canonical metadata nor revision. But (1) has already committed. Reporting “nothing was written”
would be false, and the current ObjectStore.get(losing_hash)==None test alone would not expose it
once get requires metadata. A strict migration must also test get_blob(losing_hash)==None when no
binding existed before the losing attempt.

## Smallest coordinated change that preserves the contract

Upstream: add an explicitly selected generic atomic content+append capability (name/shape still a
design choice; for example a new request containing an AppendGroup plus digest/bytes writes).
Do not silently widen the existing group method through an external callback. The capability must:

- include actual blob content/digests in durable request identity/fingerprinting;
- compare an existing digest's exact bytes and preserve legitimate deduplication;
- publish new blob bindings, object metadata and revision events in one provider transaction,
  retaining Expected::NoStream on the lineage;
- roll back new visible blob bindings and events on any expectation/guard/projector failure;
- preserve known-commit/unknown-commit and retry semantics, including existing shared blobs;
- prove independent-handle conflicts and crash/reopen behavior through applicable provider
  conformance, before a published release/pin update.

SQLite can insert into its existing blobs table inside the existing group transaction; this needs
no destructive DDL. File can write private staged files before committing one journal transaction
containing blob binding plus append operations. Its rollback/recovery must reclaim only those
unbound private files; a process-death staging artifact is different from a committed public blob
binding. The physical staging/cleanup guarantee must be specified, not inferred: existing File
transaction returns early on work error, and its cleanup currently runs on the next transaction/
open. Existing code therefore is not already proof of immediate cleanup for an extended API.
The port should never promise that a crash performs literally zero disk I/O.

EKR after that release:

- ObjectStored schema v2 contains only content_hash, storage_class, byte_len, stored_at (and any
  explicitly chosen format/location discriminator); no embedded payload or free-text duplicate.
- Keep payload hashes and object stream identities over the exact payload bytes. Changing only
  storage placement need not change seed payload hashes; the separately planned seed/graph format
  bump may change them for its own reason.
- initialize sends the seed blob write and current atomic metadata+Seeded group through the new
  capability. Ordinary ObjectStore.put uses the same safe object publication path.
- get/fold dispatch schema versions explicitly, fetch blob by digest only for v2, and verify hash,
  length, retention and missing/erased content before kernel replay. Never fall back to an embedded
  historical copy when a v2 blob is missing.
- Extend no-write, competing first writers, cached object reuse, missing blob, corrupted blob,
  digest collision and restart tests. Assert physical event bodies contain no seed/evidence marker;
  independently check the provider blob namespace as well as EKR metadata/revision streams.
- Scope EKR store eventlog.rs/lib.rs tests, store ESS event version contract, manifests/lockfile,
  seed/format replay compatibility and migration evidence. Coordinator owns normative/planning
  scope changes; the upstream capability is a distinct dependency, not an EKR storage fork.

## Why the smaller-looking alternatives fail

Unconditional staging before NoStream may leave durable losing blobs and permanent file-provider
blob-binding journal facts. It keeps canonical metadata atomic but weakens “no write.”
A precheck before put_blob races with independent writers. put-after-append creates a published
seed whose payload is missing after interruption. delete_blob after a failed append is unsafe:
another writer may have adopted the same digest, and () from put_blob does not prove ownership.
It is also unsafe after an unknown commit. A projection storing bytes is not a substitute:
projections are rebuildable and the File provider journals their row bodies. Calling outer
put_blob inside a guard is neither a supported transaction composition nor safe from deadlock.

A staged-content alternative is implementable with current 0.2.1 only if the operator explicitly
accepts orphan/staged bytes and supplies a safe retention/reclamation protocol. That is an
alternative contract, not the promised strict fix; this assessment does not approve it.

## Preservation-first legacy boundary

Do not rewrite existing ObjectStored v1 event bodies in place or reinterpret them as v2. Decode
historical versions explicitly for inventory/verification and retain original bytes. Existing raw
GraphDocument seeds still lack ontology/attribution/evidence and remain migration-required even
after their bytes are moved to a blob.

The smallest safe forward release can stop all new inline payload events, detect old inline
object histories and require explicit migration before canonical admission of those histories.
An offline, operator-approved migration can verify every original address/length/lineage, produce
a new v2/blob-backed active store and record old→new stream/event mapping and hash receipts, while
preserving the source history as a separate archive. It must not call a data copy deletion or claim
the archived inline payload is erased.

If online v1 compatibility is retained temporarily instead, name that historical exception and its
privacy limitation: copying bytes into blobs does not remove bytes from historical event bodies.
An authorized redaction/erasure or archived-store retirement is a separate action. On no path
should erased/missing v2 content be “recovered” from old inline bytes. Fence old writers during the
format/protocol rollout; reader compatibility does not permit old binaries to keep writing v1.
