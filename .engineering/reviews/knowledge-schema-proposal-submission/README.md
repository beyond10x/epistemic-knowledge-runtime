# Proposal submission and inspection checkpoint

This is part of story:propose-better-vocabulary, not its completed acceptance.
The generated ESS SubmitSchemaProposal and ShowSchemaProposal obligations execute native
kernel admission and physical retention. No new contract models are transcribed.

Exact JSON bytes and typed proposals must agree. Sources bind immutable versions and canonical
facts[index] selectors. Admission validates additive vocabulary and typed mappings before retention.
Proposal reads and exact retries preserve the original document after current-schema incompatibility.
Relevant inherited declarations enter the review digest with temporary candidate IDs normalized;
unrelated schema additions do not demand another review. Canonical facts and schema remain unchanged.
CLI/session submit and show and the typed Knowledge SDK use the same kernel path.

Observed feedback:

- Initial kernel red: missing Runtime submission/show methods; implementation reached 12 passing
  knowledge_retention cases. Baseline from the preceding discovery checkpoint was 10 cases.
- Independent review found two defects: omitted inherited schema dependencies and unreadable
  retained proposals after mapped value-kind drift. Both regressions failed: 12 passed, 2 failed.
- First correction retained the document but lost an existing unresolved-reference blocker:
  13 passed, 1 failed. The fix accumulates the incompatibility alongside that blocker.
- Final knowledge_retention: 14 passed, 0 failed, 0 ignored. New cases exercise both providers;
  unchanged events, strict fresh refusal, exact retries and full replay are asserted.
- CLI/SDK compile red: missing typed Knowledge submission/show methods. After implementation,
  knowledge_cli remains 2 passing tests because its existing real-session case was extended;
  it did not gain a new test function. docs_cli reports 18 passing tests. Both provider branches
  validate returned data against generated JSON Schema and reopen through full replay.
- cargo clippy --locked -p ekr-kernel -p ekr -p ekr-sdk --all-targets -- -D warnings: exit 0.
  The initial type-complexity refusal is retained; SourceSupport replaced the anonymous tuple.
- cargo run --locked -p xtask --bin xtask -- fmt --check: exit 0.

Independent source/log review is recorded as review-result:schema-proposal-submission-independent-r1
and -r2; no independent test execution is claimed. The new strict and stale-read branches need
further application/review integration checks as E/F are completed.

Remaining E scope: signed human review retention, proposal inbox/viewer, final correction admission,
and real conformance adapters. F application and its atomic review-stream guard are not implemented.
Conformance suites have not been regenerated for this checkpoint; no full task check or overall
conformance is claimed. This checkpoint must not move E or F to done.

Raw logs remain outside disposable outputs. Curated logs only replace home, worktree and build-lane
path prefixes; runner results are otherwise preserved. One compiler job ran in the exclusively
reserved task build lane, sequentially with the completed store worker.
