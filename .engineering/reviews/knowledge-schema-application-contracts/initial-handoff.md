# F contracts and generated models — author handoff

Contract authoring and generation are complete for this bounded prerequisite. Runtime behavior,
Cargo compilation, old-format fixture tests, crash/reopen/full replay, provider conformance and
full `task check` were not executed here. This is not F acceptance or an independent review.

Owners: ESS contract author; integration and runtime verification remain with the coordinator.

## Source delivered

- `systems/ekr/domains/integrate.yaml`: immutable application and step elections, per-step
  transaction attempts, source/mapping-qualified item keys, effective-review guard and physical
  publication marker, typed coordination history, named retained mapping/derivation projections,
  application reads and resumable reports/receipts. E Show/Approve semantics now account for
  verified own-prefix progress and residual reviewed material.
- `systems/ekr/domains/store.yaml`: typed election/step/attempt retention and history carriers,
  historical `/6` preparation reconciliation and guarded `/7` atomic publication obligations.
- `systems/ekr/domains/kernel.yaml`: the minimal related declaration is optional `application` on
  RevisionEventV3, present only in the new guarded ordinary `/6` envelope. Its absence preserves
  all historical bytes/hash inputs; generated data uses explicit absent-field serialization.
- `docs/epistemic-knowledge-runtime-design.md`: appended §105.17, including the actual source seams,
  stale-attempt recovery, strict corrections-last order, prefix-aware review and replay obligations.
- Both `generated/ekr-contracts` and `generated/ekr-contract-data` are tool output only. No generated
  model was hand-written and no runtime/CLI/SDK/conformance/AEP file was changed.

The patch `f-source.patch` is relative to the original base plus the coordinator's supported
E-only fixture metadata. It contains no fixture-input changes. `f-with-generated.patch` adds the
regenerated model trees. The separately saved E prerequisite patch and unsupported fixture probes
remain unchanged. The unresolved recursive document/proposal fixture bindings are absent from the
final specification; the remaining E metadata supplied by the coordinator is present.

## Exact verification

Released ESS 0.52.0, source `4d6a4ecafc0feb4e11e4bee777b19c7351fa3647`.
Binary SHA-256: `7c0f35fd2c2365c2390113f756eba1275ea25902d04f7ca5a8f8f6ab30395be7`.

- `validate`: `ekr v1 — 9 file(s), valid`.
- `compile`: `ekr v1 — 9 file(s), 563 declaration(s), compiled`.
- Rust workspace synthesis: `734 capabilities: 684 generated, 39 obligation(s), 11 refused`;
  `32 artifact(s)` written. The obligation/refusal identity sets are identical to the baseline.
- Rust data generation: `474 model type(s)`; the generated report retains 237 runtime obligations.
- Independent regeneration into fresh directories compares byte-for-byte for both full artifact
  inventories, excluding only the recognized `.ess-output` operational metadata directory.
- `git diff --check` passes. No Cargo lane was used.

Source digest: `69deb65cffac0d3d92bfd8884ad740651f52ebe5406c5ca1c8cfc8d57dc6fdb4`.
Semantic contract digest: `90770751542db3e875d8ccfda521685ffa0d05f34995dba8d42efe760352e5ae`.
Data schema digest: `8cc2ffd3f7cb3404c218c26a54fc359116d33c4ce218452d87d88b99506dbb6e`.

Exact CLI outputs are `f-validate-final.log`, `f-compile-final.log`, `f-semantic-final.log` and
`f-data-final.log`. Independent generation logs and zero-byte comparisons are `f-verify-*` and
`f-*-drift.log`. Earlier validation logs preserve the repaired authoring errors; they are not final
success evidence. Baseline generated ownership was adopted only after the CLI verified its bytes
against a freshly generated exact baseline; no overwrite refusal was bypassed.

## Design choices requiring implementation checks

An immutable step retains the initial ordinary transaction template. A distinct generated
RetainedApplicationAttempt fixes each transaction ID. Only a resolved terminal Stale attempt may
have a successor, and all other transaction fields/allocated identities remain identical. A
rejected attempt stops; uncertainty, Propose or Validate is not permission to elect a successor.
Atomic attempt-chain election prevents competing successors. Receipts name the actual committed
attempt. Planned transaction/derivation documents do not reference nonexistent canonical rows.

Every application ordinary occurrence needs the exact effective review and a nonempty marker in
one atomic provider group. The physical review-stream cursor includes markers, while the signed
human predecessor excludes them. Cold replay checks their one-to-one links; generic commit cannot
omit the guard. The guard contains no event/marker hash, avoiding circular digest requirements.

Schema comes first, all selected mapping items follow, and all selected corrections are one final
atomic step. Unresolved mappings cannot be silently dropped. After that correction commits, only
receipt recovery/reporting remains. Completion is reconstructed from actual commits if recording
a receipt crashed. Initial and later approvals are interpreted against their own verified prefix
and approved residual plan, never an externally coincidental change. Original corrections remain
immutable. Changed frozen operations require a new proposal/election.

Mapping bytes are generated-data compact JSON, hashed independently of their envelope. Proposal,
source, mapping and admissible evidence bytes remain independently pinned and verified. An optional
observation link permits HumanStatement-only support without inventing observations; required
Evidence and mapping references remain. Live Interpretation entities/TransientGraph roots are
never canonical replay dependencies.

Existing serialized field names remain unchanged. The prospective F report now permits absent
receipt/schema revision before a verified schema commit, qualifies remaining_items, and adds
application identity/correction progress; post-schema persisted receipts retain mandatory commit
references. The existing E read gains optional application progress. Runtime constructors and
frozen historical event/preparation tests must be updated and run by implementation workers.

No unrelated polling, checkpoint, migration, merge/split, retention/GC or hosted PostgreSQL scope
was closed or changed. The ESS recursive-fixture conformance gap remains separately recorded in
`fixture-options.md`; neither model generation nor this author handoff resolves it.
