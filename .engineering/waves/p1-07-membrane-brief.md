# Implementor brief — p1-transaction-membrane-repair

Read the installed aep-drive implementor charter, AGENTS.md, COORDINATOR.md and the complete
`story:p1-transaction-membrane-repair` artifact. Follow the acceptance and scope recorded there.
Read the archived probe in `.engineering/reviews/p1-baseline-209dd5e/probe/src/main.rs`.

Your managed tree id is `ekr-p1-07-membrane`, branch `codex/ekr-p1-07-membrane`.
Tree: `<worktrees>/ekr-p1-07-membrane`.
Build: `<cache>/b10x-target/ekr-p1-07-membrane`.
Scratch: `<cache>/ekr-completion-20260922/membrane`.
Acquire a lease named `codex-ekr-p1-07-implementor`; renew during long work and release on handoff.

Write code and tests only on the declared kernel scope. No planning-store or shared ESS writes.
The coordinator owns those. Record red test output before implementation, then package tests,
package clippy with warnings denied and formatting, using the assigned target directory.
All seven defects must become named refusals without making valid controls fail. Extend to
the underlying class: surviving assertion references, endpoint compatibility, competing
property/lifecycle writes, and applicable unsupported node constraints.

You may replace the two existing fresh-type and valid-merge positive expectations with explicit
P1 unsupported-operation expectations; the coordinator has already corrected that acceptance.
Preserve their distinct identity and malformed-operation tests. Preserve creation/deletion
cancellation semantics, including a genuinely successful permutation control.

Do not invent support for opaque constraint strings, schema evolution or merge application.
Report ambiguity with a concrete counterexample. Use runtime paths for any new source-reading test.
Do not change public transaction shapes, the store, graph, ontology crate or workspace manifests.
If resolving the acceptance requires that wider surface, report it before editing.

Do not commit or publish; return the tested diff for the coordinator to commit under the bot.
Return the required six-line unit report header, commands with executed counts and exit statuses,
the complete changed-path list, and all paths written outside your tree.
