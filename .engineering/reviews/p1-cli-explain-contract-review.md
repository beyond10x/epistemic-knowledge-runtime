# Prepared CLI/Explain contract reconciliation

Read-only review at coordinator `cb6dc4187216d54403699fa661861798dbb93b7a`,
2026-09-22. Reviewed the unapplied contract patch, its preparation note, private
host/time proposal, active DESIGN §§88–92 and kernel/graph ESS, and the current
CLI/explain story bodies. No moving durable implementation was inspected, and
no source, ESS, AEP, provider or build operation was performed.

**The proposed read shapes and host/time direction are compatible with P1, but
the prepared contract still needs the following bounded reconciliations before
dependent implementation.** One is a refusal-order conflict; the others are
concrete dispatch definitions, not evidence of faulty running code.

Source shorthand: `patch` = `.engineering/waves/p1-cli-explain-contract-draft.patch`;
`prep` = adjacent `p1-cli-explain-contract-preparation.md`; `host` = private
`cli-contract-preparation/host-and-time-choice.md`; `DESIGN` =
`docs/epistemic-knowledge-runtime-design.md`. Line citations identify those
reviewed bytes, not later amendments.

## R1 — Preserve Seed's declared anchor-mismatch outcome through reopen

`host:9–10` requires a mismatched authority/context anchor to refuse during reopen
before commands execute. DESIGN:3704–3710 and `kernel.yaml:856–863`, however,
require a changed actual bootstrap context or trusted anchor on Seed to return
`AlreadySeeded`, with no writes. If the trusted opener turns that mismatch into
an opaque startup fault, `seed` exits through the wrong contract and cannot run
the declared changed-anchor acceptance.

Required reconciliation: retain a typed mismatch without admitting state.
The Seed entry path can map that exact mismatch to its named `AlreadySeeded`
outcome; ordinary commands and reads still refuse the mismatched anchor.
Do not let this exception authorize a replacement registry or unverified read.
Corrupt retained material must remain distinguishable from an anchor mismatch.

Coordinator follow-up during this review says the current Seed handler already
maps retained `AuthorityMismatch` to `AlreadySeeded`, and the proposed opener
will preserve that typed distinction. That is coordinator-reported implementation
context, not independently inspected evidence here. The correction still needs
to be written into the prepared host/opener contract. Acceptance: after restart
and later head movement, change only the valid host context/anchor; Seed emits
the declared refusal with unchanged object/event counts, while ordinary commands
cannot use the changed anchor to admit state.

## R2 — Freeze the concrete host envelope and shared host boundary

`host:5–8,35–39` selects a required strictly decoded, format-tagged host document
but explicitly leaves its exact envelope type/home and public facade unresolved.
The patch introduces only command-result types (`patch:7–83`), so there is no
host-envelope declaration resolving that omission. An implementor still has to
guess the format literal, outer field names, document encoding and where decoding
and host construction live.

Before dispatch, record that small envelope around the existing AuthorityStateV1
and BootstrapContext, its strict duplicate/unknown-field behavior and its typed
home; do not invent another persisted authority version. Specify the kernel-owned
provider opener/host entry surface shared by CLI and controlled fixtures. A raw
store constructor or caller-supplied ontology is not an acceptable shortcut.
Error classification must retain R1 and distinguish malformed host input from
unavailable/corrupt storage; use the existing CLI exit policy rather than silently
making every pre-command failure a domain refusal.

The selected local trust assumption itself is coherent: the local operator
deliberately selects trusted host configuration, CLI acts as that configured
operator, and deterministic validation uses its distinct registered profile
validator. It does not need a new remote identity service, P5 capability policy,
or free-form actor override. State this assumption explicitly in the final
configuration contract rather than describing arbitrary proposal fields as
authentication. Match the profile/context and registered actors under
DESIGN:3637–3667,3767–3769.

## R3 — Bind Propose and Validate's actual public results before rendering

The patch provides typed command responses for Commit, Snapshot and Explain
(`patch:88–115`). Propose and Validate still have no `response` declaration:
active `kernel.yaml:870–947` declares their event/outcome metadata, including
only an issue count on TransactionRejected. Yet `prep:41–44` promises actual
records/issues in the public outcome, and `story:ekr-cli:65–67` requires a recorded
rejection to remain a successful declared outcome, not an error.

This is a remaining shared-result/API definition, not a request for another
persisted format. Select the existing retained proposal and validation/rejection
records as public outputs, or specify the exact verified read that supplies the
same records to presentation; give Validate an unambiguous validated/rejected
discriminator. The actual public handler result and ESS projection need to agree
before the CLI invents an output wrapper. If these records are intended as ESS
command responses, add their bindings alongside the proposed Commit wrapper.

A decisive control is a structurally valid proposal with an admitted transient
NaN spelling: Propose preserves the exact bytes and absent canonical hashes;
Validate returns the actual recorded named Type issue rather than a fake hash,
only an issue count, an operational fault or a JSON-coerced proposal. This follows
DESIGN:3757–3771,3979–3981 and does not add a seventh CLI verb.

## R4 — Specify the evidence set and revision boundary of the Explain list

`prep:25–39` fixes link ordering and includes later lifecycle receipts, but
`supporting evidence` is not defined as a selection rule. The draft can represent
the required records; this is a completeness ambiguity in the projection recipe.

Concrete P1 witness: assertion A is supported by retained HumanStatement E_A;
a later transaction adds B supported by distinct retained HumanStatement E_B
and supersedes A. Explaining A must preserve E_A and explain the recorded
replacement decision. An implementation appending only `A.evidence` can include
the later receipt while never exposing E_B's real source/address, producing a
different supposedly complete chain from one collecting lifecycle support.
Both evidences can already be in the seed; no P2 ingestion is required.

Define which evidence records the selected origin and lifecycle links require,
deduplicate by stable EvidenceId, verify their retained bytes, and order the
result deterministically. Also state that every link is selected from one
admitted revision reported in `ExplanationResult.at` (`patch:75–83`), so a head
advance while gathering records cannot add changes beyond that revision. The
already-admitted-replacement variant also needs an explicit rule: a supersession
receipt can reference an existing B without containing B's AddAssertion payload.
Resolve its acceptance/support through real retained history as required by the
chosen complete-chain recipe; do not fabricate an origin or infer acceptance
from the lifecycle pointer.

References: story seed-and-explain:48–58 requires exact origin, lifecycle-change
provenance and no convincing partial chain; DESIGN:3478–3492,3516–3521 requires
a genuinely accepted supported replacement. This report does not assert that
the draft necessarily omits E_B: it identifies the unchosen selection rule that
must make the two plausible implementations agree.

## Reconciled points and limits

- Snapshot's complete root/unfiltered graph plus optional matching ID list
  (`patch:15–27`, `prep:17–24`) preserves immutable-root meaning and both time
  axes. No wall-clock default or filtered-root substitution is introduced.
  The optional fields' joint-presence rule is explicit in prose and must be
  tested even though the struct shape alone permits incoherent combinations.
- Explicit supersession for the Alice/Bob example agrees with §88; plain
  retraction would remove Alice at all valid times in that later revision.
- Commit's Committed/Stale wrapper preserves actual existing receipt types
  and silent Committed retries. It is a command projection, not a retained
  receipt version change. Generated event fields must still agree with the
  actual selected branch; the wrapper does not authorize synthetic results.
- Lazy clock sampling, unchanged retry/replay times, and no public fake-time
  flag agree with §91.6. The shared injected host clock is still needed for
  decisive no-clock-on-retained-result controls; fresh-process receipt equality
  alone does not prove the callback was never invoked.
- Canonical decimal milliseconds plus exact calendar dates at midnight UTC
  is a CLI-only conversion and does not broaden core Timestamp encoding.
  Root Cargo.toml:34,40 already declares serde and time; adding those workspace
  dependencies to the CLI manifest is the narrow planned scope addition.
  Do not change versions or infer a requirement for a new date library.
- The agreed opener derives ontology from the retained seed, validates it under
  the exact trusted anchor, and replays retained history. This is consistent
  with DESIGN:3664–3667,3862–3875. **No caller ontology argument is missing**
  from the proposed host document. Reintroducing one would undo that agreement.
- HumanStatement terminates directly; the typed EvidenceRecord retains its
  real source. No Observation, interpretation, P2 adapter, trust-score model,
  migration command or extra CLI verb is required by this review.

Existing qualification artifacts report valid model, compilation status 0 and
35 generated / 0 authored / 0 refused / 0 outside scenarios. I read those
recorded outputs but did not rerun compilation or execute a scenario. They do
not establish shared-handler behavior, response correctness, no-write retries
or complete explanation. All runtime claims remain with the durable handback
and later authored acceptance. Preparation/host notes remain unapplied until
the coordinator resolves the definitions and activates the same contract in
every participating tree.

