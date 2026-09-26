---
format: aep.planning-md/2
id: review-result:p1-15-schema-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-15 schema unit
relations:
- reviews: story:machine-readable-document-schemas
revision: 1
---
Adversary pass 2 on unit p1-15-schema (wave p1-15), `aep-drive:adversary`, against `1f9f663`.

Owners: 6 findings, 6 implementor (introduced).

Verdict: NEEDS-CHANGE (1 warning), CONFIRMED (2 warnings, 1 note), INFEASIBLE (2 notes). cases: executed 210→216, red 6.

Cases in `crates/ekr/tests/adversary2_p1_15_schema.rs` (6; 6 red): a string over the byte limit in fewer characters, a tag on a mapping or list, distinct texts YAML resolves to one value, the host schema's fixed texts, ids described as minted with no mint kind, the CLI page's gap list.

Routing (coordinator): all six back to the implementor; 1, 5 and 6 as named gaps, 2, 3 and 4 as fixes.

```findings
[
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 181, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "string_bytes and key_bytes are byte limits but the schema writes them as maxLength in characters, so non-ASCII text over the limit passes the schema and is refused by the reader."},
  {"file": "crates/ekr/src/host.rs", "line": 93, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "Stripping the rustdoc removed the only statement of the six fixed texts the host authority check requires, leaving them as unconstrained strings."},
  {"file": "docs/cli.md", "line": 236, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The CLI page lists three gaps while the printed description names eight, including both where the schema refuses documents the reader accepts."},
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 242, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "IssueId and ObservationId are described as minted by ekr mint, which has no kind for either."},
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 36, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The description says the readers ignore a tag on a scalar, but they ignore a local tag on any mapping or sequence too, which the schema refuses."},
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 36, "category": "property", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "uniqueItems compares projected values while unique_set compares text, so [true, True] and [1, 1.0] pass the reader and fail the schema, the reverse of the named gap."}
]
```
