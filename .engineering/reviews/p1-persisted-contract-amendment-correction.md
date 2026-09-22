# Amendment correction disposition

Reviewed the prepared `.engineering/waves/p1-persisted-contract-amendment-draft.md` in the coordinator working tree over `4032d00`. This is a static review of a draft, not normative adoption or implementation verification. No builds or repository edits ran.

| Original finding | Disposition | Exact correction |
|---|---|---|
| 1. Same-subject equality contradicts §65 | Resolved in draft | Lines 24–29 remove subject/predicate equality and retain normal ontology, provenance and cardinality checks. |
| 2. Graph/seed format nesting undecided | Resolved in draft | Lines 90–106 enumerate legacy and new graph/seed/envelope versions, exact nesting, and strict refusal of mismatched versions. |
| 3. Transaction-time changes and valid-time eligibility undecided | Resolved in draft | Lines 37–50 specify persisted execution-context timestamps, closure of recorded_to, replay without a clock, and an exact read predicate that admits supported superseded intervals. |
| 4. Acceptance evaluation point for atomic replacement undecided | Evaluation point resolved; one eligibility residue | Lines 24 and 52–56 allow AddAssertion plus supersession, evaluating acceptance in the fully validated candidate. However, an already-withdrawn replacement is not explicitly excluded. |

The remaining concrete case: a previously Accepted assertion is later Retracted. Under lines 16–17 it retains Accepted assessment, and it remains a retained record rather than being deleted. A subsequent transaction can name it as the replacement without retracting it again: it is Accepted and survives the candidate, satisfying the explicit conditions at lines 24–25 and 53–55. Yet the exact query predicate at lines 46–49 excludes it at every valid time. Superseding the old assertion to that replacement would therefore close the old interval without an eligible replacement.

Resolve the residue by requiring the replacement to satisfy the candidate's read-eligibility predicate at the supplied effective boundary, or by explicitly requiring an Active replacement with open transaction time and a valid interval containing that boundary. The former is the smallest rule aligned with the draft's own query semantics. A check against only Accepted assessment is insufficient now that assessment and lifecycle are separate.

Owners: this is residue of the fourth coordinator-owned contract finding, not a new seed implementation finding. The first three corrections introduce no further concrete contradiction found in this bounded pass.

## Coordinator response to the remaining residue

The coordinator independently checked the retained-acceptance/retracted-lifecycle combination.
The prepared amendment now requires the replacement to satisfy the candidate valid_at predicate
at the supplied boundary and explicitly refuses an already retracted or unreadable replacement.
This is a draft correction only; no writer implementation or runtime test is claimed.
