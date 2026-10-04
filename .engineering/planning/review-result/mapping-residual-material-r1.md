---
format: aep.planning-md/3
id: review-result:mapping-residual-material-r1
kind: review-result
status: active
title: Residual mapping material independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 coordinator/kernel implementor finding; 0 delegated implementor findings.

Bounded source review of the residual projector, committed-prefix capture, continuation basis, replay graph retention and receipt derivation in the current integration WIP. This is not acceptance of mapping orchestration, correction application, full F or the full gate.

Completed schema additions still contribute live dependencies to residual review material. `application_material.rs:29` clones the complete proposal and removes completed mappings/corrections, but retains every addition after the schema commit. `schema_proposals.rs:328` passes that residual document to `schema_proposal_material::relevant`, whose addition loop at line 96 includes the completed declarations. Consequently an external change to a declaration needed only by the completed schema step changes the residual effects digest and forces another approval for otherwise unchanged remaining work.

Concrete reproduction to add: approve a proposal that adds independent type A and maps an item into existing type B; retain its verified schema commit while leaving the mapping pending. Add an optional property to A through an unrelated ordinary transaction. The remaining B mapping and its dependencies are unchanged, but the current projector includes A's changed declaration and continuation rejects the existing approval. This is a source-derived conservative refusal, not an authorization bypass or an independently executed reproduction. Keep the exact original proposal bound in the residual options/prefix, but compute live dependency material from remaining effects; retain A as a dependency when a remaining mapping actually uses it. Pair the negative case with a control changing a dependency of B, which must still require renewed review.

The inspected prefix helper counts replay-admitted committed transactions and explicitly filters their canonical revisions (`application_material.rs:50-55`), so the full transaction map retained by historical reads does not by itself introduce future progress. Continuation compares each own commit's pre-commit material before advancing to its post-commit basis (`application_auth.rs:320-347`); unrelated intervening changes are not automatically absorbed. Replay's extra pre/post coordinates retain graphs rather than authorize progress. Processing and application receipts are compared with verified committed prefixes (`application_progress.rs:137-218`); the projector does not derive authority from receipt claims. No additional concrete defect was established in those bounded paths.

Limitations: source inspection only; no Cargo, tests, mutations or build commands executed. No new execution evidence was supplied or independently claimed for these helpers. The integration is explicitly incomplete, and no finding is raised merely for the pending larger F work. The repository and AEP store were not changed; this report is retained separately as `<retained-evidence>/schema-application-kernel/residual-progress-review-r1.md`.

```findings
[
  {
    "file": "crates/ekr-kernel/src/application_material.rs",
    "line": 29,
    "category": "correctness",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Prefix::remaining retains completed schema additions. schema_proposals::project_captured therefore still hashes their live declarations through schema_proposal_material::relevant, even when the remaining mappings do not depend on them. After this application's schema step adds independent type A, an unrelated optional property addition to A forces renewed approval for an unchanged pending mapping into existing type B. Preserve the original proposal binding, but derive residual live dependencies from only the remaining work, with a control proving changes to a remaining mapping's actual dependency still require review. Source-derived conservative refusal; not an independently executed reproduction or an authority bypass."
  }
]
```
