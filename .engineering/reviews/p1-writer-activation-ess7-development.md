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
