# Wave extract-07b — resume the consumer extraction path

Skill: aep:implementing 0.19.1, wave mode. Coordinator: Codex.
Status: implementing. The operator explicitly requested continuation toward 0.0.27 on 2026-10-02.
This resumes the accepted `release-plan:next-waves-2026-10-01` and the retained extract-07
units; it does not restart their work. All running committed code is Rust with clap derive.

Base: `4832d892e7586f6cc7980fa6f99634e3c9b71356`, remote main and tag 0.0.26.
The published release and correctness run 36969172579 were read on resume; that run succeeded
on exactly this commit. The correct-07 artifacts were reconciled to implemented from that
evidence before dependent work started.

Integration: branch `wave/extract-07b`, managed id `ekr-extract-07b`.
All EKR worktree paths below are under `<state>/worktree/trees/b10x/epistemic-knowledge-runtime/`.
Build directories are under `<home>/.cache/b10x-target/`; scratch roots under
`<home>/.cache/ekr-extract-07b/`. These aliases keep machine paths out of public source.

| Unit | Artifacts | Branch / managed id | Build directory | Scratch | Stage |
|---|---|---|---|---|---|
| A | story:evidence-attaches-to-a-held-assertion | impl/evidence-attachment / ekr-x7-a | ekr-x7b-a | a | integrated at 0283e993e after decoder correction and second adversary pass |
| W | task:validate-cost-flat-with-store-size | impl/validate-cost-flat / ekr-x7-w | ekr-x7b-w | w | combined 58f53523b misses both independent timing bounds; profile analysis continues |
| consumer | K: story:sdk-store-checks; O: task:ocel-export-prints-its-counts and task:ocel-times-events-by-a-named-property; Q: task:quality-counts-seed-evidence-and-constrained-types; N: task:code-names-matches-whole-words; F: task:fact-quality-states-the-empty-interval | impl/x7b-consumer / ekr-x7b-consumer | ekr-x7b-consumer | consumer | integrated through d30befbbe with selector corrections and retained adversary cases; managed tree retired |
| F/K adversary | empty interval and judged SDK sample | review/x7b-fk / ekr-x7b-review-fk | ekr-x7b-review-fk | review-fk | F/K and subsequent O/N reviews integrated; managed tree retired |
| coordinator | integration, planning, conformance synthesis | wave/extract-07b / ekr-extract-07b | ekr-extract-07b | coordinator | all units integrated; combined correctness reconciliation and W acceptance remain |

No unit writes the planning store. Each implementation receives a file brief and is followed by
an independent adversary using the skill's role procedure. Codex dispatches generic collaboration
agents with those procedures because named plugin agent types are not exposed by this host.
No model override is requested; agents inherit the coordinator's model.

## Recovery and boundaries

- A's WIP holds tests, ESS and documentation but omits its source implementation. The recovery
  audit found the source in `<home>/.cache/ekr-x7-a-scratch/mirror/crates/`; only source differences
  may be restored. Copying the complete mirror would regress the committed tests and documents.
  There is no green implementation run to carry forward.
- W's saved profile text remains in `<home>/.cache/ekr-x7-w-scratch/`; raw perf data was deleted
  by the earlier session. Its full package gate did not complete. The new replay-history entry
  must preserve main's store replacement checks, not call the older synchronization guard.
- A and W reconcile with released main before implementation. Keep their published WIP commits;
  no forced reset, stash or branch deletion.
- A's quality evidence logic precedes Q. F's empty interval contract precedes K's SDK report.
  O and N share CLI dispatch/SDK read surfaces. The consumer artifacts use one worktree and run
  serially rather than treating their shared files as independent.
- Type packs, storage-08, new dependency releases and the open decision blockers remain deferred.
- Source publication ends at this repository's checks/release artifacts. Documentation delivery
  is asynchronous; no consumer promotion or Website work is included.

## Scheduling evidence

The initial `aep plan artifact waves --kind story --status active --format json` placed A and K
together and the already-shipped extraction verb separately. Its collisions were A versus that
verb on `cli/agent.rs`, `tests/agent_cli.rs`, conformance manifest/suite/provenance/baseline and
`docs/cli.md`; no unassessed stories or cycles were reported. The shipped item's status must be
corrected before the final scheduling snapshot. Task prose scopes are recorded before dispatch.
AEP 0.64.0 refused typed task scope: `scope` is a field of `story`, and a task inherits its story's
surface. These existing tasks decompose an epic, so there is no typed task-wave scheduling claim.
Their inspected overlap is handled by serialization. K's typed story scope was updated by the CLI.

The primary checkout's pre-existing untracked `.agents/` is left alone. All changes use the clean
managed integration checkout. The two old unit trees are retained intentionally for this recovery.
The prior session measured cold EKR build output around 21–25 GB. Start at most two builds and
stop starting builds below 12 GiB free. Each tree uses a distinct target directory and sccache;
debug info and incremental compilation may be disabled consistently to bound disposable output.
Exact measured preflight and command results are retained in coordinator scratch.

Repository gates pin ESS 0.36.0 and AEP 0.64.0. Use the existing cached binaries; the installed
ESS upgrade notification does not change these pins. Atlas tooling uses a clean exact managed
checkout `ekr-x7b-atlas` at `6574ab947f175f96a7b6ee80c63bd5ebda579b14` because primary Atlas was stale.
Connectors discovery offered only Confluence, Jira and GitLab. No GitHub adapter was available;
GitHub reads use the permitted read client and writes use `b10x-gates` as the bot.

## Commits and evidence

Opening/reconciliation commit; preserved recovery and unit commits; adversary cases and bounded
fix commits; integration merges; generated suites; closing evidence/store commit; bot pull request
and merge after the repository gate. Release 0.0.27 follows verified integration and the operator's
explicit continuation toward that release. No task is marked implemented from a WIP or an agent's
claim: each requires its own acceptance evidence and the combined gate.

## Adversary passes

- A pass 1: `review-result:adversary-extract-07b-a-pass-1`; duplicate attachment records
  silently collapsed during graph decoding. Fixed in bf48f7060, then pass 2
  `review-result:adversary-extract-07b-a-pass-2` found no residue, including equivalent decoded
  spellings. Both outcomes are recorded. The story remains active until the combined gate.
- W pass 1: `review-result:adversary-extract-07b-w-pass-1`; two added replay attacks passed,
  but the implementor's full-size SQLite run missed the per-verb acceptance bounds. The task stays
  active. Work continues from the fresh profile, including internal record sharing and selected
  state reads in session `settle`. That session change is disjoint from O's response/stderr edits;
  integration must preserve both.
- F/K pass 1: `review-result:adversary-extract-07b-fk-pass-1`; no finding, integrated with
  tests through c0ad8c535. The report retains the exact first-case and suite outputs.
- O/N pass 1: `review-result:adversary-extract-07b-on-pass-1`; a valid leading-dash event
  type works with the CLI's equals syntax but the SDK passes it as a separate argument and
  refuses it. The consumer implementor owns the correction; Q review proceeds separately.

## Intermediate retention

A's source and both review passes are published on `wave/extract-07b` through 2f37aacbc.
Managed finish, exact-id dry-run and exact-id GC removed `ekr-x7-a` with remote ancestor proof;
the coordinator scratch holds each result and A's small evidence logs remain retained.
Automatic approval review refused forced removal of its disposable build cache. The safer
non-forced removal succeeded after confirming no process used that exact task-owned directory;
the source tree and disposable build cache are gone, while small evidence logs remain.
The completed F/K review tree was reused for O/N after its clean reviewed commit. Consumer
source cadbc785a contains F/K/O/N/Q and the A integration, with generated suites still assigned
to the coordinator. All unit and review results precede the required combined gate.

Consumer integration d30befbbe includes the two verified SDK selector corrections, Q's adversary
cases and unchanged F/K review cases. The generated views suite now selects 78 scenarios
(47 generated plus 31 authored); both native providers execute every scenario with no failure,
error, unsupported or skipped result. The baseline raises answered_floor to 78 and no existing
step floor falls. Kernel and integration suites were regenerated for the common specification
digest; their selected inventories remain unchanged. Pinned specification, freshness and planning
checks pass. Raw output is retained in coordinator consumer-conformance.log and
consumer-integration-plan-spec.log. This remains unit/integration evidence, not the final wave gate.

Consumer and F/K–O/N review trees were subsequently finished and removed through exact-id
managed GC. Each had remote ancestor proof through the published integration branch;
coordinator scratch retains consumer-review-gc-dry.json and consumer-review-gc-apply.json.
Their disposable build directories were removed after their build slots were released, and their
small reports and logs remain retained. W and the coordinator keep their separate build slots.

The combined public-surface guard found new exported types and methods without explicit uses in
its source inventory. Coverage now exercises SDK wire serialization and deserialization against
real CLI replies, operation evidence manifests, attachment ordering and historical isolation, and
the pure named-timestamp OCEL projection against the runtime export. The guard itself is unchanged.
The corrected public-surface and story-contract checks pass; CLI documentation and agent checks
also pass. This is focused evidence, with the full combined gate still required after W lands.

The initial full consumer gate found two additional CLI documentation issues. The operations
section retained the old count; adversary_add_alias_l_cli caught it and passes after correction.
The attachment example also requires the preceding AddEvidence example, while its heading
claimed the seed alone supplied all ids. The page and reference now name that prerequisite.
The existing adversary case executes the printed preparation and requires the attachment to
validate, retaining its unresolved-reference assertions for every operation. Its transaction
helper now parses a complete envelope and refuses parse errors instead of silently losing the
evidence manifest. Sequential examples and documentation checks are rerun with this correction.

## Combined-gate reconciliation

The existing SQLite inode-reuse case failed on the machine's tmpfs temporary directory. A probe
using the exact same compiled binary passes when its temporary directory is on the cache's ext4
filesystem; coordinator inode-filesystem-exact-binary.log retains both observations. Subsequent
combined gates use that temporary directory. The case and its assertion are unchanged.

The core identity inventory found the declared AttachmentId lacked its projection carrier. Core
now exports it through the same id macro as the other declared identities, with serde and
rename-stability coverage. Runtime attachments remain keyed by assertion/evidence pairs; no wire
identity or mint kind was added. Its Canonical implementation changes rustc's incidental help list
in the transient-state membrane diagnostic. The coordinator compared the forbidden expressions,
error codes and primary messages before a narrow refresh and verified the membrane again with
overwrite disabled. The rejection cases remain intact.

The later SDK run found the empty-properties quality expectation missing the newly declared
constrained_types field. Its exact expected document now includes zero and checks the typed value;
omitted constrained_share and byte round-trip assertions remain intact on both providers.

W's confirmed-publication retirement and unchanged edge-index sharing are integrated at 58f53523b.
The default-size benchmark now enforces each independent timing bound directly, preserving its
aggregate checks. Its latest measurement remains red; the task records exact output and continues
from the retained profile. No artifact is closed on the unit gates or the earlier aggregate pass.
