# Private signed-answer admission

The exact changed source is hashed in tested-source.json against its recorded parent. Source tests use the current checkout; the upgrade fixture is shared between the existing provider tests and the new private admission test. Raw logs remain outside disposable compiler output; only owned checkout/build paths are sanitized here.

KernelAuthority::validate_answer reads the policy retained by the verified authority transition and checks it against the independent host binding. It matches request and signed basis, derives the effective predecessor from retained answer state, verifies signature and current evidence/options/effects, and derives the exact ordinary operations and human-statement evidence. The private pipeline runs every ordinary validator and seals a transaction; only specifically reviewed, active, open disputed retractions receive the lifecycle exception. Ordinary profiles, supersession, closed claims, unrelated withdrawals, identity freshness and proposer/validator separation retain their checks.

The new test initializes and upgrades both file and SQLite stores using a real enrolled Ed25519 key. All four correction kinds validate, repeat deterministically and apply through the real ordinary application code. Dispute recomputation preserves old intervals, original evidence and current competitor references. Forged signatures, changed statement/basis/corrections, missing host, missing upgrade, evidence identity reuse and unreviewed effects refuse. A real unrelated committed transaction permits the original answer; a real additional competing claim requires new review. Candidate validation itself publishes nothing.

Mutation: removing the reviewed-set membership check admitted an extra withdrawal and failed the test (scope-mutant.txt). The original source was restored byte-for-byte. The first compile attempt had a missing private helper argument; it is not claimed as a red behavioral test. The focused provider case passed after restoration, and the complete kernel test targets passed with measured totals {"passed":588,"failed":0,"ignored":6}. Kernel Clippy with schema and the repository xtask format gate passed.

Ignored executions from the full kernel log:

- test cold_open_time_by_number_of_add_evidence_commits ... ignored, measurement: run explicitly
- test an_alias_free_seed_under_the_byte_cap_is_not_refused_as_alias_expansion ... ignored, adversary c7-b F1: an alias-free seed under the byte cap is refused as seed-alias-expansion (\L decodes 2 bytes to 3)
- test evidence_whose_payload_is_the_migration_marker_commits ... ignored, task:migration-marker-cannot-be-evidence: a payload equal to the migration marker's bytes is refused at commit
- test a_validate_command_against_an_older_released_revision_builds_one_candidate_view ... ignored, rebuilding a released revision revalidates its history; task:rebuilt-revision-reuses-retained-verdicts
- test a_thousand_attachments_against_seventy_thousand_assertions_commit ... ignored, builds a 70,000-assertion store; run by hand to record the time
- test write_base_store ... ignored, writes the base fixture; run by hand at the base commit

These are existing ignored tests, not newly skipped acceptance. No independent review or named C conformance is claimed. The admission entry remains crate-private and is not yet wired to a production command: its explicit temporary dead-code expectation must be removed when native answer publication/replay invokes it. The retained-answer projection is initialized empty for historical states; population, durable event/recovery adapters, retry/replay, CLI/SDK/history and C conformance remain unfinished. D-F, released ESS adoption, both demonstrations and full task check remain required for PR64.
