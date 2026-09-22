# Format and seed checkpoint review

Source: edf4799b070fd71097b86bb457e0e7592221f917.
Opening base: b2b64f8159ecec0809b45e9f16183bf4e19ee8a2.
The implementor's checkpoint is retained verbatim in
p1-durable-seed-checkpoint.md. This is a bounded coordinator review allowing the
already-approved application milestone to continue, not final independent
acceptance or permission to publish an incomplete writer.

I read the seed admission/replay, physical publication, record, authority,
ontology encoding and public opening paths; compared the seed/parser/ontology
test migrations; and checked the exact frozen-source manifest and executed logs.
The five coordinator-owned files matched the coordinator before reconciliation.
Both source commit identities are the bot. The original legacy codecs, vectors,
legacy tests and vendored parser have no diff. The independent parser fixture
retains its original depth/accounting assertion under the current claim shape.

The original seed cases still execute through kernel authority. Native provider
injection in corruption fixtures prepares invalid retained state; it does not
replace the authority for the tested read. Identical seed retry now expects its
original success under the amended contract, while changed input still refuses.
The native publication explicitly calls AtomicBlobEventStore's portable method.
Store interpretation is fallible and refuses unsupported ordinary history.

The retained runner logs confirm the reported 114 selected kernel cases and
166 core/ontology cases, including two rustdoc cases. The native object case
passes. The five-library strict Clippy and scoped format checks exit zero.
Both bounded source mutations fail behaviorally; the source was restored
byte-for-byte and the selected kernel lane passed afterward. The complete target
compilation log remains red on old fixtures, with no claim of a full gate.

## Required continuation

- Close the reported inherited ontology collection strictness gap with real
  duplicate-input refusals and positive controls. New outer codecs alone do not
  cover nested declaration maps and sets.
- Remove the redundant caller ontology requirement from the public opener.
  Recover untrusted retained seed input, then validate full seed/context/exact
  host authority before exposing state. Preserve typed anchor mismatch so Seed
  can return AlreadySeeded without admitting state under changed configuration.
- Implement actual Propose, Validate and Commit through retained records, full
  basis, fixed trusted time and the shared application semantics. The shared
  command response values in DESIGN 93 bind actual records to presentation.
- Bound historical required input loading at the selected committed revision;
  stopping graph application after reading corrupt later payloads is insufficient.
- Complete lifecycle/supersession semantics, every retained terminal outcome,
  fresh-process changed knowledge, contention/interruption and original-result
  retries after later head movement, as the existing acceptance requires.
- Migrate old fixture/projection/compile-fail checks without discarding their
  invariants. The coordinator will reconcile AGENTS' obsolete ValidatedSeed
  mechanism wording with the actual final boundary and named executable cases.

No finding above is closed by allowing continuation. Both coupled stories stay
active. Final independent attack, complete repository gate and required CI still
precede publication. No ordinary durability, CLI, migration or later-phase exit
is inferred from this checkpoint.

## Shared response correction

The read-only CLI/Explain contract review identified an actual missing response
binding: Propose/Validate exposed no complete result while the Commit declaration
could represent only success. DESIGN 93 and kernel ESS now select the existing
ProposalRecordV1 and tagged ValidationCommandResult/CommitCommandResult values.
They add no persisted formats. Both active trees receive identical declarations.

Released ESS 0.29.0 validates, compiles and synthesizes the corrected specification:
35 generated, zero authored, zero refused, zero outside. No scenario was executed.
The exact outputs and individual exit statuses are retained under the unit's
private command-results evidence directory. Source handlers and later authored
conformance still have to verify these response values.
