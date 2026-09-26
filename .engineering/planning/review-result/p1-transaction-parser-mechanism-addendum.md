---
format: aep.planning-md/2
id: review-result:p1-transaction-parser-mechanism-addendum
kind: review-result
status: active
title: Bounded parser review mechanism correction
relations:
- reviews: task:bounded-transaction-document-parser
- reviews: story:commit-and-revision-lineage
revision: 1
---
# Transaction parser review: mechanism correction and bounded capability scope

This addendum corrects the causal explanation in the preserved `report.md`; it does not replace that report, its original red case, or its exact YAML. The acceptance finding remains **NEEDS-CHANGE**. This follow-up is source inspection and evaluation of the implementor's retained R1 evidence, **not a new executed test or verification of a correction**. No compiler, parser lease, source/test edit, AEP action or provider was used.

## Corrected mechanism

I retract the original claim that the representation pass counted the long global tag but resolved its scalar as a number. I also retract the claimed representation subtotal of 658,463 and the attribution of this particular witness to the budget reset.

In pinned `serde_yaml_ng 0.10.0`:

- `src/de.rs:1193–1203` exposes a tag to the Serde enum visitor only when its decoded bytes start with `!`. The decoded global URI in the witness does not. `deserialize_any` applies this filter to scalar and collection tags at lines 1214–1254.
- `visit_scalar`, lines 871–901, handles recognized built-in tags and local-tag scalar inference specially. The witness's unknown global tag takes neither branch; its numeric-looking scalar is delivered as **String**, with all 32,768 text bytes.
- `deserialize_str`, lines 1472–1492, reads the scalar text without consulting its tag. This also means the representation's requested String map keys hide both local and global tag metadata; `deserialize_string` delegates to that method at lines 1495–1499.

The retained implementor log `correction-r1/accounting-probe.log` records representation strings **658,663**, followed by twenty typed borrowed strings of 32,768 bytes, each already credited. It ends with the unchanged independent test's same acceptance failure. I inspected that log and the source paths above; I did not rerun the probe.

The correct arithmetic for the original witness is:

| Content | Bytes |
|---|---:|
| Materialized scalar strings, 20 × 32,768 | 655,360 |
| Shared envelope, keys, UUID strings and CreateNode tags | 3,303 |
| Representation subtotal measured by R1 | **658,663** |
| Global scalar tags omitted at the Serde boundary, 20 × 32,758 | **655,160** |
| Required complete profile total | **1,313,823** |

The profile limit remains 1,048,576. The total, exact 66,005-byte YAML, positive controls and acceptance defect are unchanged. **The original witness proves missing tag accounting, not a split of its tag and scalar charges between the two passes.** Other scalar spellings can genuinely differ between representation and requested-String decoding; those still require per-occurrence string credit rather than resetting or blindly adding two whole-pass totals. That separate architectural concern is not evidence for the retracted explanation.

The corrected finding statement is: *The bounded representation cannot observe supported global tags, or tags on requested String keys, through the selected public Serde visitor API; omitted decoded tag bytes allow the frozen cumulative string budget to be exceeded.* This is an EKR integration defect under its new profile, not a claim that the library changed or violated its existing decoder contract.

## Recommended smallest capability

Use an **opt-in safe observation facade over the exact vendored loader events**, consumed only by EKR's bounded preflight. Keep existing deserializer entry points and default typed behavior unchanged.

The data already exists before the Serde adapter discards it: `src/libyaml/parser.rs:35–53,116–144` retains full decoded scalar/collection tags, decoded scalar bytes and scalar style. `src/loader.rs:14–19,82–116` retains those events plus resolved alias IDs and target positions without expanding the alias tree. The new facade can borrow immutable event views from an opaque loaded document, exposing scalar/collection kind, decoded tag metadata, decoded scalar text/style, document/error status and safe alias target identities. It need not expose raw pointers, unsafe APIs, internal C types or an expanded Value tree.

Raw scalar text alone is insufficient to preserve representation semantics. Provide a narrow scalar classification/visitor operation that **reuses the existing scalar resolver and its local-enum payload context**, rather than requiring EKR to recreate YAML numeric, null or tag resolution. In particular, unknown global tagged numeric-looking text must remain String, while a local enum payload retains the existing `current_enum` behavior. Final strict typed decoding still consumes the original bytes.

Merely extending `parse_tag` to preserve every tag through `visit_enum` is not sufficient. It changes the representation's scalar-resolution path, still does not expose tags to requested String keys, and risks presenting an enum to a visitor that expects scalar text. An orthogonal callback could be viable if it observes each actual expanded occurrence across **all** relevant entry points without changing visitor dispatch; handling alias revisits and enum re-entry correctly makes that a less direct boundary than the existing event tape. Do not change all consumers' Serde behavior to repair one preflight.

## Counting and compatibility cautions

1. **One tally per expanded occurrence.** Charge decoded tag text separately from scalar/key content. Reuse the current local enum-tag spelling for its existing charge; do not charge both a raw `!CreateNode` tag and its already-counted `CreateNode` visitor string. Preserve complete decoded global URI text. Requested-String conversion adds only scalar bytes not already credited for that occurrence. Do not sum two complete passes or globally deduplicate anchor contents.
2. **Metadata does not invent enum boundaries.** Preserve the frozen node/depth definitions and existing enum-payload classification. Observing a tag ignored by requested String decoding must not itself turn that String into an enum. Tag-byte accounting and semantic enum-boundary counting are distinct.
3. **Traverse aliases in place with the shared budget.** Every alias occurrence charges its expanded target at the caller's depth; repeated siblings remain separate charges. Refuse cycles/excess expansion before constructing their expanded children. A single linear sum of source events misses repeated tags and is not a solution.
4. **Keep refusal order.** Check full collection presence before visiting the extra child; decode/check a duplicate key before its second value. Integrate tag observation at those bounded traversal points, rather than first recursively charging metadata in values the contract requires leaving unvisited. Preserve actual String key decoding, typed key uniqueness, arbitrary Record keys, and required-container syntax checks.
5. **No new allocation claim.** The loader still buffers events and decoded scalars under the existing raw byte cap before this observation. The proposed capability supplies missing metadata; it does not establish pre-loader event/scalar quotas. Keep the frozen limits, exact original bytes, nonfinite spellings and default decoder semantics unchanged.

The bounded vendor surface should be the opt-in facade/export, access to the existing private loader/scalar-resolution helpers as required, and focused package tests. Root owns the precise Cargo/vendor decision, provenance and scope. No alternate lexer, second YAML grammar, unsupported-tag ban, direct unsafe EKR integration, or new document format is recommended.

Correction acceptance should retain the original independent case unchanged, add exact/one-over global/local tag and tagged-key controls with aliases, and prove that tags already visible through enum visitors are not double-counted. Include direct-decoder parity for built-in/global/local tags, String coercions, key identity and collection shape, plus existing duplicate-before-value and allocation-order witnesses. Those are proposed checks, not results from this read-only follow-up.

Owners: I own the original review's incorrect mechanism attribution and this correction. The implementor owns the parser accounting repair once the coordinator supplies the approved dependency capability; the original observed acceptance defect remains introduced by the new bounded-parser implementation. The coordinator owns vendor adoption and any precise contract clarification, integration and publication. No new finding against the library's default behavior is asserted.

