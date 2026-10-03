needs-revision

Owners: 0 coordinator findings; 1 delegated adapter implementor finding. The coordinator owns the acknowledged external Submit witness limitation and unimplemented F application; neither is counted as an adapter defect.

Bounded read-only source/log review of the three Rust files and two finite literal fixtures in source.patch, SHA256 86affa74ac99c9d7cd0348dce58e18fee1c95316949cdcd19461b6602527f205. No Cargo, tests, repository edits or upstream actions were performed by this reviewer. This review does not approve the full conformance gate.

1. Blocker — canonical side effects of E commands are not observed. crates/ekr/src/conformance/knowledge.rs:524 dispatches the real Runtime command without capturing the canonical head or canonical occurrence stream before/after. At line 535 the review-specific postcheck reopens with full replay, but checks only that a returned review ID has the expected Approved/Rejected decision, or that a refused review left no review record. A successful full replay establishes valid stored history; it does not establish that this E command left canonical history unchanged. The adapter subsequently reports the normal success/refusal from the returned result.

   Consequently, a command that retains the expected review and also commits an unrelated valid ordinary transaction would still satisfy the review postcheck and generated result/event expectations. A refused command that first publishes a real ordinary occurrence would likewise remain undetected if it left no proposal-review record. This contradicts the review contract's unchanged canonical revisions and KnowledgeRefused's no-unreported-write guarantee (systems/ekr/domains/integrate.yaml:1453 and :1207). This is a concrete latent adapter observation gap, not evidence that the current kernel performs either erroneous write; no mutation reproduction was executed by this reviewer.

   Capture the verified canonical head and actual canonical occurrence stream before A/E dispatch; reopen/full-replay after every result and compare them before reporting the outcome. Permitted observation, interpretation, proposal and review retention metadata must remain distinguishable from canonical publications. Add a negative control that performs a real canonical write between the observations and requires the same adapter guard to reject the otherwise expected result. Do not substitute a fabricated counter or count the already failing Submit case as this control's failure.

The exact-input path otherwise forwards the requested generated values to native Runtime operations. The nominal HumanDecision1 enum conversion is an explicit semantic-to-wire conversion, not replacement of signed intent. External refusal setup establishes actual input/state, while the observed outcome comes from Runtime. The finite interpretation and proposal fixture bytes are not replaced by synthetic successful commands. Authenticated review persistence is checked after reopening/full replay rather than inferred solely from an event-shaped response.

The retained evidence is candid about its limits. <retained-evidence>/reports-final-3/partial-File.json and partial-Sqlite.json each record 19 total, 16 passed, 1 failed, 0 error, 2 unsupported and 0 skipped. SubmitSchemaProposal/answered remains failed because the finite generated witness is not an admissible proposal; the two ApplySchemaProposal scenarios remain unsupported because F is absent. The full 19-scenario floor and zero-unavailable requirement remain unchanged and red. The invalid witness explanation comes from source/fixture inspection; the run diagnostic records its outcome mismatch, not a separately observed native refusal code.

Mutation accounting correctly excludes existing failures: event/response suppression records 9 total failures, of which 8 are additional, and signature corruption records 3 total failures, of which 2 are additional, on each provider. Neither control observes unexpected canonical writes. final-3.log reports the Rust target as 5 passed and 3 failed; clippy and formatting status files are zero. These are implementor executions inspected by this reviewer, not independent execution. The report completed_at field is fixed synthetic fixture-clock time and is not actual execution UTC or freshness evidence.

```findings
[
  {
    "file": "crates/ekr/src/conformance/knowledge.rs",
    "line": 524,
    "category": "acceptance",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "The adapter dispatches A/E commands without observing canonical head/occurrence changes. Its review postcheck only verifies the retained review ID/decision or an empty review history on refusal, so an unexpected valid ordinary canonical publication can coexist with the expected result and remain a passing scenario. Capture and compare verified canonical head and actual canonical occurrence stream before/after every result, and add a real write-then-return negative control through the same guard. This is a latent adapter observation gap, not an observed kernel defect or an independently executed mutation."
  }
]
```
