Public copy: private checkout paths in the original review are elided below; its technical text and ownership are otherwise unchanged.

# Publication recovery design review

The following preserves the architectural review returned to the coordinator. It is an intermediate, source-based review; it is not final adversary approval.

## Original review, verbatim

Use a private, versioned preparation journal. The current provider port cannot resolve a lost request from its key alone, and an empty read does not fence an in-flight publication.

The observed failure follows directly from `commands.rs:94–123`: occurrence, revision ID, timestamp and prepared result are local variables. There is a second recovery gap in `eventlog.rs:461–506`: restarting `publish` rebuilds object appends and resets the attempt counter. Retaining `Publication` alone therefore does **not** retain the exact native request.

Recommended minimum protocol:

1. **Elect one immutable preparation per logical command.** Use a dedicated stream with `Expected::NoStream`, subsequently `Exact(version)`. Store its complete decision and initial native request through one atomic blob group. Commit selection binds transaction ID and retained validation identity. Seed needs **one bootstrap slot per lineage**, with parsed input/context/authority equality inside the record; seed-hash-only slots permit competing preparations.
2. **Publish only the elected preparation.** An acknowledged preparation, or a fully read and verified winning record, permits canonical publication. If preparation returns UnknownCommit, a successful empty read permits neither canonical publication nor declaring absence. Competing preparations must contend for the same conditional slot and adopt its winner.
3. **Persist exact native attempts.** Retain ordered appends, expectations, every metadata field, blob keys/bytes, attempt key and the logical decision. Resume the unresolved attempt before checking staleness or rebuilding object appends. Provider fingerprints include expectations and metadata (`eventlog-core/src/atomic_group.rs:52–77`).
4. **Separate conflict from uncertainty.** UnknownCommit retains the same attempt. After an exact-request retry definitively excludes its publication, a conditional journal successor can retain a new native attempt with the same domain occurrence/time, or a distinct Stale decision. A generic history-length mismatch is not that proof.
5. **Keep preparation outside canonical visibility.** Private preparation blobs must use separate coordinates, with no canonical ObjectStored event or public object binding. The preparation blob and selection record must share the CAS transaction, so losing preparations leave no committed binding. Missing/corrupt preparation data must refuse recovery, never regenerate it.

This breaks the journal recursion: an unresolved **preparation** has not authorized canonical publication. Only its eventual elected winner can do so. One contract clarification is necessary: losing, unelected candidates may sample IDs/time that are discarded. Once elected, the decision is immutable. If even unelected candidates must retain the same sampled values across crashes, the existing APIs are insufficient.

Concrete counterexamples to guard against:

- Preparation P is delayed; restart reads empty and prepares Q. Both must contend for one slot, and neither may publish its local candidate without winning it.
- Canonical attempt A remains unresolved while an unrelated object stream advances. Rebuilding A with the same key changes its fingerprint.
- Another canonical commit advances the head while A is unresolved. Recording Stale before resolving A can contradict a successful earlier commit.
- An exact retry follows erasure. Recovery must return the retained outcome without recreating erased bindings.
- Two bootstrap callers supply different seeds. Separate hash-selected preparation slots do not establish one bootstrap decision.

Required acceptance should inject crashes/unknown outcomes before and after preparation acknowledgement, canonical commit, and journal resolution; exercise concurrent preparers; and prove restart preserves exact native requests, actors, occurrence IDs, revision IDs and timestamps. Include request mismatch, missing journal bytes, head advancement and zero losing public objects.

DESIGN §91.6 needs the preparation/attempt distinction, selection keys, private retention boundary and definitive-resolution rule. ESS needs typed internal preparation/attempt state and explicit unresolved recovery outcomes. Domain transaction states should remain unchanged until canonical publication.

Provider basis: `atomic-blob-append.md:34–67` requires original-request receipt resolution and prohibits resurrection. No existing receipt lookup or absence fence inspected here removes the need to retain that request. This is a source-based recommendation; no builds, probes or edits were performed.

## Command-binding refinement

The coordinator's refinement is sound, with one terminology clarification below. Selection must serialize contenders for the same logical operation; it must not include mutable/differing request fields in a way that creates parallel slots:

| Command | Retained request binding | Selection scope |
|---|---|---|
| Seed | Complete parsed Seed input, actual BootstrapContext and trusted authority anchor | One bootstrap slot per lineage; different parsed hashes do not create separate bootstrap slots |
| Propose | Transaction identity, exact document bytes and authenticated submitter | One proposal slot for that transaction identity |
| Validate | Transaction identity, retained proposed occurrence, requested basis and actual validator | One validation-decision slot for that proposed occurrence; changing basis or validator does not create another slot |
| Commit | Transaction identity, retained validation-record identity and actual committer | One commit-decision slot for that validation record; changing committer does not create another slot |

“Adopt its winner” in the original review means resume the winner only after matching the bound command inputs and trusted execution context. A pending winner with different inputs returns an operational publication conflict/uncertainty. It does not manufacture TransactionStateConflict, AlreadySeeded, a terminal rejection or a new domain fact before the corresponding domain publication exists.

An elected preparation alone is not success. The phrase “known canonical success” should mean **confirmed committed publication of the corresponding decision**: Proposed, Validated, Rejected and Stale do not themselves create a canonical revision. Exact recovery can return the retained outcome only when that decision's publication is established, never merely because its preparation is retained. This preserves the existing public command state rules after resolution; it does not introduce blanket successful retries for wrong transaction states.

Recovery applies to all six publication kinds: Seeded, TransactionProposed, TransactionValidated, TransactionRejected, TransactionStale and RevisionCommitted. In particular, IssueIds and rejection time must survive recovery just as commit occurrence/revision IDs and time do. Validate selects one validation-or-rejection decision; Commit's distinct Stale successor is permitted only after resolving the earlier attempt as described above. This refinement introduces no separate provider capability claim.

## Exact inspected source pointers

EKR active writer tree:
`<worktrees>/ekr-durable-activation-20260922`.

- `crates/ekr-kernel/src/commands.rs:52–89`: proposal and validation/rejection decisions allocate local occurrence/time and return on unresolved store errors.
- `crates/ekr-kernel/src/commands.rs:94–125`: retained success lookup, local commit preparation, conflict/stale handling.
- `crates/ekr-kernel/src/commit.rs:181–206,208–287`: retained seed comparison and local seed preparation/publication.
- `crates/ekr-store/src/eventlog.rs:373–391`: bounded exact native retries, followed by UnknownCommit.
- `crates/ekr-store/src/eventlog.rs:409–509`: occurrence scan, history-length check, rebuilt object appends and loop-local attempt key.
- `crates/ekr-store/src/log.rs:108–148`: Publication stores the domain event/objects/expected revision position, not the exact native BlobAppendGroup.
- `docs/epistemic-knowledge-runtime-design.md:3882–3913`: §91.6 retained times, occurrence identity, conflict reconciliation and atomic publication.
- `crates/ekr-kernel/tests/durable_commands.rs:156–165`: implementor-authored unresolved-publication regression; its red log was inspected, not executed by this review.

Exact provider pin:
`28e578568846fc860e44a5f7c76b7e807abddc12`, as recorded by EKR Cargo.lock.

Inspected cached provider source:
`<cargo-cache>/eventlog/28e5785`.

- `docs/design/atomic-blob-append.md:27–67`: fingerprint, retained receipt, rollback, original request-key resolution and erasure behavior.
- `crates/eventlog-core/src/atomic_group.rs:32–79,108–123`: exact fingerprint inputs and public append-group port.
- `crates/eventlog-sqlite/src/atomic_group.rs:128–177`: receipt lookup before blob validation/publication.
- `crates/eventlog-postgres/src/atomic_group.rs:306–337,445–503`: native receipt lookup and unknown-outcome path.

The source was active during this read-only architectural task; these are the inspected locations, not a frozen-source handback manifest. No source edits, builds, AEP actions, lease operations, provider calls or new executable verification were performed.

Owners: the writer implementor owns the measured local-preparation failure and its eventual source repair. The coordinator owns the new versioned preparation/attempt contract, command-binding clarification, DESIGN/ESS amendments, integration and publication. This reviewer owns the architectural recommendation and its stated limits.

