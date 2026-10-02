---
format: aep.planning-md/3
id: review-result:adversary-correct-07-m-pass-1
kind: review-result
status: active
title: Adversary, correct-07 unit M, pass 1
relations:
- reviews: task:migrate-reads-a-current-store
revision: 1
---
CONFIRMED

Adversary pass 1 on unit M (`impl/migrate-added-evidence` at `d9db76b6`; cases committed as `96ae44aa`, three red and ignored). `cargo test -p ekr-kernel`: 497 passed, 5 ignored; `migrate_cli` and the adversary file pass; the consumer's minimal and full cases migrate.

Held, on both providers: evidence in rejected or stale transactions; one payload added twice or also in the seed (logged once, exact accounting); evidence cited by nothing; validation against an earlier revision; checkpoints; resolved commit preparations; a raise after the commit; every revision's roots, the head graph and evidence equal to the source's; `stored_at` and class unchanged; `agrees()` still runs.

```findings
- file: crates/ekr-kernel/src/migrate.rs
  line: 274
  category: boundary
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "an added payload first stored below Provenance is re-published with its commit at that class, so the destination's replay of the commit refuses required-object-integrity; only a direct ekr-store put reaches it"
- file: crates/ekr/tests/migrate_cli.rs
  line: 127
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the snapshot comparison reads snapshot["graph"][field], which is null for every field, so it cannot catch a migration that drops graph content
- file: crates/ekr-kernel/src/migrate.rs
  line: 92
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: a validated transaction whose evidence payload equals the migration-started marker bytes is refused at commit as migrate-incomplete
- file: docs/epistemic-knowledge-runtime-design.md
  line: 4914
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: design section 100.3 and the migrate.rs module doc still say every non-record object is carried, while added evidence payloads now ride with their commit
```
