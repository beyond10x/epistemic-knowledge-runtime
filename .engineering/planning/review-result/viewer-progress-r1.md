---
format: aep.planning-md/3
id: review-result:viewer-progress-r1
kind: review-result
status: active
title: Application viewer progress independent review
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
needs-revision

Owners: 1 coordinator/viewer implementor finding; 0 delegated implementor findings.

Bounded source/log and retained-screenshot review of the uncommitted proposal progress renderer and CLI documentation over `5e5e814457888529aa57aea3e9c12b3f1d94c6c9`. No builds, tests, browser execution, source edits, new agents, AEP operations or publication were performed by this reviewer.

The overall progress label and correction label can use a different canonical boundary from the displayed item outcomes. `inbox.rs:230-238` captures the graph at `shown.basis.observed_revision`; item/schema commits are then checked against that read. But lines 373 and 416-419 take completion from `shown.application.remaining_items` and `corrections_pending`. `schema_proposals.rs:146-158` constructs the basis first and fetches application state in a later replay, so those flags can describe a newer prefix.

Concrete reachable interleaving: the schema is committed at revision R and one selected mapping remains. A page request captures its proposal basis at R; another caller commits that mapping before the proposal read fetches application state. The returned application has no remaining items, but the viewer intentionally reads revision R. It renders overall "Complete" while the selected item says "Pending integration". Similarly, a final correction committed between the captures can be advertised as committed before the page's captured revision. This is a source-derived concurrency/presentation defect, not an independently executed reproduction or an authority bypass.

Derive remaining item and correction status from the same exact committed publication set used for the item labels: selected item keys minus mapping commits admitted within the supplied read boundary, and original nonempty corrections minus an admitted final correction commit. Alternatively obtain all application progress from the same kernel capture. Do not use later projection flags to override the captured prefix. Add a deterministic helper regression using a genuine historical read before the final mapping/correction together with a genuine later proposal projection; require one consistent page status. Comparing event IDs alone does not close this mismatch.

Other inspected properties are sound within this scope. A Commit-command marker is counted only when its transaction has a real committed outcome and the event ID matches, so a prepared or Stale attempt alone does not count as progress. Dynamic item/source text and retained JSON use the existing escaping helpers. The renderer calls read APIs and exposes no write controls. Retained application records and receipts remain available in expandable sections. The documentation describes intended commit-derived behavior without claiming browser write support.

Evidence inspected: `<retained-evidence>/schema-application-kernel/viewer-progress-3.log` is terminal with one passed test, zero failed; source loops over File and SQLite, checking Not applied before application, Complete and schema revision 2 after cold/full replay, retained history, escaped script text and unchanged published events during rendering. The fixture has no mappings or corrections: this run does not establish genuine pending-item, partial-correction or Stale-attempt rendering. The current source also contains a helper check with altered publication IDs; that is not a real pending/Stale history and no separate chronology claim is made for its execution. Earlier fixture scope-order and absent-serialization failures are coordinator-supplied history, not independent reproductions by this reviewer.

The retained screenshot `viewer-progress/applied-proposal.png` was visually inspected: Complete and revision 2 are legible, the synthetic script text appears as text, and the layout is readable. It depicts the completed schema-only fixture; it cannot establish the concurrent or pending branches above.

Inspected SHA-256 fingerprints:

- `crates/ekr/src/cli/inbox.rs`: `b326dbfafc5265e9d009695d4694d350548367909f04cc3903c611166d9a3be2`
- `docs/cli.md`: `981a53fcaefc02e02604aca12c73e3899ad96c26c1e0c7251f9c62966a984fb6`

Limitations: no independent execution, full F acceptance or full-gate claim. This finding concerns new viewer consumption of separately captured state; it does not assert that the existing kernel proposal API was independently reproduced on its base commit.

```findings
[
  {
    "file": "crates/ekr/src/cli/inbox.rs",
    "line": 373,
    "category": "correctness",
    "severity": "blocker",
    "verdict": "NEEDS-CHANGE",
    "origin": "introduced",
    "message": "Progress uses application.remaining_items/corrections_pending from a later proposal-state capture while per-item commits use read(shown.basis.observed_revision). A concurrent final mapping commit between the kernel's basis and application captures can render Complete together with Pending integration; final corrections can likewise be claimed beyond the captured revision. Derive every completion label from the same admitted commit set/read boundary, and test a genuine old read with a genuine later projection. Source-derived presentation race; no independent reproduction or authority-bypass claim."
  }
]
```
