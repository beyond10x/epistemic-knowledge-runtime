# Read-only proposal presentation checkpoint

Retained proposals join the typed attention projection without a separate queue. The viewer links
/inbox to /schema-proposal/<id>, rendered in Rust with additions, mappings, corrections, immutable
source documents, observation bytes, retained and canonical evidence, review basis and history.
The shared HTTP guard admits GET only; proposal query parameters refuse. No browser write UI exists.

Observed feedback:

- Initial real-session case failed: one blocker question existed but its retained proposal question
  did not (1 passed, 1 failed). Implementation made the existing 2-case target pass on both providers.
- Independent review found a retained evidence record could hide canonical material sharing its ID.
  A synthetic distinct-payload regression failed (1 passed, 1 failed). The page now renders both
  records explicitly; canonical evidence comes from the proposal's captured basis revision.
- Final docs_cli: 18 passed, 0 failed, 0 ignored. Final knowledge_cli: 2 passed, 0 failed, 0 ignored.
  Counts are unchanged because the existing child-process case was extended. Both providers check
  discovery/submission/show, projection, full-replay reopen, HTML escaping, GET and write refusal.
- authority_upgrade: 7 passed, 0 failed, 0 ignored. knowledge_retention: 14 passed, 0 failed,
  0 ignored. The upgrade test's generic helper now declares the added physical retention bound;
  no assertion was removed. The earlier all-target compile refusal is retained.

The final saved File-provider HTTP response was rendered by headless Firefox and visually inspected:
proposal digest, additions, source sections, separate retained/canonical evidence sharing an ID,
review basis and history are visible. Collapsed evidence payloads and SQLite parity are asserted by
the executable case. This screenshot inspected saved route HTML, not a live server. Two Chromium
capture attempts produced no screenshot (one stopped, one timed out); their logs remain in raw
evidence. Firefox completed. No synthetic review approval or application is implied by the empty
history panels, and this empty-mapping presentation fixture is not the project-health demonstration.

Independent source/log review is retained as schema-proposal-viewer-independent-r1 and -r2.
Neither reviewer ran an independent build. Signed decisions, review-aware inbox filtering,
application conflict validation and full E/F conformance remain required. No full task check is
claimed. The application design candidate is separately retained and remains unvalidated.

Final cargo clippy --locked -p ekr-kernel -p ekr -p ekr-sdk --all-targets -- -D warnings
and repository xtask fmt --check both exited zero; their final logs are retained here.
