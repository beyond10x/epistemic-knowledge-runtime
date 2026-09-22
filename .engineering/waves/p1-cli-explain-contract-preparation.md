# CLI and explanation contract preparation

This is an unapplied proposal. It does not change the active durable unit's
source contract. Reconcile it against that unit's public checkpoint before
activating it in every tree that must agree. The cited/inferred scope report is
`.engineering/reviews/p1-cli-explain-scope.md`; its packaging deviation and
remaining host-boundary questions remain visible there.

Partial adoption after the seed checkpoint: DESIGN 93 and active kernel ESS now
bind Propose and Validate's actual retained response records and the proposed
CommitCommandResult. The original draft patch is preserved as reviewed evidence;
do not apply it wholesale against the updated declarations. Snapshot/Explain
projections and exact host transport remain preparation. The bounded follow-up
review is .engineering/reviews/p1-cli-explain-contract-review.md.

## Proposed public results

The accompanying patch adds value projections, not new durable entities or
formats. Existing receipt, graph, assertion and evidence types remain authoritative.

- Commit's command result distinguishes Committed(CommitReceiptV1) from
  Stale(StaleRecordV1). Exact retained commit success keeps the same Committed
  result and original receipt. A stale outcome cannot fabricate a commit receipt.
- Snapshot accepts an optional valid_at Timestamp alongside optional revision at.
  Its result carries the selected revision identity, complete root and unfiltered
  graph document. With valid_at, matching_assertions is the ordered identity list
  produced by the graph's shared valid_at read; an empty match is an empty list.
  Without a valid-time selector, both valid_at and matching_assertions are absent.
  It does not silently choose the wall clock or pretend a filtered graph hashes
  to the complete revision root. Node and property reads remain available in the
  complete graph document.
- Explain returns a typed list of links over actual retained records. Order is
  assertion, origin admission, later lifecycle changes in revision order, then
  supporting evidence in stable identity order. A seed origin uses one Seed link;
  an ordinary origin uses Proposal, Validation and Commit links. Each later
  lifecycle change adds its exact receipt and resulting lifecycle. The Explained
  event's links count is the actual list length, not a guessed constant.

The kernel must verify every selected address and required payload before
returning Explain. A Seed link carries actual bootstrap context/profile and
original SeedResult; a Validation link carries the actual retained basis,
validator set and profile. An Evidence link carries its real source and content
address after payload verification. HumanStatement ends directly at that source;
P1 invents no observation, model interpretation or integration plan. Missing or
corrupt required history refuses instead of producing a convincing partial chain.
Lifecycle links preserve original acceptance rather than replacing origin links.

The proposed JSON presentation must retain typed integers exactly. These public
results are derived from the real handlers; conformance may not synthesize them
from expected outputs. Recorded validation rejection and commit staleness remain
outcomes, with their actual records/issues, distinct from domain refusals.

## Compiler qualification

Released compiler validation and compilation both exited zero. Synthesis counts
from the retained proposal output:
- generated: 35; authored: 0; refused: 0; outside: 0.
No scenario was executed. Keep all generated obligations; add authored witnesses
for complete read results, both valid-time selections at the same latest revision,
earlier-revision reconstruction, exact retained success and the real stale branch.
A generated suite with no refusals does not prove these return values are correct.

## Before source dispatch

The durable owner supplies a kernel-owned provider opener and real retained
handlers. CLI gains no raw-store dependency. Its host inputs must come from an
explicit trusted host configuration, separate from proposal/seed content; a freely
supplied agent ID alone is not authentication. The exact local authentication and
configuration transport remains to be settled against that facade. Clock sampling
must be lazy after retained-result lookup, while validators/replay receive fixed
recorded timestamps. No implicit unrestricted registry or validator is acceptable.

Document input uses the existing bounded stream reader directly; reading unlimited
bytes first defeats its ingress cap. CLI date syntax still needs an explicit
choice; core Timestamp is decimal milliseconds, and the original example's ISO
dates cannot be assumed to parse. Use explicit supersession for the replacement
example: retraction removes the former assertion at every valid time of the later
revision. Reserve operational faults for unavailable/corrupt state, and do not
fabricate a missing identity's prior state to fit a declared refusal.

Explain's added module export and dedicated test file are recorded before
implementation. Existing seed implementation ownership stays with the durable
unit. Schedule the dependent source after the durable handback, with measured
fixture/test ownership; separate story titles alone do not remove overlap.
