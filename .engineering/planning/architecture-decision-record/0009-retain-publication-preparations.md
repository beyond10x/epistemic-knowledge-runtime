---
format: aep.planning-md/1
id: architecture-decision-record:0009-retain-publication-preparations
kind: architecture-decision-record
status: accepted
title: ADR 0009 — Retain exact publication preparations through uncertain outcomes
relations:
- decides: story:commit-and-revision-lineage
- decides: story:version-persisted-contracts
revision: 2
---
## Status

Selected during the authorized durable writer implementation after an executable
unknown-outcome failure and an independent architectural review. This decision
adds private recovery persistence; it does not declare the writer implemented.

## Evidence

The implementor's retained application-red-unknown report shows
unresolved_publication_cannot_be_replaced_by_a_new_occurrence failing: a later
Commit loses the unresolved occurrence and samples replacement IDs and time.
This is a kernel-port fault wrapper around real SQLite, not a native crash test.

The independent publication recovery design review confirms that retaining only
Publication also loses the exact native object appends, expectations, metadata
and attempt key. Eventlog's atomic-content contract requires retry of the
original request; its current port provides no separate complete receipt lookup
or absence fence that recovers a lost request from an empty history read.

## Decision

Implement DESIGN 94 with the strict internal carriers declared in
systems/ekr/domains/store.yaml. Elect one immutable preparation in an Eventlog
conditional slot for the logical command, retaining its complete decision and
exact native request through atomic private blob/metadata publication. Bootstrap
uses one slot per tenant lineage; transaction command slots bind the retained
predecessor, and the selected record separately binds actual input/host authority.

Unknown preparation authorizes no domain publication. Competing candidates must
win or adopt the same slot; only unelected candidates may discard sampled IDs and
time. An elected unknown native attempt is retried exactly before staleness or
new attempt construction. Only a definite conflict from that exact attempt
permits a conditional successor. Unknown publication never becomes a fabricated
Stale outcome or an invented transaction state.

Preparation is private recovery data, outside canonical object coordinates and
with no canonical authority. Keep all ordinary receipt/revision/seed versions,
real kernel replay checks, and the canonical atomic publication guarantee.
Missing/corrupt preparation refuses; ordinary read commands do not recover by
writing. No new storage engine, filesystem sidecar or Entity Runtime dependency.

## Alternatives

An in-memory retry cache cannot survive restart. Rebuilding Publication changes
the provider fingerprint as stream expectations or object appends move. A new
occurrence after an empty history read cannot rule out the old in-flight write.
Making times or IDs derive from caller input would change trusted time/identity
semantics and still would not retain the native request. A retry-token-only API
would not satisfy the existing Commit(transaction_id) restart contract.

## Verification and ownership

The original red remains an unfiltered writer acceptance. DESIGN 94 lists the
new concurrency, restart, corruption, exact-request and no-losing-binding controls
on both providers and all decision kinds; they remain unexecuted at declaration.
The later full gate and independent source review remain required.

Owners: implementor for the measured preparation loss and source repair;
coordinator for the missing recovery contract, shared declarations and integration;
independent reviewer for its architectural recommendation and stated limits.
