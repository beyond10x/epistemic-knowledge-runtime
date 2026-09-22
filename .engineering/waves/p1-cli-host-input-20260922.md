# Independent CLI host input preparation

This bounded task continues the operator-approved completion plan while the
durable writer's failure controls run. It is not dispatch of the blocked complete
CLI story. task:strict-cli-host-input decomposes story:ekr-cli; AEP explicitly
refuses typed scope on a task, so its parent story holds the file scope.

The source owner is the existing transaction_parser worker, resumed for this
independent task. Only crates/ekr/src/host.rs and crates/ekr/src/lib.rs belong to
it. The worker owns no kernel/store source, planning, specification, dependency
manifest, main binary or command handler. The durable implementor keeps its
existing source ownership. No source file has two implementors.

Managed id: ekr-cli-host-input-20260922.
Branch: codex/ekr-cli-host-input-20260922.
Tree: <worktrees>/ekr-cli-host-input-20260922.
Target: <cache>/b10x-target/ekr-cli-host-input-20260922.
Scratch: <cache>/ekr-completion-20260922/cli-host-input.
Coordinator lease: codex-ekr-cli-host-input-coordinator.
Worker lease: codex-ekr-cli-host-input-implementor.

Use an exclusive temporary-memory build directory with two compiler jobs,
incremental/debug output disabled, direct rustc and a task-owned TMPDIR. Before
compilation require 4 GiB persistent free, 8 GiB temporary-memory free and 8 GiB
MemAvailable. Preserve original failures and reports on persistent storage.

The typed model is the host portion of the reviewed R2 contract. The coordinator
activated that projection and copied the same specification into the durable
tree immediately. The CLI adds only existing workspace serde/time dependencies;
the lock diff lists those dependencies without changing package pins.

Acceptance is the task's strict JSON and calendar/decimal behavior. Tests must
call the production decoder, exercise escaped duplicate keys and decoded IDs,
and retain positive nested authority controls. Authority semantic checks still
belong to kernel opening; this decoder supplies no allow-all registry or actor.
No compatibility claim is made for clock sampling or commands not yet integrated.

Run the bounded library tests, strict Clippy and formatting; the coupled full
gate follows integration with the writer and real CLI. An independent review
attacks the host decoder before task closure. This task does not close P1, the
CLI story or any durable writer obligation.

Coordinator checks: the new decoder claims are explicitly unexecuted at dispatch;
no existing acceptance is removed; shared declarations are synchronized; source
readers must use runtime paths. This preparation uses existing reviewed contract
decisions and preserves the unapplied read-result portion of R2.
