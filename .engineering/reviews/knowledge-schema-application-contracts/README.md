# F cursor observability repair — bounded author handoff

The three predicates are preserved as authority-entity invariants and observable scalar view
fields. All seven component suites now synthesize without refusal. This is not execution of
those invariants or F conformance. No Cargo, runtime, CLI/SDK, AEP, commits or publication ran.

Owners: ESS contract author; runtime admission/replay and authored view conformance remain with
the implementation coordinator.

## Incremental scope

`systems/ekr/domains/integrate.yaml` adds three retained/derived read entities and matching views:

| Entity | Exact retained predicate | View |
| --- | --- | --- |
| ApplicationPublication | review_stream_version > 0 | ApplicationPublicationRecords |
| ProposalCoordinationOccurrence | stream_version > 0 | ProposalCoordinationOccurrenceRecords |
| ProposalCoordination | stream_version >= 0 | ProposalCoordinationRecords |

A publication row uses the real ordinary event ID and references the retained proposal,
application and ordinary transaction. An occurrence row uses the native proposal/sequence address
within the trusted tenant. A coordination row is the current fold for a retained proposal,
including an empty stream at cursor zero. Rows project existing streams; there is no new queue,
retention database or synthetic lifecycle create outcome.

The predicates moved from untrusted transport types to these authority entities. Their scalar
fields must exactly match the corresponding complete generated records. Store/kernel admission
and replay still reject invalid or mismatched cursors and links. Transport deserialization alone
no longer claims their truth.

Design §105.17.1 names the remaining authored scenarios:
`proposal_coordination_zero_and_positive_cursors_are_observable`,
`invalid_application_and_coordination_cursors_are_refused`, and
`application_publication_view_requires_exact_marker_links`.
They require both providers and appropriate reopen/full replay. They were not implemented or
executed here. Since the current commands have no truthful lifecycle-subject outcome for these
read entities, synthesis produces no generated invariant-execution claim. This limitation is
explicit, not hidden by an invented create outcome.

## Exact verification

Pinned released ESS 0.52.0; generator pin unchanged.

- Validation: 9 files, valid. Compilation: 566 declarations, compiled.
- All seven `conform synthesize --component ... --suite-format 5 --compact` calls exit zero,
  write suites and contain zero refusal diagnostics: kernel 46 scenarios, ontology 0, graph 0,
  store 0, observe 5, integrate 19, views 47. These are synthesized counts, not execution counts.
- Rust synthesis: 746 capabilities, 696 generated, 39 obligations, 11 refused; 32 artifacts.
- Data generation: 477 types and 231 retained report obligations. Moving transport constraints
  changes this accounting; it does not discharge the authority/entity invariants.
- Both generated artifact trees independently regenerated into fresh directories and matched
  byte-for-byte, excluding only recognized `.ess-output` operational metadata.
- `git diff --check` passes. No Cargo lane was used.

Source digest: `d93593588d2611a3d25542c06ca23a3ac2992d57a37c6785cf8b89f0846dc087`.
Semantic contract digest: `ea2e8b0cc8f708c0b41768fcc389f371d3eddc8554b05b10d5d26385cc30a130`.

Incremental patches are `observability-source.patch` and `observability-with-generated.patch`,
against the exact prior F source/generated snapshot already integrated by the coordinator.
No E fixture metadata or conformance suite files are part of these patches. The exact suite
outputs/logs are `observability-*-suite.json` and `observability-*-suite.log`; model generation and
fresh-output comparison logs use the same `observability-` prefix.
