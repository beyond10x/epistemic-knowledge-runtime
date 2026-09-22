# P1-07 correction — ontology decoder

Apply the installed aep-drive:implementor charter. This is a sequential follow-up unit under the
approved completion plan; the independent checkpoint is recorded before dispatch. Read
AGENTS.md, COORDINATOR.md, story:refuse-discarded-ontology-semantics, and
review-result:p1-07-independent-checkpoint in the integration store.

Reuse managed tree ekr-p1-07-membrane, branch codex/ekr-p1-07-membrane, after its coordinator
fast-forward to the integration checkpoint. Build only in <cache>/b10x-target/ekr-p1-07-membrane;
scratch only <cache>/ekr-completion-20260922/membrane/decoder. Acquire your own lease and
release it at handback. Two build jobs; pause new compilation below 10 GB free.

Reproduce the serialized sets defect first, keeping a supported serialized lifecycle control.
Then refuse unknown semantic fields at the ontology decode boundary. Do not implement templates.
Review the ontology's nested input structs for the same information-loss class; meaningful
unknown-member cases belong in ontology_load.rs. Add a kernel serialized-input control if needed
to establish that accepted ordinary lifecycle invocation remains valid. No test weakening.
Scope is exactly the story's ontology source files, ontology_load.rs and kernel validation.rs.
Coordinator owns AEP and systems files; hand back a patch if a comment outside scope needs repair.

Confirm the scope with cited/inferred rows. Run package tests for ontology and kernel, package
clippy, and rustfmt; the coordinator owns the full gate. Retain red and green outputs. Report
commands, statuses, executed counts, changed paths, outside paths, and any decision needed.
Do not commit, publish, or edit planning. Explicitly report if unknown-field strictness conflicts
with an existing supported fixture rather than weakening it to pass.
