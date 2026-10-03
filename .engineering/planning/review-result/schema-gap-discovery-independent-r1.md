---
format: aep.planning-md/3
id: review-result:schema-gap-discovery-independent-r1
kind: review-result
status: active
title: Schema gap discovery independent review r1
relations:
- reviews: story:propose-better-vocabulary
revision: 1
---
needs-revision

Owners: 1 coordinator/implementor finding; 0 delegated implementor findings.

```findings
- file: crates/ekr-kernel/src/schema_gaps.rs
  line: 49
  category: correctness
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: >
    Discovery reports import-time blockers against the current schema.
    Import an interpretation with unknown Project, then commit a canonical
    Project declaration and call discover_schema_gaps. The request names the
    newer base_schema but still includes UnknownType Project because discovery
    never evaluates held.blockers against read.graph.ontology.
    Derive current pending gaps against the verified current schema while
    preserving immutable historical records and deterministic identities.
    Cover disappearing UnknownType and newly exposed UnknownProperty gaps.
    The new knowledge_retention test never advances the schema. Add both-provider
    coverage after schema advancement, including reopen/full replay and unchanged
    canonical root/publication assertions around discovery.
```

Otherwise, source inspection supports deterministic ordering, retained document/source verification, preservation of infrastructure errors, and no canonical mutation.

Limitations: read-only source review; no compilation or test execution. Scope is E’s discovery slice and `DiscoverSchemaGaps`, not full E or PR acceptance.
