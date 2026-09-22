# Read-only provider prerequisite for EKR inventory

**Conclusion: no currently published Eventlog API inspected satisfies this contract.** EKR must not use ordinary provider opening for migration inventory. The smallest safe route is a provider-owned, non-provisioning inspection capability, released upstream and then pinned by EKR. Reuse Eventlog's own journal decoder, state fold and SQL row decoder; do not implement a parallel Eventlog parser in EKR.

This is read-only source/provenance scoping, observed 2026-09-22. No fetch, build, store opening, upstream implementation, repository edit, AEP write or ESS write ran. This report is the only artifact written. Source citations below name repository-relative paths at the stated exact revision. Runtime byte-preservation remains unexecuted.

## Exact provenance

Connectors 0.7.0 was checked first; local doctor reported no daemon, and operation discovery for GitHub refs/releases returned no admitted operations. That capability gap was reported before using read-only `git ls-remote`.

Remote `origin` advertisements and locally present objects agree:

| Source | Exact revision | Relevant capability |
|---|---|---|
| EKR dependency tag `0.2.1` | peeled commit `77cda0803b9392298952312e06b85b2d87548236`; annotated tag object `04b0287a6a48974290b795677bb7eea2c2eb9734` | No strict inspection/capture entry point. |
| Published `main` | `2e482aa1bbfcad7dc116a3eb1f442bb3512ebe8b` | The core/file/SQLite source trees are byte-identical to tag 0.2.1 (`git diff --quiet` exited 0). Upgrading merely to main does not supply the capability. |
| Initial local capture implementation | `5b4d4f83efe23f85294de7b5fef8b7de3ebdc878` | Adds capture and `FileTenantCapture`; not contained by any locally known remote branch or tag. |
| Later local capture fixes | `76aad5e3790e302e6c7f348a4fe4d9229cd17bce`, `7fbd37cf8c6d914670554fde358207b1d612a982` | Cached source is development work, not tag 0.2.1 despite its package version string. |
| Inspected local evolution integration tip | `ed6960d01f70cf4d49b143bb9c8e7d9f56a19650` | Local branches `integrate/ess-evolution-eventlog-20260915` and `impl/ess-evolution-lazy-blob-bytes-20260922`; neither appears in the current remote head advertisement. |

All remote heads and tags were advertised without fetching. Advertised tags are 0.1.0, 0.2.0 and 0.2.1; no advertised head/tag was found containing the initial capture commit. This establishes that these inspected capture branches are not an available published release dependency; it does not claim GitHub can hold no unadvertised object or pull-request ref.

The development tip still says package version 0.2.1 but changes SQLite from rusqlite 0.37 to =0.40.2 and contains substantial unrelated provider work. Its version string is not release provenance. Do not point EKR at a Cargo cache directory or assume a cached `capture.rs` belongs to the pinned tag.

## Why neither existing opener is the inventory path

At **77cda080**:

- `crates/eventlog-sqlite/src/lib.rs:65,108–140`: `open` uses `Connection::open`, sets WAL mode and executes schema creation/migration. The connection and `Inner` are private; no public constructor accepts a pre-opened read-only connection. Public read methods do not solve the unsafe opening step.
- `crates/eventlog-file/src/journal.rs:113–205`: `Journal::open` creates the root and writer lock; creates an absent manifest/history; recovers append/privacy intents; deletes unselected staging files. `crates/eventlog-file/src/lib.rs:632–639` sends even `read_stream` through the mutating transaction path.
- `crates/eventlog-core/src/lib.rs:502–522` already carries event ID, tenant/stream coordinates, stream/global positions, event name/schema version, timestamps, request ID, attribution and JSON data. Preserve that envelope instead of EKR's current `read_all` projection that returns only `recorded.data` (`crates/ekr-store/src/eventlog.rs:250–283` in EKR integration 4032d00).

At **ed6960d**, useful donor work exists, but it is not a complete substitute:

- `crates/eventlog-file/src/capture.rs:62–94` provides a separate `FileTenantCapture` with no writer methods. Its `journal::open_strict` (`journal.rs:684–746`) opens only existing files, holds the existing writer lock, rejects pending intents, validates committed length/digest and performs no initialization or cleanup. Pending-intent checking uses `symlink_metadata`, so a dangling reserved-name link is not mistaken for absence (`:749–762`). Reuse these decisions and their corrections upstream.
- File tests `tests/consistent_capture.rs:145–221,228–295` explicitly compare stored entries/bytes across successful open/capture, missing tenants, pending intents and missing roots. Those are source-inspected tests, not executions in this review.
- SQLite `open_existing` (`src/lib.rs:144–158`) uses `SQLITE_OPEN_READ_WRITE`. Its capture runs `BEGIN IMMEDIATE` (`src/capture.rs:68–95`), and the opener checks the evolved blob-table shape (`src/lib.rs:231–236`). It avoids provisioning tables but is not a read-only handle or a demonstrated no-WAL/SHM-change route for a 0.2.1 database. Do not conflate “no DML in capture” with “source bytes unchanged”.
- `TenantCapture` returns events, blobs and requested projections (`eventlog-core/src/capture.rs:37–49`), not original command-idempotency entries or exact physical frame/SQL-text bytes. EKR request IDs are present in `RecordedEvent`; if inventory also promises original request hashes/idempotency keys or physical byte verification, expose those original provider records/fingerprints rather than reconstructing them from reserialized JSON.

## Bounded upstream patch

Base a minimal compatibility-preserving addition on published Eventlog main **2e482aa** (provider format equivalent to EKR's **77cda080**). Alternatively, the Eventlog owner may finish and release the existing evolution line, but that is a larger dependency migration and still needs the SQLite non-mutation contract. Do not merge the whole evolution branch solely to gain this reader.

1. **Core read capability:** a distinct inspection handle/result with explicit finite limits and named missing-source, recovery-required, corrupt/unsupported-source, source-changed and limit refusals. Return original `RecordedEvent` metadata, source identity/fingerprints and the original provider records needed for the inventory's declared coverage. No append, recovery, identity minting or default emulation through ordinary `EventStore` methods. Keep inspection results transient; no persisted source-format change is needed.
2. **File provider:** add/export the strict reader in `eventlog-file/src/lib.rs`; share `journal.rs` framing/manifest checks and `state.rs::State::replay`. Port the existing strict-reader and pending-entry behavior above, including later fixes. Hold the source writer lock while observing history and required bindings. Enforce input-size bounds before reading the whole journal; the development strict reader's full `fs::read` is not itself a bounded-memory proof. Missing lock/root/manifest and pending intents must refuse without creating or repairing anything. Leave staging and orphan bytes untouched.
3. **SQLite provider:** add a separate read-only inspector alongside `eventlog-sqlite/src/lib.rs`, reusing its row decoder/schema knowledge while bypassing `Inner::from_connection`, DDL and writer authority. Open without CREATE/READ_WRITE and keep one explicit read transaction across the observation. Include committed WAL content; do not silently inventory only the main DB file. Require the provider to establish no source-side WAL/SHM creation, writes, deletion or recovery, including on close and failure. Return recovery-required/source-unavailable instead of invoking ordinary recovery.
4. **Tests and publication:** add provider-owned tests plus the narrow common inspection contract; no PostgreSQL, projection administration, write-path optimization or blob-format upgrade is required. Release this capability through Eventlog's normal exact-tag checks. Only then update EKR's coordinated eventlog-core/file/sqlite pins and Cargo.lock to that verified release/commit. No suitable new version exists to name today; do not invent a 0.2.2/0.3.0 pin before publication.

Likely upstream surface: `eventlog-core/src/lib.rs` plus a small inspection module; `eventlog-file/src/lib.rs`, `journal.rs` and a strict inspection module; `eventlog-sqlite/src/lib.rs` and an inspection module; provider/common inspection tests and public API documentation. Existing format decoders remain owned by Eventlog. The EKR change is then thin: store inspection adapters preserving envelopes and a kernel classifier using frozen EKR encoders. No direct SQL or file-journal decoder belongs in the EKR CLI.

## Existing SQLite primitives: useful, but not yet the supported route

The exact pinned lockfile resolves **rusqlite 0.37.0 / libsqlite3-sys 0.35.0**, bundling **SQLite 3.50.2**. `Connection::open_with_flags` supports READ_ONLY and URI flags, but Eventlog 0.2.1 does not expose such a constructor.

The bundled SQLite source also recognizes `readonly_shm=1`: its Unix opener avoids O_RDWR|O_CREAT for the shared-memory file and falls back to O_RDONLY (`sqlite3.c:43594–43606`). This is a promising provider implementation primitive that preserves normal SQLite consistency checking, not a completed Eventlog API or a runtime preservation proof. Qualify WAL/SHM behavior across the supported platforms and same-process writable handles; never assume READ_ONLY alone prevents shared-memory writes. Missing/unusable SHM or required recovery can be a named refusal rather than permission to create it.

`immutable=1` is **not a safe shortcut for an ordinary live source**. Its bundled contract explicitly disables locking and change detection and warns of incorrect results if the file changes (`sqlite3.c:4189–4197`). It is viable only against a separately established immutable snapshot, with all required WAL content and snapshot provenance preserved. No such snapshot/freeze capability was established in this task. Do not pass an immutable URI to the ordinary Eventlog opener, make a best-effort file copy, or drop the WAL to evade the prerequisite.

## Required acceptance before EKR can claim inventory support

- Exact entry/byte hashes unchanged after success, refusal and handle drop, including SQLite DB/WAL/SHM/journal and file manifest/log/lock/staging/blobs.
- Missing source, parent directory, owner tables, file lock or manifest creates nothing.
- Pending file append/privacy intents (including dangling reserved-name links), SQLite hot journal or recovery-required WAL state refuse without repair. Committed, healthy WAL is read or explicitly reported unsupported; never omitted.
- Concurrent append/checkpoint/erasure yields one consistent observation or an explicit refusal; event IDs, schema versions, stream/global positions, request identity and attribution are retained unchanged.
- Current and legacy 0.2.1 fixtures stay readable without upgrading their schema. Unknown/corrupt structures refuse. Limits apply before large allocations and do not return a misleading partial successful inventory.
- If idempotency records or raw-byte fingerprints are promised, prove their coverage separately from event-body decoding.

No upstream implementation was started. The next owner is the Eventlog maintainer/agent, with this narrow read capability as the prerequisite; the EKR implementor can continue format classification and refusal handling meanwhile, without claiming provider inspection works.
