# Coordinated durable-kernel activation

This source unit follows the bounded transaction parser's second independent
review, complete integration gate and merged PR #11. Its resource and dispatch
record is p1-durable-activation-20260922.md. It continues the operator-approved
completion plan; source work begins under the coordinated ownership below.

## One source owner

Use one isolated worktree and one implementor for the coupled activation surfaces
of story:version-persisted-contracts and story:commit-and-revision-lineage. Their
scope overlaps in graph, transaction, seed, store and replay types. Do not schedule
two workers on those surfaces. This is a coordination exception to one worker per
story: the stories retain separate acceptance, while the source must agree in one
candidate. Root alone owns planning, shared specifications and publication.

The older request for a complete format gate before writer dispatch is superseded
for this coordinated activation. The source inspection in
.engineering/reviews/p1-writer-readiness.md and the complete declaration patch
require current graph/seed codecs, receipt records, atomic publication and kernel
replay to agree. A passing old seed-only writer cannot establish that contract.
Review the format/seed milestone before completing the application milestone;
the final integrated gate and independent adversary still precede publication.
No temporary permissive authority, inline-payload fallback or altered legacy
decoder may bridge the implementation stages.

## Authoritative inputs

Read AGENTS.md and COORDINATOR.md first. Root must apply the exact released-compiler
qualified p1-writer-activation-ess7-draft.patch to the coordinator and unit together
before implementation. Its dated DESIGN additions and ESS are the activated
contract. Root checks both copies and records the opening revision and resource
paths in the wave page. The released compiler preflight is
.engineering/reviews/p1-writer-released-compiler-preflight.md; its generated suite
is compiler evidence only.

Read p1-durable-record-decisions.md, p1-writer-readiness.md and
p1-writer-activation-corrected-acceptance.md. Their older compiler-gap passages
are historical: the verified release closes that capability gap. Their runtime,
preservation and test obligations still apply. Prefer the final dated DESIGN/ESS
over earlier draft spelling, including event/2 rather than proposed event/3.

## First milestone: coherent formats and seed publication

Implement the current graph/seed/envelope versions, independent assertion
assessment and lifecycle, ordered outer property values, reasoned retraction,
supersession, occurrence identity and complete record codecs. Preserve every
original-format codec and immutable vector; changing a current encoder cannot
re-certify a historical address. Implement complete ontology export/encoding and
the host-supplied authority/profile representation and roots.

Adopt the pinned provider's atomic blob/event capability in the existing store.
ObjectStored/schema2 contains metadata only; ObjectRetentionRaised stays schema1.
Explicit version dispatch retains original inline schema1 for verification and
migration only. Verify the actual blob bindings, length, addresses and retention.
No second storage implementation or put-then-append sequence.

Kernel seed admission must validate full graph/ontology/evidence and actual
bootstrap context under the exact profile, atomically publish the complete
Seeded occurrence and return its own retained result. Preserve the no-losing-
object/concurrent seed guarantees. Identical parsed seed/context/anchor retries
return the original result; a different one refuses without writes.

Update current projection and fixture checks to the new contract using the
explicit case mapping in the acceptance document. Keep historical fixture data
unchanged. Preserve kernel-only writer ownership, canonical/transient typing and
the synchronous-runtime refusal. Hand back a reviewable milestone with source
diff and actual test statuses; do not label unsupported stages green.

## Second milestone: real durable decisions and application

Use the shared exact-byte parser for actual Propose. Retain the original document,
trusted submitter/time and all derived facts. Semantically invalid but structurally
valid input can reach a durable refusal; malformed documents create no proposal.
No JSON round trip or fabricated canonical Float hash.

Validate against a complete retained revision basis. Persist the actual validator
set, profile and canonical validation material. An absent requested revision
returns RevisionNotFound without inventing a basis or a Rejected record. Commit
consumes the real private capability and applies the admitted unordered set using
the same candidate semantics as validation. All admitted operations must apply;
schema evolution and entity merge retain their explicit P1 refusals.

Use fallible real kernel authority for live publication and replay. Retain all
Proposed, Validated, Committed, Rejected and Stale states and strict record links.
Reopen reparses, revalidates, reapplies and recomputes every root field. Missing
or corrupt required history refuses; it cannot silently return the seed head.
Historical reconstruction stops at the requested committed revision.

Resolve occurrence retry before staleness, including lost-response retry after
restart and later head advancement. Canonical-head movement differs from provider
stream movement. Preserve immutable occurrence IDs and trusted times through
conditional publication retries and unknown outcomes. Never clean up on an
unknown commit result by deleting data.

The first decisive witness is the already specified fresh-process test on both
real providers: seed, create a node and evidence-backed assertion, preserve Many
and single List properties, validate, apply, persist, restart and query changed
knowledge through actual kernel authority. It is a milestone, not the whole exit.
Then complete lifecycle/history, time, root sensitivity, terminal-state retention,
contention/interruption, corrupted-record and exact-result controls from the
existing acceptance. CLI and conformance will consume these same handlers.

## Scope, checks and handback

The recorded story scopes and inspected dependency table define source ownership.
Report any additional path before editing it. Root supplies shared DESIGN/ESS and
planning; the unit must not edit them, Cargo pins or the already reviewed parser
profile. Shape changes to the shared GraphOperation require the parser's final
retraction/supersession integration tests, not a private operation enum.

Start with failing behavior witnesses. Preserve original adversary assertions;
mechanical fixture changes must retain their reason for failure and positive
controls. New dependencies, reduced acceptance or changed frozen semantics are
not routine implementation choices. Report concrete contradictions with evidence.

Use the assigned isolated target, bounded compiler settings and disk floor.
Run touched checks while developing; root captures every final gate step and its
own exit status. Retain logs and exact failed inputs outside public source.
Leave a clean handback for independent attack before integration/publication.
This unit cannot claim complete migration, later phases or live cutover.

## Dispatch qualifications

Eventlog PR #15 is merged with required production, comparative and restart CI
green. The consumer selects its exact reviewed source, as recorded in
.engineering/reviews/eventlog-repair-adoption.md; the full consumer gate and
direct old-writer/repaired-reader proof passed. Use the portable
AtomicBlobEventStore trait explicitly: the File inherent method of the same name
has a different contract. Preserve UnknownCommit classification before flattening
provider errors. No new tag is needed for this already published source.

The two stories form the explicitly approved single source unit. The format
story's participation here is the exception to the generic implementor charter's
single-artifact/predecessor rule; it is not an unlanded external dependency to
silently replace. Every internal child finding remains acceptance for this unit.
The repository's assigned external target likewise overrides that generic
charter's in-tree target rule. No extra implementation owner is admitted.

First send a coherent format/seed checkpoint with the actual diff, original
failure evidence and scoped command statuses. Freeze source while the coordinator
reads that checkpoint, then continue the application milestone on acknowledgement.
Do not infer a passing application/replay result from that checkpoint.
