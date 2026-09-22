# Development adopter preflight

This is an unreleased development-binary measurement, not compiler adoption or
runtime conformance. The primary implementor supplied its completed CLI build;
root checked and retained the exact executable before the next correction.

Compiler SHA-256:
4daeaf104c082f3358f349aec54f020094d0ae42ded1424c8bd87e8721f79c44

The candidate passes specify validate and compile. Synthesis selects suite/13 but
exits1. The exact counts and refusal list are in development-summary.json,
derived from the original development-synthesis.json. No scenario was executed.
Both retained-result scenarios are generated with the actual origin identity:
Commit's transaction_id input and Seed's originally emitted revision_id. Queries
and snapshots include the complete Transactions and Revisions observations.
The generated retry steps contain no ConfigureExternalOutcome.

Two Stale-state refusal obligations cannot be emitted: Commit and Validate.
Their route enters Stale through the external Commit/stale branch of the same
state-guarded command. The diagnostic claims no input reaches that branch. The
original synthetic fixture's separate Stale command did not cover this adopter
shape. The source implementor is adding the matching fixture and correcting
bounded arrangement; the Go split is adding execution and negative controls.
The two obligations remain in the inventory and are not filtered or waived.

The exact original compiler output, statuses and executable remain in private
scratch. Verified released-compiler validation/synthesis and actual provider
acceptance are still required before production activation or phase completion.

## Corrected development compiler

The primary implementor reproduced the real external-Stale arrangement failure
in a finite fixture and corrected the source path. Root repeated the exact EKR
candidate with development CLI SHA-256
cee3d72257f6ac61f6f7d841e62fd3a4698cda82d4dc24d126cf57eb6a470a87.
Validate, compile and synthesis now exit0. The corrected suite retains both
original retry scenarios and now includes Stale-state refusal for both Commit
and Validate, with no refused or outside obligations. The original failed output
and original executable are preserved; corrected outputs use a separate
`development-corrected-` prefix. This corrects synthesis, not runtime delivery.
Measured corrected inventory: 33 generated, 0 authored, 0 outside, 0 refused; ess-conformance/13. Runtime EKR executions: none in this preflight.

## Complete source7 refusal observations

A later Go target control showed that ordinary Validate/wrong_state witnesses
could pass an undeclared direct event or subject mutation. The adopted ess/7
contract now requires complete held-subject preservation and zero direct events
for ordinary named wrong-state refusals, while keeping legacy witness bytes.
Root reran the unchanged EKR model with development CLI SHA-256
ea8a842f8700ee106ca4f3dd2a06e14390ba6877f2c5b57b835a612b1d98ab53.
Validate, compile and synthesis exit0. The actual Validate/Stale scenario now
snapshots Transactions before the command, asserts ExpectNoEvents, then queries
and compares the complete subject. Its exact output is retained separately under
`development-source7-`. This establishes generated obligations, not EKR execution.
Measured latest inventory: 33 generated, 0 authored, 0 outside, 0 refused; ess-conformance/13.

## Typed actual rows and reachable immediate retries

The independent compiler review showed that complete view declarations did not
ensure complete actual rows, and that a retry could be generated despite an
unreachable state guard. The corrected compiler now emits explicit complete
snapshot pairs with finite typed descriptors, and proves immediate retry
eligibility from the original input and post-command state. Root reran the
unchanged EKR draft; validation, compilation and synthesis all exit0. The outputs
are retained separately under the development-r1 prefix.

Both retained Seed and Commit scenarios remain generated, alongside the Stale
Validate refusal. Complete snapshots include Revisions and Transactions with
their declared identities and full field descriptors. Additional partial views
are independently observed; they do not substitute for complete coverage. No
scenario is executed by this preflight. Released-compiler adoption and real
provider restart/durability acceptance remain outstanding.

Compiler SHA-256: 61d8e008594e69763b07c824d32e6d647bc442f56bbd3bad5b4e8d0ba68558b6
Measured inventory: 33 generated, 0 authored, 0 outside, 0 refused; ess-conformance/13.
