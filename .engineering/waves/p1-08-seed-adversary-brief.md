# P1-08 — Seed adversary

Load the installed aep-drive:adversary charter. Read AGENTS.md, COORDINATOR.md,
story:kernel-validated-seed, the seed unit brief and the returned implementation report.
Attack the completed seed unit in its assigned tree; do not change production source,
ESS or planning. Add independent executable cases where they can establish a defect.
No commits, publication or cleanup. Acquire and release your own managed lease.

Tree/branch: `<worktrees>/ekr-p1-08-seed`, `codex/ekr-p1-08-seed`.
Build: `<cache>/b10x-target/ekr-p1-08-seed`, two compiler jobs.
Scratch/TMPDIR: `<cache>/ekr-completion-20260922/seed/adversary`.
Pause new compilation below the approved 10 GB free-space floor.

## Questions the review must answer

- Do initialization and restart both reach real kernel admission? Is every production route
  that could admit an unchecked seed gone? Inspect read helpers and compatibility constructors,
  not only the new happy path. Store-only authorities prove provider behavior only.
- Does an invalid seed write nothing to both object and revision streams? Exercise separately
  opened handles and first-writer competition through SQLite and file providers, plus reuse of
  a verified content object already retained at a weaker class. A no-write refusal cannot leave
  a losing seed object or retention mutation behind.
- Does replay verify the actual retained input, full ontology and bootstrap attribution?
  Missing authority, tampering, wrong ontology content behind the same schema ID, unsupported
  format and legacy raw graph input must refuse. A process-local accepted hash set is not proof.
- Are graph and ontology both genesis state? Check root filing, map identities, parent/revision,
  declared types, references, property multiplicity, initial lifecycle, canonical-only values,
  applicable unsupported constraints and preservation of every stored node/assertion field.
- Does the kernel attribute assertion acceptance itself? Caller verdicts, fake proposer,
  self-validation and retained context disagreement must refuse. Do not claim the future full
  capability registry is implemented by bootstrap identity comparisons.
- Are exact HumanStatement bytes retained and verified for all evidence, including uncited
  records? Missing/mismatched payloads and unsupported source variants must refuse. User-defined
  Record keys remain data while unknown semantic record fields refuse decoding.
- Do the original graph-document boundary assertions survive as kernel behavior? Read the
  implementor's old-to-new case map. No-authority tests may refuse earlier; transaction-forgery
  tests must still reach the transaction rule they claim to test.

Coordinator pre-review found missing ontology genesis checks in the intermediate implementation;
the worker was asked to test and repair them. Verify the completed result, not that early snapshot.
Event occurrence identity, backend event-envelope version dispatch, durable post-seed operation
application, full-root receipts, historical reconstruction and explain remain subsequent stories.
Do not turn their documented absence into a claim this seed wave introduced it.

Report exact commands, exit statuses, case counts and one outcome per finding with file:line,
origin and reachable scenario. Distinguish a runtime reproduction from static inference. Name
every file written outside the tree. Do not quote private filesystem roots in public evidence.
