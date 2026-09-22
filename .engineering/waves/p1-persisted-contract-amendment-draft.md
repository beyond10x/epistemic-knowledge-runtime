# Amendments — 2026-09-22

# 88. Assertion Assessment, Lifecycle and Historical Reads

*Resolves the overlap between §§17 and 36, and makes §65's two time dimensions explicit.
Implementation is unexecuted at this amendment's preparation.*

An assertion stores its validation assessment and lifecycle independently. Validation retains
Proposed, Validating, Accepted, Rejected and Disputed, with their existing payloads. Accepted
retains the exact accepting validator set. Lifecycle is stored, not derived from validation:

- Active: no withdrawal or replacement is recorded.
- Retracted: the committing revision and stated reason.
- Superseded: the replacing assertion, committing revision and effective valid-time boundary.

Lifecycle changes preserve the assessment, evidence identities and proposer. Neither Active nor
Superseded grants acceptance. A fresh proposal supplies Proposed assessment and Active lifecycle;
only kernel validation establishes acceptance, and only a validated lifecycle operation changes
lifecycle. The general assessment policy remains distinct from this storage shape.

A retraction says the runtime no longer endorses the claim. At that revision and later,
valid-time reads exclude it at every time. A supersession records a supported fact's replacement:
the earlier claim remains answerable within its closed valid-time interval, provided its retained
assessment is Accepted. The replacement must itself be Accepted in the validated candidate,
have a distinct assertion identity, begin at the supplied boundary, and satisfy the candidate
valid_at eligibility predicate at that boundary. An already retracted or otherwise unreadable
replacement refuses even though it retains Accepted assessment. Subject and predicate
equality is not required: the §65 example replaces assertions about different subjects.
Normal ontology, provenance and cardinality checks still apply. The former interval
ends at that boundary; intervals are half-open. The boundary cannot precede the former interval's
start or extend a finite existing end. A self-reference or supersession cycle is refused.

For §65, a latest-revision query before the handover returns the earlier relationship; one at
or after the handover returns its replacement, subject to the replacement's own valid interval.
These are both valid-time reads of one revision. Reconstruction at an earlier committed revision
answers what that revision believed, before later retraction or supersession. Transaction-time
recording does not replace the effective valid time.

Ordinary transaction publication receives its timestamp from execution context and persists it
in the commit receipt. Newly accepted assertions have recorded_from set to that timestamp and
recorded_to unset. Retraction and supersession close the former record's recorded_to at that
timestamp, preserving recorded_from. A timestamp preceding the latest committed timestamp or
an affected record's recorded_from refuses; replay uses the recorded timestamp, never its clock.
Supersession's replacement receives the same recording timestamp when newly added; an already
accepted replacement keeps its recorded_from. Retraction leaves the historical valid interval
unchanged; supersession closes it at the supplied effective boundary.

At a selected revision, valid_at(t) includes exactly assertions with Accepted assessment, a
valid-time interval containing t, and either (Active lifecycle and open transaction time) or
Superseded lifecycle. Retracted assertions never qualify. Closing transaction time on a
Superseded record does not suppress its supported historical valid interval. Revision selection
supplies the transaction-history axis; valid_at does not guess a wall-clock instant.

AddAssertion and explicit supersession may occur in one unordered atomic transaction. Fresh
assertions arrive Proposed; assess replacement acceptance against the fully validated candidate,
not the pre-transaction graph. The replacement must survive and be readable at the boundary in
the candidate. Conflicting lifecycle
changes, retraction of that replacement, self-supersession and cycles refuse. These operations
are implemented by the writer; the format unit defines the retained state and read semantics.

This changes new-format assertion canonical encoding: assessment then lifecycle are both encoded
in declaration order, and either payload affects the hash. Preserve frozen old encoders for
legacy verification. Never reinterpret old Retracted or Superseded assessment variants by
inventing an accepting validator set. Prior acceptance must be recovered from independently
verifiable retained history or migration refuses with the affected assertion identity.

# 89. Revision Occurrences and Persisted Format Dispatch

*Extends §§34, 56 and 57. Independent equal-content decisions are distinct facts; retrying a
decision does not produce another fact. Implementation is unexecuted at preparation.*

Every new revision-log fact is enclosed in a strict versioned record with an EventId:
format "ekr.revision-event/2", event_id, payload. EventId is a UUID-backed stable identity,
allocated once by the kernel for the occurrence. The payload keeps the existing six event names
and variant indices. Include the envelope format and occurrence identity in canonical encoding.
Store the record with backend schema version 2; preserve backend event identity, stream position,
event name and schema version when reading it.

A new proposal, validation, refusal or stale decision receives a new occurrence identity even
when its payload repeats earlier content. A retry uses the same identity and exact payload.
Use occurrence identity for idempotency and complete record bytes for request equality. Reusing
an identity for different content refuses. Never generate a new identity inside a retrying append,
and never derive identity from content or the stream position about to be written.

Canonical validation addresses use ContentHash::of over the exact canonical transaction and
explicit validation basis, never ContentHash::of_bytes over encoded canonical values. Preserve the
existing payload/value hash labels. Version dispatch distinguishes the historical payload-domain
scheme from the corrected value-domain scheme; it never silently verifies an old hash with new
rules. The durable writer additionally binds acceptance to the full prior root and retained
validator policy; a revision number by itself does not prove equivalent state in different stores.
Its durable receipt declares its basis format and must be specified before first publication.

New assertion shape requires explicit graph-document format dispatch. The compatibility table is:

| input | graph representation | admission |
|---|---|---|
| unversioned GraphDocument | original fields and assessment enum | legacy inventory and original hash verification only |
| ekr-seed/1 | original GraphDocument in graph | legacy inventory and verified migration only |
| ekr-seed-envelope/1 | input is ekr-seed/1; retained bootstrap context | legacy inventory and verified migration only |
| ekr.graph-document/2 | graph contains the new assertion assessment and lifecycle | new-format graph decoder |
| ekr-seed/2 | graph is an ekr.graph-document/2 envelope; ontology and evidence_payloads remain explicit | new kernel seed admission |
| ekr-seed-envelope/2 | input is ekr-seed/2; context retains operator and validator | new kernel seed replay admission |

The graph envelope is {format: "ekr.graph-document/2", graph: <graph fields>}. A seed's graph
field holds that complete envelope, not its inner graph. A persisted seed envelope holds the
complete versioned seed input in input. Every layer is strict. Unknown versions, mismatched
nesting, new graph shape under an old seed tag and unknown semantic fields refuse. Preserve
user-defined record keys as data. Successful conversion of a verifiable seed/1 is a migration
with an explicit address map, never normal ingestion under new defaults.

# 90. Preservation-First Store Migration

*Extends §§34, 52 and 57. Migration preserves evidence; it cannot manufacture omitted history.
Implementation is unexecuted at preparation.*

Inventory and verify the source without changing it. Retain original object bytes, event records,
backend coordinates and hashes. Decode each source version with its matching original canonical
rules. Convert into a separate destination only after required input can be verified. Preserve
stable domain object identities and record old-to-new object and revision addresses.

Freeze destination event occurrence identities in a migration manifest before writing, binding
each to a source event coordinate. Retry an interrupted conversion from that same manifest.
Events absent through old content deduplication cannot be reconstructed from a later state.
A hash is not its operation payload, and a retracted legacy record is not its prior acceptance.

If operation payloads, governing ontology, validation attribution, prior acceptance or retained
evidence cannot be verified, refuse with the exact missing evidence and affected record. Preserve
the source; do not reset its lineage, fill in synthetic metadata, or treat a seed snapshot as
lost history. New-format admission is not an implicit conversion path.

Publish or switch to a destination only after complete mapped-history verification and restart
equivalence through the real kernel authority. Unsupported conversion may finish with a named
refusal and unchanged source; it must not be reported as a successful migration.
