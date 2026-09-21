# Roadmap — Epistemic Knowledge Runtime

**Status:** top-level execution plan, drafted 2026-09-21, recorded in the AEP planning store as
`initiative:epistemic-knowledge-runtime` with one epic per phase and three ADRs.
**Scope:** the order in which `docs/epistemic-knowledge-runtime-design.md` is built as a multi-crate
Rust workspace, what each phase must deliver, and the evidence that closes it.
**Companion:** `docs/predecessors.md` — what v1 (`company-brain`) and v2 (`org-brain`) do that the
design does not yet say, and how their data enters this runtime.

---

## 1. What this repository is

The third generation of the organisation-memory line:

| generation | repository | shape | fate |
|---|---|---|---|
| v1 | `company-brain` | Python scripts over a curated markdown store with an event-sourced `observations.jsonl` | retired into v2 on 2026-09-03 (`import-report.json`) |
| v2 | `org-brain` engine (`beyond10x/org-brain` 0.5.0) + `org-brain` instance | Rust engine on the entity-runtime kernel, connectors and AEP; typed definitions, a signal ledger, bounded model extraction, attention page | superseded by this runtime (decision D2) |
| v3 | this repository | the design document: kernel, ontology, canonical core, incubation forest, observation layer, frontier, maintenance | built here |

The design document is the architecture. This roadmap adds what the design leaves open — storage,
CLI, operator surface, outputs, cost — by carrying the capabilities v1 and v2 proved necessary, and
it orders the work so that each phase leaves a runnable, tested system.

## 2. Decisions taken for this roadmap

Each is recorded as an ADR in P0. Reopening one is a design change, not a task.

### D1 — own kernel, eventlog persistence

The kernel (design § 9), the ontology (§ 11–12), assertions (§ 13), the canonical/transient membrane
(§ 6.1, § 23) and schema evolution as a transaction (§ 26, § 51) are implemented as this workspace's own
crates. Persistence goes through `beyond10x/eventlog` (append-only log, folds, snapshots, redaction,
SQLite/PostgreSQL/file providers): a committed revision root (§ 34) is an event, the canonical graph is
a fold, and § 60 deletion requests use eventlog's redaction contract.

Why not entity-runtime, as v2 did. Its kernel evaluates `definition + instance + operation → Decision`
over definitions that are authored YAML validated at registration time (`entity-runtime/README.md`).
It has no graph roots, no canonical/transient distinction, no bitemporal assertion, no
schema-as-runtime-transaction and no assertion primitive. v2 stored whole-entity records for that
reason (predecessors § 6, conflict 9). Building § 13/§ 22/§ 26 on top of it means fighting the
substrate. entity-runtime remains in the picture only where AEP governance requires it (driver maps,
planning store).

### D2 — this runtime supersedes the v2 engine and instance

One runtime, one store. The v2 engine's crates are lift sources — adapters, view templates, cost
accounting, the request/receipt protocol, the MCP facade, the adversary test cases — not dependencies.
The v2 instance's definitions become the local schema of a transient root (predecessors § 6, policy 5)
and its store is imported, never wrapped.

### D3 — names

Crate prefix `ekr-`, binary `ekr`. The repository keeps the name `epistemic-knowledge-runtime`.
`atlas/AGENTS.md:443` prefers a plain lowercase noun for a repository; that is flagged for the operator
and not applied here.

## 3. Workspace layout

Mapping of the design's module sketch (§ 68) onto crates. One crate per domain, one ESS domain per
crate under `systems/ekr/` (the v2 convention; `ess specify validate` runs in the gate).

| crate | design sections | owns |
|---|---|---|
| `ekr-core` | § 6.4, § 10, § 57 | identity newtypes, content hashing, canonical encoding; depends on nothing else in the workspace |
| `ekr-kernel` | § 6, § 9, § 19–20, § 34, § 71–72 | reference rules, `GraphTransaction` → validators → `ValidatedTransaction`, commit, revision roots, optimistic concurrency, seed, explain; depends on core, ontology, graph, store |
| `ekr-ontology` | § 11–12, § 26, § 50–52 | node/edge types, property definitions, typed values, per-type lifecycles and named operations (A13), schema versions, compatibility mappings, migration plans |
| `ekr-graph` | § 13–14, § 17, § 21–23, § 36 | assertions (bitemporal), validation state, canonical graph, transient graph, `CanonicalRef`/`TransientRef` |
| `ekr-store` | § 34, § 37, § 57 | eventlog-backed persistence, storage classes, content addressing |
| `ekr-observe` | § 15–16, § 54–56 | observations, evidence, `SourceAdapter`, checkpoints, idempotency, poll health (A8), blobs (A12), credential redaction (A6) |
| `ekr-adapters` | § 54 | Slack, GitLab, Jira, Confluence, GitHub through connectors; adapter declarations lifted from v2 `adapters/*.yaml` |
| `ekr-import` | § 15, § 22 | v1 and v2 importers (raw in P2, curated in P7) |
| `ekr-incubate` | § 22–25, § 40, § 53 | incubation forest, transient roots with local schemas, root utility metadata |
| `ekr-interpret` | § 18, § 24, § 48–49 | agent contract, interpretation session protocol (A5), trust profiles, consensus |
| `ekr-integrate` | § 27–28, § 44–46 | integration plans, mappings, resolver with explicit merge/split transactions, contradiction handling |
| `ekr-views` | § 47, § 62, A1–A4, A15 | attention queue and numbered answers, approvals for outward writes, obligations and horizons, rendered views, query scopes, explain chain |
| `ekr-mcp` | § 70, A14 | read-only MCP tools for agents; record text is untrusted evidence |
| `ekr-frontier` | § 30–32 | knowledge frontier, tasks, prioritisation, budgets |
| `ekr-schema` | § 25–26, § 50, § 64 | schema discovery over transient roots, `SchemaProposal`, risk classes, gates |
| `ekr-runtime` | § 33, § 43, § 70, A7, A11 | the epistemic loop as `ekr` verbs ordered by an AEP driver map; timer and continuous modes; cost accounting with a stop condition |
| `ekr-maintain` | § 35–43, § 60, § 66–67 | retention, consolidation, decay, GC with quarantine and grace, deletion requests |
| `ekr-metrics` | § 61, § 75, A9 | epistemic health counters |
| `ekr` | — | the binary: clap derive over every verb |
| `xtask` | — | development and migration utilities |

Rules that hold across crates: no crate below `ekr-graph` names a domain concept (no `Person`, no
`Project`); agents never hold a writer to canonical state; only `ekr-kernel` constructs a
`ValidatedTransaction`.

Crate dependencies are acyclic: `ekr-core` ← `ekr-ontology` ← `ekr-graph` ← `ekr-store` ←
`ekr-kernel` ← `ekr`. The design's § 68 sketch is a module tree inside one crate, where references
run both ways; as separate crates the primitives every layer names must sit below the graph and the
transaction boundary that reads the graph must sit above it. The `ekr.kernel` ESS domain is
therefore implemented by two crates, `ekr-core` (its types) and `ekr-kernel` (its commands). Found
in the P1 decomposition review (`review-result:p1-design-round-2`, 2026-09-21).

## 4. Phases

Each phase is one AEP epic. "Carries" lists the predecessor capabilities (predecessors § 2, ids A1–A15)
the phase must not lose. "Exit" is the evidence that closes the epic; a phase without its exit
evidence is open, whatever its stories say.

### P0 — Bootstrap and design v3.1

Outcome: a repository that can accept governed work, and a design that says what the predecessors
proved necessary.

- `git init`; Cargo workspace with `xtask` only; `rust-version`, `[workspace.lints]` with
  `unsafe_code = "forbid"` and `missing_docs = "warn"` (v2 engine convention).
- `AGENTS.md` with a `## Serves` section naming O2, O5, O6 by id from `atlas/ROADMAP.md`;
  `README.md`; `CHANGELOG.md`; LICENSE (Apache-2.0); `.github/workflows/shared-gates.yml`;
  `Taskfile.yml` with the gate.
- AEP planning store, from scratch: `.engineering/project.yaml` written by hand and pinned to the
  published AEP `0.55.0` tag; three vision projections (O2, O5, O6); one initiative; one epic per
  phase P0–P7; this file's decisions as three ADR artifacts.
- Design amendments 81–87 appended to `docs/epistemic-knowledge-runtime-design.md`: operator surface
  (A1), projections and views (A2), invariant 6.11 on outward writes (A3), obligations and horizons
  (A4), interpretation session contract (A5), `ObservationContent::Blob` (A12), per-type lifecycles
  and operations (A13).

Exit: `task check` green on the empty workspace; `aep plan artifact validate` green; amendments
81–87 present; the three ADRs accepted.

Pending outside this repository, not part of P0's exit: `b10x.docs.yaml` (written by
`atlas docs reconcile`, not by hand); Gates enrollment of this repository (administrator action; the
workflow file is in place); creation of the GitHub repository through the bot route.

### P1 — Kernel, ontology, canonical core

Outcome: a seed loads, a typed transaction validates and commits, the revision lineage is
reproducible.

- Design § 8–21, § 34, § 70–72.
- Deterministic validators 1–7 of § 20 (structural, reference, type, cardinality, ontology
  constraint, provenance, authorization); validators 8–10 arrive with the subsystems that need them.
- Per-type lifecycles and named operations on `NodeType` (A13), enforced by the ontology-constraint
  validator.
- eventlog-backed store with the SQLite and file providers.
- `ekr seed`, `ekr propose`, `ekr validate`, `ekr commit`, `ekr snapshot`, `ekr explain`.

Carries: A13.

Exit: property tests show that no dangling reference can commit and that `Canonical → Transient` is
unrepresentable at the type level; replay from the seed reproduces the root hash; the § 65 retraction
example runs.

### P2 — Observation layer and adapters

Outcome: sources enter as immutable, content-addressed observations, twice-safe, with health.

- Design § 15–16, § 54–57.
- `SourceAdapter` over connectors; checkpoints per source unit; idempotency at observation and
  integration layers; poll health with `checked_through`, `attempt | complete | partial | failed`, and
  the rule that a successful poll proves only its window (A8).
- Credential redaction before any model input, PII/secret gates (A6); `Blob(ContentHash)` for
  attachments (A12).
- Adapters: Slack, GitLab, Jira, Confluence, GitHub, lifted from v2 `adapters/*.yaml`.
- Raw importers: v1 `knowledge/raw/**/*.jsonl` and v2 `raw/` become observations.

Carries: A6, A8, A12.

Exit: the same Slack delta ingested twice produces zero new observations; a coverage report prints
declared denominators; v1 and v2 raw imports reconcile against source file counts.

### P3 — Incubation, interpretation, integration

Outcome: the § 63 flow runs end to end on a fixture.

- Design § 22–29, § 44–46, § 52–53.
- Transient roots with their own stable schemas; root utility metadata.
- Interpretation session contract (A5): immutable request bytes with digests, one disposition per
  signal including no-ops with a `basis`, receipts, a bounded number of visits — the shape of v2's
  `brain.extraction-request/4` / `receipt/4`.
- Resolver (A10): typed references, explicit merge and split transactions with lineage; no string
  matching.
- Integration plans with versioned mappings; contradiction produces `Disputed`, never a silent loser.

Carries: A5, A10.

Exit: the § 63 Slack-to-canonical fixture commits the proposal and, on the confirming message, the
fact; a parked root integrates after a mapping is added; a wrong merge is reverted by a split with
provenance intact.

### P4 — Operator surface and projections

Outcome: a person can see what the runtime is unsure about, answer it, and read what it knows.

- Attention queue with numbered items; an answer becomes `HumanStatement` evidence and a transaction
  (A1).
- Approvals: every outward write is an approved transaction (A3); obligations with an external clock
  and horizons past which a fact reads `?` (A4).
- Rendered views with templates lifted from v2 `views/*.j2` (A2); a human work backlog projection
  distinct from the frontier (A15).
- Query scopes of § 47; the § 62 explain chain.
- `ekr-mcp` read tools, lifted from v2 instance `src/mcp.rs` (A14).

Carries: A1, A2, A3, A4, A14, A15.

Exit: the attention page renders from canonical plus disputed state; an answer commits a transaction;
`ekr explain` prints the § 62 chain for a canonical assertion; two renders of the same revision are
byte-identical.

### P5 — Frontier, schema evolution, scheduler

Outcome: the graph drives its own observation, the ontology grows from evidence, and the loop runs
unattended within a budget.

- Design § 25–26, § 30–33, § 48–50.
- Frontier tasks and prioritisation; schema discovery over transient roots producing
  `SchemaProposal` with risk class, migration plan and stronger gates.
- The epistemic loop as `ekr` verbs ordered by an AEP driver map, in both timer and continuous
  modes with independent per-source collectors (A11).
- Cost accounting: micro-USD, daily boundaries, reservations, a stop condition (A7).

Carries: A7, A11.

Exit: the § 64 ontology-discovery fixture integrates parked facts after schema v(n+1) is approved; a
day's spend stops at the configured limit and the stop is visible in status.

### P6 — Maintenance and observability

Outcome: growth without maintenance is impossible by construction.

- Design § 35–43, § 60–61, § 75.
- Retention classes, consolidation, decay scoring, GC with quarantine and grace period; deletion
  requests through eventlog redaction with dependent assertions moved to `Disputed` or retracted.
- Epistemic health metrics; lessons and gates as ordinary canonical nodes with recurrence counted (A9).

Carries: A9.

Exit: the § 66 expiry and § 67 consolidation fixtures run; an erasure request leaves dependents
`Disputed` or retracted and the bytes gone; the metrics surface lists the § 61 counters.

### P7 — Migration and cut-over

Outcome: the v2 instance runs on this runtime with nothing lost that the predecessors held.

- Curated v1 records (including the kinds the v1→v2 import skipped), the v2 `store/`, and the v2
  infrastructure mirror enter transient roots `v1-workday`, `v2-store` and `infrastructure` with
  `origin: inherited`; re-observation promotes; nothing inherited is `Accepted` without evidence.
- v2 `source-cursor` records become checkpoints; v2 per-subject history becomes provenance blobs.
- systemd units; release 0.1.0 through the bot route.

Exit: record counts reconcile per kind against v1 and against `import-report.json`; every open item on
the v2 attention page has a counterpart on the v3 one; seven days unattended produce the same evidence
a CLI round produces.

## 5. Ordering

```text
P0 ─▶ P1 ─▶ P2 ─▶ P3 ─▶ P4 ─▶ P7
                     └──▶ P5 ┐
                     └──▶ P6 ┘ (parallel, both before P7)
```

P5 and P6 depend on P3 only. P4 can start once P3's transient roots exist. P7 waits for everything.

## 6. Conventions this repository follows

- Organisation rules live in `atlas/AGENTS.md` and `beyond10x/AGENTS.md` and are not restated here:
  anything that runs is Rust with clap derive; every GitHub write is the bot's; releases finish only
  after the repository's own checks and artifacts are verified.
- Work is planned in `.engineering/planning/` through `aep plan artifact`; records are never
  hand-edited.
- Each crate declares its ESS domain under `systems/ekr/`; the gate validates it.
- Design documents carry the full reasoning; this roadmap carries the order and the evidence.

## 7. Out of scope

Carried by neither this runtime nor its instance repository: v1's story backlog and CHANGELOG (a
development process), v1's telephony probes (`acd-health` and siblings), v2's adoption screen
(`src/screen.rs`). See predecessors § 4.
