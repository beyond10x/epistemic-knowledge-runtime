---
format: aep.planning-md/2
id: initiative:epistemic-knowledge-runtime
kind: initiative
status: active
title: Build the Epistemic Knowledge Runtime
summary: The design document as a multi-crate Rust workspace, in phases P0–P7, superseding company-brain and org-brain.
relations:
- serves: vision:o2
- serves: vision:o5
- serves: vision:o6
revision: 4
---
## Context

`docs/epistemic-knowledge-runtime-design.md` specifies a self-maintaining epistemic knowledge
runtime: observations become typed, provenance-aware, validated, revisable knowledge through a
governed loop, with a canonical core surrounded by an incubation forest. It is the third
generation after `company-brain` (v1) and `org-brain` (v2). `docs/predecessors.md` lists the fifteen
capabilities those two proved necessary and the design does not yet say, and the import policy for
their data. `docs/roadmap.md` orders the work into phases P0–P7 and records decisions D1–D3.

## Outcome

A Cargo workspace, one crate per domain, that runs the epistemic loop end to end: sources enter as
immutable observations; interpretation is a bounded, receipted model session; integration is a
validated transaction; the ontology evolves from evidence; maintenance reclaims what is no longer
needed; a person sees what the runtime is unsure about and answers it. The v2 instance runs on it
with nothing lost that the predecessors held.

## Phases

One epic per phase, each closed by the exit evidence `docs/roadmap.md` § 4 names:
P0 bootstrap and design v3.1 → P1 kernel, ontology, canonical core → P2 observation layer →
P3 incubation, interpretation, integration → P4 operator surface → P5 frontier, schema evolution,
scheduler ∥ P6 maintenance, observability → P7 migration and cut-over.

## Exit

Record counts reconcile per kind against v1 and against `org-brain/import-report.json`; every open
item on the v2 attention page has a counterpart on the v3 one; seven days unattended produce the
same evidence a CLI round produces.

## Decisions

Three ADRs `decides` this initiative: own kernel on eventlog persistence (D1), supersede the v2
engine and instance (D2), crate prefix `ekr-` (D3). Reopening one is an ADR, not an edit.

## Completion instruction, 2026-09-22

The operator approved the consolidated completion plan and instructed implementation through P7, including live cutover, a verified 0.1.0 release and seven days unattended. `.engineering/waves/COMPLETION-2026-09-22.md` records the sequence and recovery paths. Existing stores must be preserved; an unverifiable migration stops rather than inventing history or replacing it with a snapshot cutover. The membrane is repaired before durable application. Independent reviews occur after membrane repair and at P1 exit.
