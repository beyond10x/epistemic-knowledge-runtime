# Preservation-first migration inventory and bounded scope

Owners: coordinator; kernel migration implementor; later P2/P7 predecessor importer.

Observed 2026-09-22 against EKR integration commit `4032d00`. Read-only filesystem metadata and source inspection only: no builds, provider open calls, replay verification, store/service/repository changes, copying, compaction or cutover. No source payload values are included. Placeholder roots below are resolved only in the private companion report.

Dependency checks: EKR pins eventlog `0.2.1`; the cited provider-open implementations were checked against that tag. V2 pins entity-runtime `0.17.7`; its DecisionCommand/DecisionRecord schema was checked against that tag. Installed service-binary provenance was not verified.

## What was found

V2 user-service configuration selects `<v2-instance>/brain.toml`. Its `[paths]` section selects `definitions`, `store`, `raw` and `views` relative to that instance. This resolution agrees with `<v2-engine>/crates/brain-core/src/config.rs:400`. EKR currently has an empty clap CLI (`crates/ekr/src/main.rs`), library constructors accepting caller-supplied paths, and temporary-store tests. No configured EKR deployment or retained EKR database/file-eventlog was found in the repository, user/system service configuration, relevant local state roots or EKR cache directories inspected. This is bounded discovery, not proof that an operator-created EKR store exists nowhere; a later inventory command must take an explicit source path and must not initialize one when absent.

Connectors 0.7.0 local discovery was used first. Doctor reported the default daemon absent; bounded operation search for local persisted-store inventory returned an empty operation list. No alternative integration client was invoked. The inventory below used ordinary filesystem metadata and structured field projections, not an EKR or predecessor store-opening API.

| V2 surface | Observed metadata |
| --- | --- |
| Store root marker | `entity.file-store/2` |
| Subject envelopes | 5,301 files, all `entity.file-subject/2`; 3,805,807,177 logical bytes |
| Subject histories | 232,033 decision records: 5,301 create, 226,732 execute; zero `legacy_import`, zero missing/null definition snapshots, zero empty record arrays |
| Other subject history | 96 observation records; zero legacy top-level domain events |
| Definitions | 36 YAML files |
| Source-cursor subjects | 207, included in the subject count |
| Raw tree | 619,487 files; 25,070,696,241 logical bytes; 27,391,832,064 allocated bytes |
| Recognized acquired-page paths | 290,779 files; 2,266,046,376 logical bytes |
| Page-retention sidecars | 19,426 files in 207 directories |
| Signal ledgers | 178 files; 245,465,943 logical bytes |
| Supervisor history | 32,521 files; 724,200,954 logical bytes |
| Other JSON | 274,495 files; 21,769,365,470 logical bytes; not semantically classified in this inventory |

All 5,301 subject files were parsed for the aggregate format/count/presence fields above. One subject's nested key/type schema was inspected. None of the 232,033 decisions was replayed, no record identity uniqueness or chain continuity was checked, and no data values or hashes were verified. The difference from the earlier survey's 5,304 subjects is an observed count difference, not evidence of what changed. These are live observations, not a frozen migration manifest.

## Actual auxiliary evidence versus hashes

V2 retains substantially more than hashes. `DecisionRecord` includes the exact definition snapshot, normalized create fields or execute arguments, before/after state, complete result, changed fields and events (`<entity-runtime>/crates/entity-core/src/runtime.rs:99`, `:122`). Its envelope retains record identity, timestamp, actor, causation and correlation (`<entity-runtime>/crates/entity-store/src/envelope.rs:9`). The all-file scan confirms definition presence and recognized command variants; it does not prove they are internally valid. Subject state and all these records live in the same envelope (`<entity-runtime>/crates/entity-store/src/file.rs:74`).

There are also 254 retained supervisor request files: request format v2=54, v3=12, v4=114, v5=74. There are 219 receipt files: receipt format v2=51, v3=14, v4=113, v5=41, plus a top-level v1 request/receipt pair. The engine retains normalized operations in decision records and extraction result bindings in receipts; newer candidate records bind request/transcript paths and digests (`<v2-engine>/crates/brain-rules/src/extract.rs:223`, `:1338`, `:1728`, `:1805`). Native extraction output can use names such as `extract-0-1.jsonl`; the absence of filenames containing “transcript” does not mean no result bytes exist. Pair completeness, digest matches, exact acknowledgment and actual validation outcomes were not established. The later inventory must join these artifacts by their recorded bindings, not assume matching filenames or request/receipt counts prove coverage.

Thirty-two retained observations expose the `brain.observation/1` envelope with submission, receipt and supporting-signal fields. Source code verifies receipt/content bindings (`<v2-engine>/crates/brain-ledger/src/observation.rs:332`). The page-retention format binds source/unit/filename, payload digest, length and local acquisition time; its reader treats missing legacy metadata as unknown (`<v2-engine>/crates/brain-core/src/acquisition.rs:19`, `:120`, `:142`). Sidecar counts alone do not establish coverage of all acquired pages or source citations.

EKR's current source has the opposite gap: revision events carry operation and validation hashes (`crates/ekr-graph/src/events.rs:48`), commit records validation in a process-local set and emits hashes without durable operation/receipt payloads (`crates/ekr-kernel/src/commit.rs:66`, `:234`, `:258`). Its old GraphDocument omits the governing ontology (`crates/ekr-store/src/snapshot.rs:121`, `:141`). No independent auxiliary EKR payload archive was discovered. If such a legacy store is supplied, migration must locate and verify the actual missing payloads against the frozen original encoders or issue named missing-history refusals. A hash, current ontology or current snapshot cannot manufacture that history.

These are two distinct conversions. EKR format migration preserves and verifies its own revision lineage. V2 import follows `docs/predecessors.md:140`: raw pages become observations; interpreted subjects remain transient and inherited; predecessor identities survive as aliases; cursors go to checkpoints; per-subject history becomes retained provenance. Full v2 decision history is valuable evidence, but does not authorize conversion into an invented global EKR revision history or immediate Accepted assertions.

## Capacity and retention

Source and scratch share one filesystem. Available bytes at inspection were 31,112,110,080. Store plus raw logical bytes total 28,876,503,418 before destination encoding, indexes, WAL, manifests or active growth. The raw allocated footprint is larger than its logical byte count. The current EKR object encoding places `Vec<u8>` in JSON (`crates/ekr-store/src/eventlog.rs:101`, `:348`), so duplicating payloads through that path can expand them materially. Duplicate-all staging is not established as feasible.

The existing v2 signal-to-page references and retention sidecars support a small metadata inventory that points back to preserved source bytes. They do not provide an EKR external-blob retention contract. V2 page publication refuses differing existing bytes, preserves missing legacy acquisition clocks as unknown, and uses a temporary hard link for publish-once (`<v2-engine>/crates/brain-core/src/acquisition.rs:107`, `:142`); this is not cross-store deduplication. No raw files with multiple hard links were observed. Do not assume reflinks, compression, automatic cleanup or source deletion will make a rehearsal fit.

Start with a metadata manifest and source fingerprints. Estimate exact destination encoding for each bounded import batch before admitting writes; keep source bytes in place and retained throughout. Deduplicate destination objects by verified content hash and checkpoint progress, but do not label a mere filesystem locator as durable imported evidence. Successful complete rehearsal needs either measured capacity for the actual destination representation or an explicitly implemented retention-backed reference/blob strategy. Unclassified JSON accounts for most raw bytes and cannot be discarded or excluded merely to make the estimate fit.

## Recommended Rust unit

Implement the read-only inventory/legacy-verification portion now through a kernel-facing API; CLI composition calls that API and never receives a raw writer. Proposed command shape is `ekr migrate inspect --source <source> --source-format <format> --report <private-report>`; it is a recommendation, not an existing command contract.

The store layer needs explicit read-only adapters retaining original event id, schema version, stream position, request identity and raw bytes. Its current `read_all` discards envelope metadata (`crates/ekr-store/src/eventlog.rs:250`). Do not call ordinary provider constructors for inspection: SQLite initialization creates schema, and file-eventlog opening can recover append/privacy state and remove files (`<eventlog-source>/crates/eventlog-sqlite/src/lib.rs:117`; `<eventlog-source>/crates/eventlog-file/src/journal.rs:130`). If the pinned eventlog port has no read-only API, add that capability upstream as a coordinated dependency change; do not quietly call a recovering open method under an inspect verb.

The kernel classifies versions with frozen decoders and hash domains, computes dependency completeness, and returns aggregate coverage plus named refusal counts. Inventory must distinguish parsed, payload-present, digest-verified and replay-verified. A live SQLite source needs a consistent read transaction including its WAL; file sources need stable manifest/fingerprint checks and a source-changed refusal. Missing source, unsupported version, redacted/unverifiable history or interrupted provider state never creates, repairs or resets source state.

Keep successful conversion and publication coupled to the durable writer. It must replay the original verified sequence into a separate destination using kernel authority, retain source identities and a verifiable migration mapping, and compare historical state/attribution before reporting success. V2's adapter is a later P2/P7 unit using the same report vocabulary, not extra implementation scope for this P1 story.

Acceptance: byte-unchanged inspection for both backends; no new files for missing paths; explicit refusal of pending recovery instead of repair; preserved occurrence metadata; old-vector verification; missing operations/ontology/validation/acceptance evidence refusals; bounded reads and source-change detection; fixture-only complete and incomplete predecessor manifests; capacity estimate includes actual serialized destination bytes. No reset, snapshot-only conversion, inferred approval or silent unsupported-version acceptance.

```markdown
## Scope

Derived 2026-09-22 by story-scoper for story:version-persisted-contracts, limited to inventory preparation.

- **Primary surface:** `crates/ekr-kernel/src/migration.rs` — inferred, new read-only classification and verification API.
- **Files:** `crates/ekr-store/src/eventlog.rs` — cited, provider opening, lost envelope metadata and object representation.
- **Files:** `crates/ekr-store/src/log.rs` — cited, replay authority and recorded validation contract.
- **Files:** `crates/ekr-kernel/src/commit.rs` — cited, current hash-only durable evidence and authority.
- **CLI:** `crates/ekr/src/main.rs` — inferred, thin inventory command composing the kernel API.
- **Tests:** `crates/ekr-kernel/tests/migration_inventory.rs` — inferred, frozen fixture and refusal tests.
- **Tests:** `crates/ekr-store/tests/read_only_inventory.rs` — inferred, provider non-mutation and metadata tests.
- **Also likely:** eventlog provider read-only port — inferred, dependency-owned prerequisite if no suitable pinned API exists.
- **Confidence:** medium — inferred, EKR surfaces are read; final seed format and upstream read-only capability are still changing/unsettled.
- **Would collide with:** kernel commit/seed integration and store eventlog/log work — cited, shared admission and serialization surfaces.
```

```sh
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/migration.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/eventlog.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/src/log.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/src/commit.rs
aep plan artifact scope story:version-persisted-contracts --add crates/ekr/src/main.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-kernel/tests/migration_inventory.rs --inferred
aep plan artifact scope story:version-persisted-contracts --add crates/ekr-store/tests/read_only_inventory.rs --inferred
```

Unestablished: explicit EKR source deployment path; complete upstream read-only inspection API availability; frozen source consistency; record replay and identity integrity; source citation/sidecar coverage; request/result/receipt joins; class of the 21.77 GB other-JSON partition; exact encoded migration capacity; final seed format. None is represented as verified here.

## Coordinator scope disposition

The proposed migrate CLI verb is not adopted for P1: its current approved surface is six verbs.
P1 inventory is a kernel library API with executable fixture tests; later operator tooling may
compose it when explicitly specified. No inference that the inspected live v2 data has passed
replay is adopted. The read-only provider prerequisite is being scoped against exact published
source before a dependency change is proposed or dispatched.
