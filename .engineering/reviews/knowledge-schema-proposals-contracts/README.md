# Schema proposal contract prerequisite

Source base: `5398f77681e21c382fee1608eead1e80c3afe65f`.
Refined specification digest: `f878e31bbf912e1096577aea52f5df406087b6610af3197f6839e5a414249291`.

ESS 0.52.0 validates the refined contract. Both generated model trees match fresh output and
the synthesized workspace compiles. No runtime model was hand-transcribed. This checkpoint
does not implement proposal commands or establish implementation conformance; component suites
must be regenerated and executed alongside E/F, without relaxing their floors.

The refinements make mappings identify an exact immutable source version and fact position,
expose the material basis and previous proof digest to external signers, and name proposal/review
transport and retention records using existing entities. Independently retained human evidence
does not itself advance canonical revisions. Enum variants require complete-set equality with
a selected retained local declaration. Unknown or ambiguous canonical subjects and relation
objects remain blocked; there is no inferred entity creation.

Independent read-only contract review found no blocking inconsistency. It requested one wording
correction: the signed target has neither recording time nor a separate review id, so only fields
represented in each record must agree. The final comment distinguishes signed target fields from
HumanDecisionRecord fields. The correction is included; this was source review, not independent
execution. The E scoping and review dispositions are recorded in the integration planning store.

Review also identified a mandatory F race: rejection can advance the review stream while leaving
canonical head unchanged. Canonical-head CAS alone therefore cannot ensure the latest approval
governs publication. F must atomically guard review state alongside each canonical publication.
E may record a decision on a verified observed basis and CAS only the immutable proposal and
review predecessor; it never grants unconditional applicability. F's durable elections and
qualified remaining-item/provenance projections still need contracts before implementation.

Only host path prefixes in logs were sanitized.
