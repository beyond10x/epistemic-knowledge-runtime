# V2 format and durable-writer activation patch

Owners: coordinator for DESIGN/ESS application and scope; format implementor for current codecs and graph projections; writer implementor for retained records, authority and replay; CLI/conformance implementor for document handlers and actual scenarios.

This is scratch preparation against exact EKR commit `a2593122b04c3b422cf685ba7fc0db2444e3b3a5`, dated 2026-09-22. No repository, planning record, production source, frozen legacy encoder, provider store or service was changed. No Rust build, product test or kernel scenario ran. The patch is a proposed activation contract, not evidence of runtime agreement. Amendments 88, 89, 90 and 91 each carry the explicit date 2026-09-22.

## Deliverables and application boundary

`activation.patch` changes exactly five tracked paths:

- `docs/epistemic-knowledge-runtime-design.md`: append §§88–91; existing sections remain unchanged.
- `systems/ekr/domains/graph.yaml`: independent assessment/lifecycle and complete typed payload projections; nonempty ordered outer property collections; canonical-byte value projection; complete admitted graph/seed-support shapes.
- `systems/ekr/domains/kernel.yaml`: EventId and strict event/2 shape; authority/profile, seed context, complete records for all six occurrences and validation basis/material; typed canonical operations; document inputs and retained Transactions.
- `systems/ekr/domains/ontology.yaml`: full recursive value-type and complete declaration/document projections so populated ontology roots can bind every semantic field.
- `systems/ekr/domains/store.yaml`: metadata-only ObjectStored; required blob bytes leave the event only when atomic provider publication activates.

The exact `proposal/` files accompany the unified patch; `baseline/` contains the corresponding source snapshot. `git apply --check ../activation.patch` in the baseline copy exits0. The patch adds 1485 and removes 96 lines. It includes all adopted contract changes, rather than appending the earlier conformance patch without reconciliation.

Keep current production APIs during preparation. Coordinate graph/seed/current transaction codecs, atomic provider blobs, writer authority and these declarations before claiming activation. Applying the declaration patch alone will deliberately disagree with current source projections; do not remove those tests to obtain a green preparation commit. No original-format fixture or encoder is changed or superseded.

## Settled translations

The coordinator's `p1-durable-record-decisions.md` overrides older preparation recommendations. The patch therefore uses event/2 and backend schema2 throughout, not the earlier proposed event/3; full host AuthorityStateV1 and prior receipt address in the basis; complete ontology and authority roots; exact document bytes rather than a JSON GraphTransaction field; immutable own-result retry; and fallible real-kernel replay/apply.

The later coordinator decision fixes one UTF-8 YAML transaction document:
`{format: "ekr.transaction-document/1", transaction: <full typed GraphTransaction<Value>>}`.
It uses the established serde_yaml_ng0.10 parser family. JSON is only that parser's supported subset. The original bytes, including well-formed NaN spelling, survive Proposed/restart and then the real Validate refusal. Record keys remain user data. Parsing, hashing and validator eligibility are separate facts.

The patch makes previously prose-only record spelling concrete, without adding authority:
`ekr.seed-result/1`, `ekr.rejection-record/1`, `ekr.stale-record/1`; field names
`document_bytes`, `authority`, `requested_basis`, `rejected_at`, `stale_at`,
and the observed root/revision/event/record/hash fields. The coordinator accepted these exact new spellings as translations, subject to independent activation review before source dispatch. Their semantics come directly from the adopted preparation; no compatibility translation is implied.

The original six revision names/indices remain Seeded/0, TransactionProposed/1, TransactionValidated/2, TransactionRejected/3, TransactionStale/4 and RevisionCommitted/5. Proposed's optional operations_hash accommodates a recorded invalid Float without fabricating an address. Retraction retains operation index5 with reason; new SupersedeAssertion appends at index11. Operation application is unordered, while the operation vector's actual canonical order remains address-significant.

Graph does not gain a dependency on kernel receipt implementations: AgentId, IssueId, EventId and revision identifiers are foundational ekr-core identities projected under the existing kernel namespace. Graph assessment carries IssueIds, not ValidationIssue records. All authority, receipt and apply semantics remain kernel-owned; store retains generic payload bindings/coordinates. The graph-document projection describes admitted inputs; original transient source documents still use the strict Rust typed parser.

## ESS capability evidence and limits

Installed compiler: `ess 0.26.0`, binary SHA-256
`bf7ed60017e6634c736e7b2fcdf97d0402feadd8983b74eb7b74903593d57b0d`.
The requested source was inspected at `a5f1bea13294510819b266561c83be9509e6ba57`; the executable's build revision is not independently attested. Its hash and actual behavior are the measurement authority here.

The final candidate proves Bytes and recursive named types are accepted. Generated schema projects Bytes as a base64 string, not a numeric-array log payload. Earlier comments claiming that ESS lacks Bytes/recursion are corrected only where this activation needs their types.

An initial candidate using enum variant objects with a custom wire spelling was refused by this installed executable: `invalid type: map, expected a string` for graph and kernel. The final patch uses named String format types and explicit required literal comments; runtime strict version dispatch must enforce those literals. It does not pretend that the generated schema constrains them.

ESS has no usable empty-struct payload for a unit variant in this model. Following the coordinator's accepted projection decision, AssessmentProjection and AssertionLifecycleProjection contain a kind and exact typed optional payloads. These are explicitly projections, not the Rust wire enum. Required per-kind field presence/absence, bounds, set uniqueness, UUID map-key agreement and source-kind coherence are runtime obligations, not established by ESS validation. Canonical graph values expose exact canonical bytes/kind; no generic JSON or Float-to-Decimal substitution was introduced.

`task:ess-has-no-byte-string-type` and the compound-value/projection tasks are not closed by this discovery or this draft. Their full runtime changes and required tests must actually be implemented first.

## Measurements

| check | baseline | proposal |
|---|---:|---:|
| ESS validate | exit0, six files | exit0, six files |
| ESS compile JSON IR | exit0 | exit0 |
| kernel synthesize, suite/5 | exit1 | exit0 |
| generated scenarios |26|30|
| authored scenarios |0|0|
| outside obligations |0|0|
| synthesis refusals |4 ESS-SYNTH-011|0|
| runtime executions |0|0|

The four formerly refused obligations are GraphTransaction's operation_count invariant after Commit/committed, Commit/stale, Validate/rejected and Validate/validated. The unfiltered durable Transactions view makes all four expressible. Counts can grow; 30 is not a permanent cap.

The compiler generated 167 public schema files. Two sequential generations produced byte-identical `schema/` subtrees. The output ownership state files differ as expected and are not contract artifacts. An attempted concurrent generation was refused by the compiler's parent output-ownership lock; the second command was rerun sequentially. No artifact was edited to force a comparison.

Recorded addresses:

- activation.patch SHA-256: `1ee49a48fa1afbb6b24d8e1415e69a5135846233e7c34cecba5d9fe2ec4824a1`
- proposed suite byte SHA-256: `c3d00035d2842c79257e54a4e5551d95e699402e2fc3628658b4cd7eaf68f2ab`
- baseline suite byte SHA-256: `91af1a90c7a9b582d9369c85efd0982af6c73f1af7fde12b8c17865984d45a8e`
- proposal IR byte SHA-256: `cab8d82a3bf9ce1c49d485e0ae37969225c76e30ec18b984c94579e38a8f1cfd`
- proposal spec digest: `6cd0f5deaf44398d891fc55675cc86a88c6a86cebc24c85a1854c3b43f3856e8`
- proposal contract digest: `e8510eb43e155f85197591b6e9dfb829d2a872725b01a3f858fc019f11b37265`

`commands.md` records reproducible relative commands. `coordination.md` names existing tests requiring coordinated updates and proposed future cases, all unexecuted here. Do not publish the compiler's private output-ownership directories; the patch, this report, IR, suite and actual generated schema subtree are public-safe contract artifacts.

## Settled document-limit boundary

The coordinator settled DocumentLimits as trusted host configuration, fixed for an execution and unavailable in caller documents. The draft requires conservative explicit implementation defaults, bytes/depth/collection bounds, named refusals and exact-boundary tests. Numeric defaults are chosen and recorded with the shared parser implementation, without changing the document grammar or authority meaning. No semantic contract question remains pending in this preparation.

No parser, persisted record, runtime projection, mutation, restart, migration, conformance Runner or full gate result is claimed. Those remain the implementation acceptance, with preservation-first migration after the durable path.
