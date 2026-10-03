# Dispute analysis and human-review foundations

This is a development checkpoint for `story:show-disputed-knowledge`, not inbox delivery.
The operator's accepted implementation plan authorizes this work. The exact current specification
and contract digests and measured test execution counts are in `verification.txt`.

`preview_contradictions` analyzes admitted graphs without mutation. It respects declared One
cardinality, inherited node and edge properties, relation targets, half-open valid times (including
integer endpoints), claim state, and equivalent admitted node-reference representations. Its
empty-result stub failed the positive cases before implementation; the red log is retained.

The human-review code operates on generated ESS semantic contracts. It verifies Ed25519 proofs,
scope, independent host/policy binding, audience, exact target/statement, and expected predecessor.
It derives operator attribution only from the policy and returns a capability callers cannot
deserialize. The initial permissive verifier failed its negative cases. Final tests include a
fixed binary-layout vector, decoding every decision target, all truncated prefixes, trailing bytes,
and flipping one bit at every retained-proof byte position. Product code has no signing path.

The object-retention test writes the proof, public policy and statement through both native
providers, closes and reopens them, checks both hash domains, and re-verifies from retained public
bytes plus the independent host binding. This is object/proof replay, not canonical authority
transition replay. Atomic enrollment/publication, decision deduplication and predecessor CAS,
pending-validation revalidation, assessments, attention commands, SDK and viewer remain unfinished.

Root-local review found that the draft ESS relations used plain review-protocol SHA-256 digests
as StoredObject addresses, which actually use EKR's payload-domain prefix. Explicit storage-address
fields now carry those relations. ESS validation and regeneration passed; generated files were
produced with the pinned development candidate, with its existing synthesis obligations/refusals
still visible in the generated plan. The public-surface guard required qualified calls and an
explicit verified-capability type annotation in the tests. The dependency guard required the
documented kernel `ring` edge. Neither guard was weakened. An earlier retention omission also
failed the CLI help inventory: observe/incubate now name their formats and input guidance, and
their required help rows are checked.

Owners: the contract mismatch and retention help omission were coordinator-owned; this was a
local review, not an independent adversarial review.

The kernel regression log predates the final decoder additions; final focused tests cover those
additions, and final CLI/conformance/architecture tests cover the combined source. Do not sum
overlapping execution counts into distinct test counts. The conformance directory holds a fresh
carrier and actual report/2 results for the existing retained-knowledge scenarios at the current
specification digest. Both loss mutants fail every selected scenario; unselected obligations are
not claimed conformant. Historical retention reports remain unchanged.

Workspace Clippy, member-only formatting, generated-tree byte drift and focused/cross-crate tests
passed. `task contracts-check` still refuses the unreleased generator pin, as intended. The full
`task check` and final adoption remain pending the approved ESS publication resolution; no ESS
ref, release policy or held-bundle ownership was changed. Raw logs stay in the task-owned cache;
these copies replace only the local home prefix for publication.
