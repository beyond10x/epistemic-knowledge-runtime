# Bounded guard-debt scope — 2026-09-22

Read-only preparation for existing approved work, using the installed aep-drive story-scoper charter. No repository or planning edits, builds, provider access, leases, commits or publication were performed. This report is the only output written. The coordinator retains ownership of dispatch, decisions and AEP updates.

Inputs: all seven complete task bodies named below; their cited guards and contracts; the story roster; `AGENTS.md`; `.engineering/waves/COORDINATOR.md`; current manifests, toolchain and correctness workflow; current store bridge and projection tests. Coordinator source measured at `5ae71d16f1119daaadaf16d9b362162a342839be`; reviewed legacy source measured at `17a926211fa2fd7bef753d8e266805fe1c822903` with its reviewed implementation. Counts below are recalculated while writing this report.

## Recommendation and ownership

Dispatch one source-guard unit from the integrated legacy baseline: runtime checkout paths, the workspace temporal-read ban, public-type usage and README membership. These overlap each other and are small enough to share ownership. They can proceed beside upstream atomic-provider source work and coordinator-only V2 contract drafting. They must not run against the same EKR test files concurrently with actual V2 implementation.

The current story roster has no unimplemented guard-debt story. `public-surface-covers-types` derives from implemented `story:ontology-types-and-values`; `readme-status-case` derives from implemented `story:workspace-crate-skeleton`. Those are historical origins, not active ownership. The other routine tasks have no story relation. Select a dispatch owner explicitly; do not silently reopen the implemented stories. No broad planning panel is needed to make these existing tasks concrete.

`story:version-persisted-contracts` is active and owns the overlapping graph/store tests and `crates/ekr-store/src/eventlog.rs`. Its current broad scope already includes the reviewed canonical-family guard changes. The byte-representation work belongs with its activation and atomic-provider dependency; the exact `systems/ekr/domains/store.yaml` entry is presently missing from that story's recorded scope and should be added before its ESS change. Store ambient-runtime refusal has no current story relation; implemented `story:eventlog-store` and ADR 0006 are its origin.

## Runtime path task: measured scope

Task: `task:guards-read-source-through-a-compile-time-path`.

The coordinator tree has **34 executable macro uses across 18 files**. The reviewed legacy unit has **34 executable uses across 18 files**. Each tree has one additional comment quotation at `crates/ekr/tests/adversary_p1_06_authority_guard.rs:128`; it is not executable and is excluded. Every matching line was inspected, including multiline `concat!` expressions whose `env!` call is on its own line. The historical 20/39 and 19/38 figures must not be copied as current measurements.

| File (all cited) | Coordinator executable uses | Reviewed executable uses |
|---|---:|---:|
| `crates/ekr-core/tests/adversary2_public_surface.rs` | 1 | 1 |
| `crates/ekr-core/tests/identity_serde.rs` | 1 | 1 |
| `crates/ekr-core/tests/public_surface.rs` | 1 | 1 |
| `crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs` | 7 | 7 |
| `crates/ekr-graph/tests/adversary2_membrane_and_addresses.rs` | 1 | 1 |
| `crates/ekr-graph/tests/adversary_canonical_value_reach.rs` | 1 | 1 |
| `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` | 3 | 3 |
| `crates/ekr-graph/tests/canonical_value_and_assertion.rs` | 3 | 3 |
| `crates/ekr-graph/tests/domain_projection.rs` | 3 | 3 |
| `crates/ekr-graph/tests/revision_events.rs` | 1 | 1 |
| `crates/ekr-ontology/tests/domain_projection.rs` | 3 | 3 |
| `crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs` | 2 | 2 |
| `crates/ekr-store/tests/adversary2_retention_event_contract.rs` | 1 | 1 |
| `crates/ekr-store/tests/domain_projection.rs` | 2 | 2 |
| `crates/ekr/tests/adversary_docs_contract.rs` | 1 | 1 |
| `crates/ekr/tests/msrv_contract.rs` | 1 | 1 |
| `crates/ekr/tests/public_surface.rs` | 1 | 1 |
| `xtask/src/main.rs` | 1 | 1 |

At the initial a4b21d5 measurement the coordinator had 36 executable uses; the reviewed unit had 34. Integration landed during this read-only check, so the freshly measured coordinator and reviewed counts in the table now agree. The decrease is the reviewed unit's two runtime-path replacements in `canonical_value_and_assertion.rs`: the sibling `revision_events.rs` read and its `crate_modules` helper. Keep these changes and both current/frozen Canonical-family roster guards. Legacy also updates two E0277 help-list snapshots; this is diagnostic refresh, not resolution of the trybuild task.

The task's final paragraph calls Cargo reuse unsettled. Current AGENTS records the actual two-checkout reuse experiment and its conclusion; reconcile that stale task paragraph rather than treating it as a fresh unresolved investigation.

```markdown
## Scope

- **Primary surface:** source-reading test helpers and xtask doctor — cited.
- **Files:** the eighteen exact paths in the measurement table — cited.
- **Symbols:** runtime crate/workspace-root helpers, inline source readers and xtask::doctor — cited.
- **Documents:** AGENTS.md source-path debt wording and task:guards-read-source-through-a-compile-time-path's stale counts/mechanism, coordinator-owned — cited.
- **Acceptance:** all executable source-location uses read the runtime checkout; the comment quote stays a comment; preserve the current/frozen Canonical roster and all guard assertions — inferred from the task and reviewed diff.
- **Confidence:** high, every current use was read and counted — cited.
- **Would collide with:** canonical_value_and_assertion.rs legacy/V2 guards, graph projection tests, store projection tests, and the public-surface/README guards in this same unit — cited.
```

Smallest correction: replace compile-time paths with `std::env::var("CARGO_MANIFEST_DIR")` and owned `PathBuf` joins. This is not literally one token for `concat!` or borrowed temporaries. Runtime directory walking is only needed where invocation lacks Cargo's per-process manifest variable; decide whether direct standalone xtask invocation is supported and give a clear failure/fallback accordingly. No build script and no cargo-clean workaround.

Acceptance checks: zero executable forbidden macros; measured nonzero source scan; ordinary affected package suites; xtask doctor from the invoking tree; one two-checkout witness using the same compiled guard binary and a distinguishing source mutation in the runtime checkout. The second checkout must fail on its mutation while the original remains unmodified; removing the original checkout is not necessary. Guard compilation may use isolated assigned targets normally; deliberately shared artifacts belong only in the scoped regression experiment. Do not count the documentation quote or weaken assertions to make source scanning green.

## Workspace temporal-read task

Task: `task:open-ended-read-ban-binds-one-crate`.

Still active. `adversary_snapshot_and_assertion.rs` scans direct graph src files only. `adversary2_guard_bounds_and_ranges.rs` repeats the local checks; neither covers kernel/store consumers or nested source modules. The source predicate is specifically valid-time open-ended filtering; legitimate `TransactionTime::is_open()` remains allowed. The graph test deliberately constructs the deleted filter as a negative semantic witness and must remain outside the product-source ban.

```markdown
## Scope

- **Primary surface:** workspace source guard under crates/ekr/tests — inferred.
- **Files:** crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs and crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs, existing local guard and its adversary — cited.
- **Also likely:** crates/ekr/tests/temporal_reads.rs, a new recursively scanned workspace guard — inferred.
- **Symbols:** the valid_time.is_open / valid_time.to.is_none ban and workspace recursive Rust-source discovery — cited.
- **Documents:** update the local follow-up comment to name the actual workspace owner/case — inferred.
- **Confidence:** high, the missing cross-crate coverage is visible in both current guards — cited.
- **Would collide with:** runtime-path edits in both graph guards and later V2 assertion/snapshot test changes — cited.
```

Acceptance: placing the prohibited read in graph, kernel, store and a nested src module trips the workspace case; current src remains green; comments/test negative witnesses do not create false product violations. Preserve fixed-term and future-successor temporal behavior cases. No public-field privacy change, new temporal API or reversal of the deleted active() decision is needed. Minimal execution is the new ekr guard plus both existing graph guard targets and their semantic cases, followed by the combined unit gate. A raw literal scanner is a policy tripwire, not a proof that arbitrary equivalent expressions are impossible; avoid claiming the latter.

## Public-type usage task

Task: `task:public-surface-covers-types`.

Still active. `declared_public_name` accepts only pub fn/pub const fn/pub const. `is_used` recognizes method/path suffixes, and comments are stripped from pooled tests. Recursive crate discovery already exists. The recent Candidate helper visibility repair illustrates why pub(crate)/pub(super) must not be treated as exported API.

```markdown
## Scope

- **Primary surface:** crates/ekr/tests/public_surface.rs — cited.
- **Symbols:** declared_public_name, is_used, without_comments and no_public_item_in_any_crate_is_untested — cited.
- **Documents:** this guard's introductory limitations — cited.
- **Confidence:** high, the task names the exact file and its current declaration/use mismatch — cited.
- **Would collide with:** runtime-path changes in public_surface.rs; future additions of public legacy/V2 types require re-running this guard — cited.
```

Acceptance: an unused pub struct turns the actual workspace guard red; exercise pub enum, pub trait and pub type too so the entire stated declaration family is covered. Positive controls cover Type::, &Type, : Type, Type { and -> Type with identifier boundaries. A comment-only name, a longer identifier containing the name and restricted visibility must not satisfy or invent public use. Keep the existing stricter function/constant criterion; do not solve unused current types with comments, an allowlist or weaker function matching. Current workspace green is necessary but unexecuted by this scoping report; actual untested types may require real usage cases, at which point name the additional test paths before expanding worker scope.

Minimal checks: `cargo test -p ekr --test public_surface`, focused scanner controls and the injected-unused-type mutation. No product source change is implied.

## README membership task

Task: `task:readme-status-case`.

Still active. The case computes Cargo crate members and only rejects three phrases; missing Status passes. Current Status already names all six crates and xtask, so membership equality can pass without changing the README. Separately, it calls every member empty and says no runtime logic exists; that is demonstrably stale after seed/membrane work and should receive a truthful coordinator-owned wording update. Membership validation alone cannot substantiate implementation-progress claims.

```markdown
## Scope

- **Primary surface:** crates/ekr/tests/adversary_docs_contract.rs — cited.
- **Symbols:** crate_members and the_readme_status_matches_the_workspace_members — cited.
- **Read inputs:** Cargo.toml and README.md Status — cited.
- **Also likely:** README.md Status wording, if the coordinator includes the observed stale bootstrap claim — inferred.
- **Confidence:** high, the current case and Status section were both read — cited.
- **Would collide with:** runtime-path changes in adversary_docs_contract.rs and concurrent README status work — cited.
```

Acceptance: missing Status, a missing actual crate and an unknown claimed crate fail; the current declared membership passes. Compare the Status section, not incidental names elsewhere in README. Clarify whether the exact comparison is six runtime crates or all seven Cargo members including xtask; the existing helper deliberately excludes xtask, while current Status includes it. Recommended implementation recognizes all actual member package names including xtask and excludes unrelated prose/backticks. Synthetic parser controls are sufficient without rewriting README during tests. Minimal check: `cargo test -p ekr --test adversary_docs_contract`.

## Trybuild diagnostic task: reconcile before choosing additional work

Task: `task:trybuild-stderr-is-toolchain-pinned`.

Its no-toolchain-pin premise is stale: `rust-toolchain.toml` pins 1.98.1 with clippy/rustfmt, and `.github/workflows/correctness.yml:34` uses that same version. This meets the task's original exact-pin alternative. Its later amendment remains relevant: a new trait impl changes rustc's incidental implementor help under the same compiler. The reviewed legacy unit's two E0277 refreshes are current evidence of that narrower problem.

Existing targets are `crates/ekr-graph/tests/membrane.rs`, `crates/ekr-graph/tests/review_p1_membrane_is_by_id.rs`, and `crates/ekr-kernel/tests/validation.rs`, loading four graph compile_fail snapshots, one review_p1_compile_fail snapshot and two kernel snapshots. No robust diagnostic harness is implemented or required by this report.

```markdown
## Scope

- **Primary surface:** rust-toolchain.toml and .github/workflows/correctness.yml, already aligned — cited.
- **Read inputs:** crates/ekr-graph/tests/compile_fail, crates/ekr-graph/tests/review_p1_compile_fail and crates/ekr-kernel/tests/compile_fail — cited.
- **Also likely:** AGENTS.md, a narrow documented review/refresh policy if selected — inferred.
- **Alternative:** the current graph/kernel trybuild wrappers and a new structured-diagnostic matcher if the coordinator selects that larger solution — inferred.
- **Confidence:** high for the stale pin premise and existing snapshots; medium for the still-unselected closure policy — inferred.
- **Would collide with:** the reviewed legacy snapshot refreshes and future V2 type/trait changes; a documentation-only policy has no source collision — cited.
```

Recommendation: reconcile the original pin claim now and explicitly choose whether the residual same-compiler churn is closed by a reviewed refresh policy or remains debt. A policy should allow refresh only after checking the same forbidden operation, error code and diagnostic span; incidental help-list changes must not mask successful compilation or an unrelated compiler error. If chosen, this is a small documentation addition in the same guard unit, with existing compile-fail targets confirming current snapshots. Do not silently replace trybuild with a code-only matcher: an unrelated E0277 can satisfy such a matcher. A stronger custom harness is a separate implementation choice, not a one-file certainty.

## Store ambient-runtime task: separate runtime source ownership

Task: `task:ekr-store-block-on-cannot-nest`.

Still active by source inspection, not a newly executed panic probe. The bridge owns Runtime and invokes block_on at eventlog.rs lines 151, 168, 262, 404, 476, 528 and 649. Constructors, reads, append/retention and atomic initialize all participate. A constructor-only check leaves pre-opened stores panicking when a method is called from an entered runtime. FileStore::file also creates the directory before constructing its runtime, so ordering matters for an early refusal.

```markdown
## Scope

- **Primary surface:** crates/ekr-store/src/eventlog.rs, runtime creation and every bridge entry — cited.
- **Files:** crates/ekr-store/src/lib.rs, StoreError — cited.
- **Also likely:** crates/ekr-store/tests/runtime_context.rs, a new two-provider refusal/control target — inferred.
- **Read contract:** .engineering/planning/architecture-decision-record/0006-ekr-store-bridges-the-async-port.md — cited.
- **Symbols:** new_runtime, EventlogStore::sqlite/file, Runtime::block_on, StoreError and the synchronous RevisionLog/ObjectStore/Initialize implementations — cited.
- **Confidence:** high for current source reach; medium for full ambient-runtime lifecycle until executed — inferred.
- **Would collide with:** atomic blob adoption and V2 changes in eventlog.rs; store/lib.rs error additions; provider tests — cited.
```

Smallest approved option is named refusal using Handle::try_current, retaining the synchronous contract. Cover constructors before their side effects and operations on an already-open store before creating/entering another Runtime. Runtime ownership/drop inside async context also needs a bounded lifecycle check before claiming that *all* ambient use is panic-free; merely guarding block_on does not establish that broader claim. Scope its outcome explicitly rather than inventing an async trait rewrite.

Acceptance: both constructors return the exact named error inside an entered runtime without unwinding; a pre-opened store's representative read/write/initialize routes refuse consistently and do not append; the same operations work normally outside that context. Keep store lifetimes explicit in the test so destructor behavior is visible rather than accidentally hidden. Run store package tests and kernel seed acceptance after implementation. Schedule after/beside a genuinely separate source owner, not concurrently in eventlog.rs. No existing active story claims this task specifically.

## ESS byte task: keep with persisted activation

Task: `task:ess-has-no-byte-string-type`.

The task already records upstream Primitive::Bytes support at ESS a5f1bea13294510819b266561c83be9509e6ba57. This report relies on that recorded source verification; it does not claim an independently executed EKR projection. The store YAML still says ESS has no byte string and declares ObjectStored.bytes as List<Integer>. Actual ObjectRecord still embeds Vec<u8>. The current projection guard compares event names and field names, **not field types or byte encoding**; it does not pin List<Integer> as an executable assertion.

The accepted persisted story now says payloads live behind atomic provider blobs and event bodies contain metadata/addresses. A unilateral YAML Bytes/base64 substitution would both change persisted JSON and contradict the pending blob cutover. The representation choice is no longer an independent one-file cleanup.

```markdown
## Scope

- **Primary surface:** systems/ekr/domains/store.yaml ObjectStored contract — cited.
- **Files:** crates/ekr-store/src/eventlog.rs ObjectRecord and publication, crates/ekr-store/tests/domain_projection.rs event/body agreement — cited.
- **Existing owner:** story:version-persisted-contracts activation, with coordinator-owned ESS changes and atomic provider capability dependency — cited.
- **Documents:** task:ess-has-no-byte-string-type's stale premise and selected replacement record contract, coordinator-owned — cited.
- **Also likely:** typed projection tests for the new metadata/blob reference contract; the exact filename depends on the adopted activation shape — inferred.
- **Confidence:** high for current record mismatch and ownership collision; medium for the not-yet-activated replacement encoding — inferred.
- **Would collide with:** ObjectStored V2/blob publication, frozen legacy decoding/vectors, and store.yaml/domain_projection.rs changes — cited.
```

Acceptance belongs to the coordinated activation: the chosen typed schema/projected JSON agrees with actual new stored metadata, raw payload lives in the provider blob store, legacy inline records retain original bytes/hash verification, and no failed seed leaves durable staged blobs/events. The present original inline record must not be rewritten just to use Bytes. Primitive capability alone does not close the task. No new upstream ESS scalar story is needed from the evidence currently recorded.

## Proposed scope commands (not executed)

AEP scope accepts a **story**, not these task ids. There is no current guard story to substitute honestly, so the guard commands below become executable only after the coordinator supplies its selected existing/new dispatch owner in `GUARD_STORY`; this report creates none. Historical implemented origins are not automatic owners. Entries are cited unless explicitly marked inferred.

```sh
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-core/tests/adversary2_public_surface.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-core/tests/identity_serde.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-core/tests/public_surface.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/adversary2_membrane_and_addresses.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/adversary_canonical_value_reach.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/canonical_value_and_assertion.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/domain_projection.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-graph/tests/revision_events.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-ontology/tests/domain_projection.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-store/tests/adversary2_retention_event_contract.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr-store/tests/domain_projection.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr/tests/adversary_docs_contract.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr/tests/msrv_contract.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr/tests/public_surface.rs'
aep plan artifact scope "$GUARD_STORY" --add 'xtask/src/main.rs'
aep plan artifact scope "$GUARD_STORY" --add 'crates/ekr/tests/temporal_reads.rs' --inferred
aep plan artifact scope "$GUARD_STORY" --add 'AGENTS.md'
# Optional truthful Status correction, if included:
aep plan artifact scope "$GUARD_STORY" --add 'README.md' --inferred
# Optional diagnostic refresh policy shares AGENTS.md; do not add snapshot writes by default.

# Runtime refusal requires a separately selected RUNTIME_STORY and a free source owner:
aep plan artifact scope "$RUNTIME_STORY" --add 'crates/ekr-store/src/eventlog.rs'
aep plan artifact scope "$RUNTIME_STORY" --add 'crates/ekr-store/src/lib.rs'
aep plan artifact scope "$RUNTIME_STORY" --add 'crates/ekr-store/tests/runtime_context.rs' --inferred

# Existing persisted owner; declare exact activation/test overlap before dispatch:
aep plan artifact scope story:version-persisted-contracts --add 'systems/ekr/domains/store.yaml'
aep plan artifact scope story:version-persisted-contracts --add 'crates/ekr-store/src/eventlog.rs'
aep plan artifact scope story:version-persisted-contracts --add 'crates/ekr-store/tests/domain_projection.rs'
```

Read-only inputs such as Cargo.toml, already pinned toolchain/workflow and compile-fail snapshots are deliberately not added as writable scope where no edit is proposed. Planning updates remain coordinator-only. Broad existing directory entries do not prove disjointness: the AEP help explicitly states paths are not normalized to infer directory/file containment, so inspect these exact overlaps manually.

## Unestablished items and check boundary

- No tests or mutation witnesses ran during this read-only preparation; implementation acceptance remains unexecuted.
- No unimplemented story currently owns the routine guard tasks or runtime-refusal task; dispatch identities remain coordinator decisions.
- Public-type scanning may reveal current unexercised types; exact additional behavioral test paths cannot be known without the new guard.
- Standalone xtask invocation without Cargo environment needs an explicit fallback/refusal choice.
- The README membership parser must state whether it validates all Cargo members or its existing six-crate subset.
- The residual diagnostic-churn task needs an explicit closure disposition despite the existing exact compiler pin.
- Ambient Runtime destruction behavior has not been executed here; do not infer whole-lifecycle safety from guarded block_on sites.
- New ObjectStored/blob wire shape and its executable typed projection remain part of V2 activation, not a completed byte-type fix.
- The reviewed legacy merge landed while this report was prepared; begin from that integrated source and preserve its current/frozen-family guard and refreshed snapshots.

For the combined routine unit, add fail-first guard mutations, run affected core/ontology/graph/store/ekr test targets and xtask doctor, then the repository's required fmt/clippy/test/doc/planning gate at integration. No new compiler run was launched by this scoping work. The runtime and typed-ESS units retain their own later acceptance and cannot be marked complete by a source-guard green gate.

