---
format: aep.planning-md/1
id: review-result:p1-15-schema-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-15 schema unit
relations:
- reviews: story:machine-readable-document-schemas
revision: 1
---
Adversary pass 1 on unit p1-15-schema (wave p1-15), `aep-drive:adversary`, against `b0698d6`.

Owners: 7 findings, 7 implementor (introduced).

Verdict: NEEDS-CHANGE (1 warning), CONFIRMED (2 warnings, 1 note), INFEASIBLE (3 notes). cases: executed 199→208, red 6.

Cases in `crates/ekr/tests/adversary_p1_15_schema.rs` (9; 6 red): empty operations, the v1 document limits, bare scalars read as text, a tag on a scalar, integers past their type, a NaN float; 3 green: null collections, seed fields left out, host edge cases.

Routing (coordinator): all seven back to the implementor.

```findings
[
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 96, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The transaction schema accepts operations: [] which the reader refuses as empty_operations."},
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 23, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The fixed DOCUMENT_V1_LIMITS (256 operations, 1024 evidence entries, 65536-byte strings) are neither in the schema nor among the gaps its description names."},
  {"file": "crates/ekr-ontology/src/value.rs", "line": 198, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "String fields read a bare YAML scalar like 12.50 or true as text, and the schema refuses it after JSON conversion."},
  {"file": "crates/ekr-kernel/src/schema.rs", "line": 23, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The transaction reader ignores a tag on a scalar such as !AgentId <id>, while the schema refuses the one-key object that tag becomes."},
  {"file": "crates/ekr-core/src/time.rs", "line": 60, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "int64 and uint32 formats are annotation-only, so values one past their range pass the schema and fail the reader."},
  {"file": "crates/ekr-ontology/src/value.rs", "line": 196, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "A .nan Float the reader keeps becomes null in JSON and the schema refuses it."},
  {"file": "crates/ekr-graph/src/assertion.rs", "line": 1, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Printed schema descriptions carry maintainer rustdoc into an agent-facing artifact."}
]
```
