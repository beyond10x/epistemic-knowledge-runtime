# Wave extract-07b — resume the consumer extraction path

Skill: aep:implementing 0.19.1, wave mode. Coordinator: Codex.
Status: implementing. The operator explicitly requested continuation toward 0.0.27 on 2026-10-02.
This resumes the accepted `release-plan:next-waves-2026-10-01` and the retained extract-07
units; it does not restart their work. All running committed code is Rust with clap derive.

Base: `4832d892e7586f6cc7980fa6f99634e3c9b71356`, remote main and tag 0.0.26.
The published release and correctness run 36969172579 were read on resume; that run succeeded
on exactly this commit. The correct-07 artifacts still say active and are reconciled from that
evidence before dependent work starts.

Integration: branch `wave/extract-07b`, managed id `ekr-extract-07b`.
All EKR worktree paths below are under `<state>/worktree/trees/b10x/epistemic-knowledge-runtime/`.
Build directories are under `<home>/.cache/b10x-target/`; scratch roots under
`<home>/.cache/ekr-extract-07b/`. These aliases keep machine paths out of public source.

| Unit | Artifacts | Branch / managed id | Build directory | Scratch | Stage |
|---|---|---|---|---|---|
| A | story:evidence-attaches-to-a-held-assertion | impl/evidence-attachment / ekr-x7-a | ekr-x7b-a | a | recovered source 157b89a6; reconciling main |
| W | task:validate-cost-flat-with-store-size | impl/validate-cost-flat / ekr-x7-w | ekr-x7b-w | w | main reconciled at 099821581; regression work |
| consumer | K: story:sdk-store-checks; O: task:ocel-export-prints-its-counts and task:ocel-times-events-by-a-named-property; Q: task:quality-counts-seed-evidence-and-constrained-types; N: task:code-names-matches-whole-words; F: task:fact-quality-states-the-empty-interval | impl/x7b-consumer / ekr-x7b-consumer | ekr-x7b-consumer | consumer | serial changes; F before K, Q after attachment integration; spec/test preparation while A/W build |
| coordinator | integration, planning, conformance synthesis | wave/extract-07b / ekr-extract-07b | ekr-extract-07b | coordinator | opening |

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

Pending.
