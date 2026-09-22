unit: V2 activation contract correction, findings F1–F4
verdict: blocked — F3/F4 corrected; F1/F2 retain measured ESS representation gaps
cases: runtime executed 0→0; generated partial kernel obligations 30→31
origin: coordinator-owned declaration correction; no production implementation finding
wrote-outside-worktree: assigned writer-contract-preparation/correction scratch only
needs-coordinator: independent review and bounded ESS capability decision before adoption

The candidate is **not activation-ready**, despite compiler validation succeeding. Its inherited Commit wrong-state declaration still contradicts the required successful retry, and no truthful document-only Seed preservation branch can be expressed with the measured compiler. Both places are explicitly marked `UNMAPPED`; no generated suite was hand-edited to conceal them. F3 and F4, all four DESIGN decisions, typed responses, emitted-field ownership and future acceptance were corrected in a new scratch copy. Original proposal, patch, baseline and review evidence remain unchanged.

Owners: coordinator — decisions, ESS adoption, repository/AEP edits, integration and publication; correction worker — scratch proposal, patches and measurements; independent reviewer — correction disposition. Runtime agreement remains unexecuted.

## Deliverables and findings

`activation.patch` is the complete six-file draft against the original preparation baseline `a2593122b04c3b422cf685ba7fc0db2444e3b3a5`; `correction.patch` is the four-file increment against the independently reviewed proposal. `proposal/` contains the exact candidate, `coordination.md` updates future acceptance, and `capability-gaps.md` names the two remaining language requirements. No patch was applied to a repository.

| Finding | Correction and exact candidate location | Disposition |
| --- | --- | --- |
| F1 Commit retry | DESIGN §91.6 at line3867: same public transaction_id resolves retained CommitReceiptV1/result before new occurrence/time, including after head advance; Committed is silent success and Proposed/Rejected/Stale refuse. Kernel response declared at line947; explicit `UNMAPPED` at line983. Future public-handler/no-write/refusal controls added. | DESIGN/acceptance settled; ESS partition blocked by measured 0.28 refusal. The partial suite's Committed refusal is still a known contradiction, not acceptance. |
| F2 Seed retry | DESIGN §91.2 at line3704: compare full parsed Seed2 input plus actual BootstrapContext and trusted host AuthorityStateV1 anchor before allocating occurrence/time; return original SeedResultV1/Root0/committed_at after later head advance. Different logical input/context/anchor refuses; whitespace alone does not differ. Kernel AlreadySeeded condition narrowed and SeedResultV1 response declared at line834; explicit `UNMAPPED` at line854. | DESIGN and mismatching-seed refusal corrected; silent ESS success cannot bind retained Revision from document-only input. |
| F3 Object event dispatch | DESIGN §§89/90/91.6, lines3543/3606/3893; store.yaml lines81 and100. New metadata-only ObjectStored is backend schema2; original inline ObjectStored/schema1 is frozen verification/migration-only; unchanged ObjectRetentionRaised remains schema1. Unknown/mixed dispatch and missing/mismatched bindings refuse. | Contract corrected; actual source/provider readback and frozen-format tests remain future acceptance. |
| F4 absent Validate basis | DESIGN §91.5 line3822 and kernel.yaml line929. RevisionNotFound on absent `against`, transaction stays Proposed, no validation/rejection event, receipt, object or issue. Older existing revision remains a valid basis; later Commit may be Stale. | Declared and re-synthesized. The new generated external-refusal scenario does not by itself arrange/prove the real retained proposal or no-write behavior; authored shared-handler acceptance is explicitly required. |

The exact transaction-document byte-retention contract is unchanged. Seed's typed-input retry comparison does not reinterpret transaction whitespace, Float spelling or document hashes. No new CLI verb, caller-controlled authority hash/identity, duplicate occurrence, replacement expected-result map or broad wrong-state success was introduced.

Every emitted kernel event field now has its actual command-input source or `{generated: true}` ownership. The real kernel owns derived IDs, hashes, actor, counts and status from retained inputs/trusted context; generated ownership is not permission for a target to return fixture expectations. All eight emitted event families are covered: Seeded, TransactionProposed, TransactionValidated, TransactionRejected, TransactionStale, RevisionCommitted, SnapshotTaken and Explained. ESS0.28's ess/4+ field-completeness validator checks omissions. Seed/Commit response schemas require the typed retained record; these declarations alone do not compare silent returned values, which remain runtime acceptance.

## Released compiler and source provenance

Only the isolated released **ESS0.28.0** executable was invoked. SHA-256:

```text
3c003e662ce6c2b28af16ada748b2c707abf546d12c05516294217c582afe09c
```

The locally available annotated release resolves to source commit `68581d70bb47a048cd399e55c68f225b80303977`. The read-only checkout is `16aa8c7617214420d7d7f2108d0a896a5ed14eb0`; `git diff` from the release commit showed no differences in the four inspected command/subject modules. No executable was built from that checkout, installed globally or changed.

## Measured capability probes

Every refusal below was observed with the checksum-verified executable, not inferred only from source. Complete raw diagnostics are retained; some repeat the primary error with formatted ESS codes or show cascading missing-command/causation diagnostics after the command is rejected.

1. `commit-state-probe/`: explicit Validated committing branch and Committed `preserves: GraphTransaction`, both using real input `transaction_id`, beside legacy wrong_state. Validate exit1, ESS-COMMAND-004: **“explicit subject-state guards cannot also declare wrong_state precedence”**.
2. `commit-partition-probe/`: same successful branches, but Proposed/Rejected/Stale error declared external. Validate exit1, ESS-COMMAND-005: **“held state Proposed and input [] select 0 branches”**, likewise Rejected and Stale. An external fault is correctly not ordinary partition coverage.
3. `commit-default-error-probe/`: coordinator-requested natural ordinary default error, with no external, subject, guard or wrong_state flag; existing stale external branch retained. Validate exit1, ESS-COMMAND-003: **“every input-selected branch of a subject-state command must name an existing moves or updates subject”**. This is the bounded correction witness; attaching a fake mutating subject to an error is not a repair.
4. `seed-binding-probe/`: `preserves: Revision; instance: revision_id` under document-only Seed, with the same-input/context predicate. Validate exit1, ESS-COMMAND-001: **“acts on the instance named by revision_id, which is no input field of it”**. The diagnostic requires RevisionId while the actual declared input is SeedDocumentPath. Neither a path nor a fabricated caller identity is the retained revision selector.

Source explanation at release-equivalent paths: `crates/specify/ess-domain/src/command/subject_state.rs:103–118` disallows wrong_state and requires a subject on every ordinary state-selected branch. `command.rs:1460–1480` disallows a subject on an error. `command.rs:661–673` makes preserves identity come only from command input. Thus the ordinary default-refusal and response/retained-subject paths are concrete missing constructs, rather than an absent `preserves` parser feature.

Positive control `preserves-positive.yaml` uses a separate synthetic two-state Record, with a real input identity, a reachable Done state and a complete immediate view. Synthesis exits0, **4 generated, 0 refused, suite/11**. Its retained outcome establishes Done, snapshots the actual subject, invokes the same command with its captured ID, checks `expect_no_error`, `expect_subject_unchanged` and no events. This proves the compiler's preservation feature is available. It adds no product command and does not weaken EKR's Proposed/Rejected/Stale refusals. No generated control scenario was executed against a runtime.

## Compiler, suite and schema measurements

Commands use `<ess028>` for the isolated verified executable; `commands.md` records exact relative arguments and expected exits. Paths with machine-specific home directories occur only in private raw logs.

| command or artifact | measured result |
| --- | --- |
| Original reviewed proposal: specify validate | exit0, `ekr v1 — 6 file(s), valid` |
| Original reviewed proposal: conform synthesize | exit0; 30 generated, 0 authored/outside/refused |
| Partial correction: specify validate | exit0, `ekr v1 — 6 file(s), valid` |
| Partial correction: specify compile --format json | exit0; proposal-ir.json retained |
| Partial correction: conform synthesize, kernel, requested suite5 | exit0; 31 generated, 0 authored/outside/refused; suite/5 |
| Synthetic preservation positive control | exit0; 4 generated, 0 refused; automatically selected suite/11 |
| Four complete EKR retry probes above | each exit1 with the named refusal |
| Schema generation A and B | exit0 each, 169 compiler-owned public schema files each |
| Sequential schema subtree comparison | exit0, byte-identical `schema/` trees |
| Full activation patch: git apply --check against baseline copy | exit0 |
| Runtime/kernel/Cargo/full repository gate | not run; prohibited by this bounded preparation assignment |

The partial suite explicitly still contains `ekr.kernel.GraphTransaction/state/Committed/refuses/ekr.kernel.Commit`; it has no exact-seed retry outcome. **Its 31 generated obligations do not close F1/F2.** The added scenario is `ekr.kernel.Validate/outcome/revision-not-found`. Comments do not change compiled semantics, and a compiler-valid partial contract is not a corrected activation contract.

Schema169 is the previous167 plus the Seed and Commit response documents. No generated file or suite was hand-edited. Output ownership state directories are private compiler bookkeeping, excluded from the deterministic artifact comparison.

Observed full patch stat:

```text
 docs/epistemic-knowledge-runtime-design.md |  493 +++++++++++++++++++
 systems/ekr/system.yaml                    |    2
 systems/ekr/domains/graph.yaml             |  324 +++++++++++-
 systems/ekr/domains/kernel.yaml            |  729 +++++++++++++++++++++++++++-
 systems/ekr/domains/ontology.yaml          |  134 ++++-
 systems/ekr/domains/store.yaml             |   14 -
 6 files changed, 1600 insertions(+), 96 deletions(-)
```

The correction increment is four files, 121 insertions and6 deletions. Original frozen source sections remain unchanged; the DESIGN changes extend the dated prepared §§89–91. `coordination.md` is a separate updated acceptance document, not an automatic repository edit.

Artifact SHA-256 values:

```text
7884a971d8d5d3d371c03d03ffd1d0cb3503f229580f2c00917728a88d630c52  activation.patch
14f3a117664d2e7b60d3761bcee9972b544c1bad2c418beb3c1f02b4fffe0ef5  correction.patch
fb16e683c82998c68a6fd7f057509d9c6a583f4ad945b0c265996a519f65c318  kernel-suite.json
a1807f94b4e119a48d3b6c5a9392789632ff73f79685882d199d5cbd8537add5  proposal-ir.json
5439cddcd0795d769ea2e056ee7889bc4a1505b8aef13e27b33c0c914a4aada6  preserves-positive-suite.json
```

## Handback

Reconcile the two ESS capability gaps before adopting the full patch or dispatching the writer against it. The natural Commit default-error candidate and Seed missing-identity probe are preserved for bounded upstream scoping; this worker made no upstream source changes. The independent reviewer can assess F3/F4 and DESIGN/acceptance now, then review exact re-synthesis after a real retained-subject/refusal capability exists.

Every write is under the assigned `writer-contract-preparation/correction/` directory: copied proposal and four probes; synthetic positive-control source; compiler output/logs; generated IR/suites and two schema output roots; updated coordination/report/commands; full and incremental patches, stats and hashes; privacy log and file inventory. No repository, AEP, atomic implementation, global tool installation, build target or live store was changed. No managed worktree lease was required for scratch-only work. All invoked ESS processes completed.
