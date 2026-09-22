unit: story:version-persisted-contracts + story:commit-and-revision-lineage — format/seed checkpoint
verdict: red — coupled unit incomplete; bounded format/seed lanes pass
cases: whole-suite compilation refused; original seed executed 25→25; new seed executed 4 red→9 green
origin: n/a
wrote-outside-worktree: assigned scratch and dedicated memory-backed target; exact private inventory adjacent
needs-coordinator: yes — review checkpoint and reconcile five coordinator-owned files before continuing

This is the requested frozen implementation checkpoint, not the coupled stories' final handback. The full graph/store/kernel target set does not compile yet: old provider and kernel fixtures still name the removed interface. No whole gate, complete writer, migration, CLI or phase completion is claimed. The original seed lane retains its 25 tests, and the four original independent seed cases remain executable. New deciding cases run in the separate durable_seed and current_records lanes, not in that unchanged-count original lane.

## 1. Assigned acceptance and ownership

Implement coherent current graph/seed/occurrence/receipt formats, complete ontology and host authority roots, and atomic native-blob seed publication through the actual kernel; preserve frozen original encoders and independent assertions. The approved brief allows this intermediate source checkpoint before ordinary application.

Opening source: b2b64f8159ecec0809b45e9f16183bf4e19ee8a2. The exact source patch and per-file SHA256 manifest are adjacent as source.patch and source.sha256. The patch excludes the coordinator's five intentionally dirty files: DESIGN, kernel ESS, wave resource page, kernel Cargo.toml and Cargo.lock. Coordinator supplied the five already-pinned provider test dependencies; there is no implementor-authored pin/version change. The first proposed dependency patch had a malformed hunk count and is retained unchanged; coordinator applied the exact intended additions separately and compared both lockfiles.

Own source changes cover core identity/strict collection decoding; ontology export/canonical encoders; current graph/property/assertion/event shapes; store publication and history ports; kernel current codecs, host anchor, seed replay/admission and public opening facade; scoped fixtures and tests. No AEP, shared specification, parser-profile or frozen original encoder/vector changes were made by this unit. No second storage implementation was introduced.

## 2. Reviewable changes

The actual tracked diff stat is retained in tracked-diff-stat.txt. source-files.txt includes newly added files as well as tracked changes; source.patch contains them all.

Current formats are graph-document/2, seed/2, seed-envelope/2 and revision-event/2. Assertion assessment and lifecycle are independent. Node/edge property values have an ordered outer vector; an empty outer vector refuses, while a single empty List remains one property value. Retraction has its reason and Supersession has its replacement/effective time. EventId is independent from provider position, and a revision occurrence binds a retained record address.

Seven current record codecs retain the complete proposal, validation basis, validation receipt, commit receipt, seed result, rejection and stale shapes. Codecs refuse unsupported format tags and unknown/duplicate record fields; decoding a record is explicitly not admission authority. Validation material uses the full canonical transaction and basis in its own canonical hash domain. Ontology canonical encoding includes unused declarations and recursive parameters. The host supplies the full Agent registry, distinct bootstrap identities and exact deterministic profile; no authority or clock default is fabricated.

ObjectStored now uses backend schema 2 and exactly four metadata fields; bytes are native provider blobs. ObjectRetentionRaised remains schema 1. The portable AtomicBlobEventStore trait stages blobs and object metadata with the revision occurrence in one conditional publication. Legacy inline schema 1 refuses live ingress and remains available only through frozen verification; no live migration is claimed. The actual native blob bytes, size, address and required retention are checked.

Seed validates the complete original input under the actual host anchor, publishes seed envelope/result/evidence together and returns SeedResultV1. Identical parsed input/context/anchor retry returns the retained original result before invoking the host clock. The original competing-seed/no-losing-object controls execute through both providers. Replay revalidates the complete retained seed and root instead of trusting an in-memory allowlist. Non-seed decisions currently refuse explicitly as unsupported rather than yielding the seed head.

Public facade currently exposes Runtime::file/sqlite(path, tenant, ontology, context, anchor), seed(document, lazy_clock), head, snapshot, replay and content. The raw store remains private and CLI needs no ekr-store dependency. The final opener must remove the redundant caller ontology: retained ontology is decoded as untrusted input, then full seed/context/host-anchor verification precedes any state exposure. That seam is agreed with the coordinator and remains source work after checkpoint acknowledgement.

## 3. Original red evidence

Exact commands, complete stdout/stderr and exit statuses are retained per lane, without overwriting failed runs. commands.tsv indexes them. First baseline-seed refused before Cargo with status 125 under the original resource floor; the coordinator amended this unit's resource policy and baseline-seed-resumed then passed 25.

red-durable-seed.command:
`cargo test --locked --offline -p ekr-kernel --test durable_seed`

Verbatim result:
`test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.72s`
Exit 101. Graph version was absent; seed/2 failed decoding; identical retry returned AlreadySeeded on both providers; unused ontology edits left ontology_root all-zero on both providers. Full failure messages are in red-durable-seed.log.

red-durable-objects.command:
`cargo test --locked --offline -p ekr-store --test durable_objects`

Verbatim result:
`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s`
Exit 101. Both native providers emitted inline bytes in schema 1 and returned no native blob. Full actual envelopes are retained in red-durable-objects.log.

These were executed behavior failures before the corresponding fixes. Additional later codec/refusal cases are regression evidence, not represented as pre-implementation red-first evidence.

## 4. Executed verification

Each lane's exact command, status, summaries and raw log remains adjacent in the parent evidence directory. No baseline count is invented for a new combined command. Runner summaries, not source test attributes, supply the following counts.

| Lane | Before / final executed | Exit | Actual result |
|---|---|---:|---|
| Original seed (`baseline-seed-resumed`, final `checkpoint-kernel-tests`) | 25→25 | 0 | Same original names; explicit current-format fixture migration |
| New durable seed (`red-durable-seed`, final combined run) | 4 failing→9 passing | 0 | Both providers, own-result clock/reopen, complete host anchor, Many/One-List, corrupt native blobs, synchronous runtime refusal |
| Original independent seed (final combined run) | no opening standalone count measured; final 4 | 0 | Original refusal/positive assertions retained through mechanical fixture/API changes |
| Record codecs (final combined run) | new lane; final 9 | 0 | All seven codecs plus nested strictness and occurrence hash sensitivity |
| Frozen kernel legacy (final combined run) | earlier 19→19 | 0 | 13 legacy + 6 original independent frozen cases; original sources/vectors untouched |
| Parser integration (final combined run) | previous selected run 48→48 | 0 | 37 authored + 5 first review + 6 second review; complete withdrawal shapes |
| Kernel combined final | no identical opening combined run; final 114 | 0 | Nine named targets, zero failures/ignored |
| Core + ontology complete packages (`foundation-package-suites-sixth`) | no opening whole-package run; final 166 | 0 | 164 test cases + 2 rustdoc cases, zero failures/ignored |
| Native object metadata/blob (`green-durable-objects`) | 1 failing→1 passing | 0 | Both actual providers, schema/name/body/native content checked |
| Scoped format (`checkpoint-fmt-check`) | not a test lane | 0 | Five packages |
| Scoped library strict Clippy (`checkpoint-library-clippy`) | not a test lane | 0 | Five library targets, -D warnings |
| Full five-package target compilation (`milestone-existing-target-compilation`) | no tests executed | 101 | Old fixtures still use ValidationState, RecordedValidation, attests/admit_seed, scalar properties and bare RevisionEvent variants |
| `git diff --check` | not a test lane | 0 | No whitespace errors |

Core/ontology reconciliation retained its failed runs: first public-surface guard caught unique_set lacking direct test use; explicit empty/distinct/duplicate/decoded-identity tests were added. The second guard run still failed because its textual scanner requires a qualified call; the deciding invocation was qualified without weakening an assertion. Next the EventId exhaustive ESS enumeration failed and gained the new actual ID contract cases. The activated recursive projection then failed historical partial-projection guards; their replacements assert the actual recursive ESS field types, typed codec roundtrip and canonical parameter retention. Core/ontology passed after those explicit migrations.

Two targeted source mutations were measured after the green implementation. Replacing the ontology root by a constant produced 0 pass/1 fail, explicitly observing both providers. Sampling the clock on own-result retry produced 0 pass/1 fail with `own-result retry sampled time`; that mutation stopped at the first provider and is not claimed as a both-provider red. Both mutations exited 101. The exact original commit.rs was restored with cmp and SHA256 proof, then the final combined 114-case run passed. Source mutation logs are retained under checkpoint-mutation-*. The later format pass did not change restored commit.rs.

Test fixture mapping: all 25 original seed case names are listed verbatim in baseline-seed-case-names.txt and still pass under those names. Shared mechanical migration changes format tags, explicit host anchor, Assessment plus Active lifecycle, vector property values and SeedResult.result projection. `repeated_initialization_preserves_the_lineage_and_writes_no_second_object` now expects the same exact retained result for identical input, while its changed-input refusal remains. `legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted` and the independent `persisted_seed_format_and_context_fields_refuse_before_admission` now inject corruption through actual native provider APIs because raw append was removed. Admission still goes through real kernel authority. `seed_ontology_property_definitions_must_be_filed_under_their_own_ids` retains valid compatibility ontology and both node/edge refusal assertions. `inherited_record_properties_remain_typed_and_constraints_cannot_disappear` only wraps its original one Record value in the outer vector. The complete independent fixture-only diff is retained separately. Frozen legacy files have an empty diff.

## 5. Remaining work and limits

Ordinary Propose/Validate/Commit, complete per-transaction retained states, stale results, all admitted operation application, lifecycle/supersession rules and complete revalidation/reapplication are the second milestone; they are not implemented by this checkpoint. Supersession's new payload and reference narrowing do not establish full semantic admission/application yet. Runtime currently refuses unsupported ordinary history. No temporary permissive replay authority bridges that gap.

Verified opener derives the ontology from retained seed input under the host anchor; no allow-all schema or caller ID is to be invented. Historical reconstruction must limit required payload loading at the requested committed revision, not merely stop application after later objects were loaded. Full root sensitivity, native interruption/unknown-outcome continuation, retained terminal outcomes, later-head own-result retries and fresh-process ordinary query controls remain required. Immediate native UnknownCommit retries preserve the exact provider request and do not delete data; longer-lived command retry/restart behavior is not qualified here.

A final source inspection also identified inherited ontology collection decoding still needing the agreed strict collection audit: ValueType.allowed_types/variants, lifecycle sets and several declaration maps use ordinary serde collections. The new strict graph/authority/receipt carriers do not prove those inherited nested carriers refuse duplicate decoded members. This is a source observation, not a claimed executable finding, and must be closed before final format acceptance.

Existing graph/store/kernel fixture suites, trybuild/public-ownership guards, rustdoc and final whole gate still need migration/execution. The old provider tests' substitute authorities cannot stand in for real kernel durability acceptance. AGENTS' old ValidatedSeed mechanism wording is coordinator-owned and requires final correction to the actual private admission/replay mechanism and named cases. No source was changed merely to preserve that obsolete prose.

No provider live store was opened; all provider tests used synthetic temporary state under the assigned TMPDIR. No full migration, archive/backup, CLI or conformance runtime execution is claimed. Released ESS declaration validation remains compiler evidence, separate from these runtime tests.

## 6. Retained outside-worktree evidence

The private inventory adjacent to this report contains exact absolute paths for all scratch files and the dedicated compiler target plus resolved memory-storage path. Raw commands/logs include local paths and remain private. source.patch, source.sha256 and this report contain no private fixture data. The coordinator owns cleanup and subsequent integration. No cleanup was performed.

Checkpoint source was then committed, at coordinator instruction, as edf4799b070fd71097b86bb457e0e7592221f917. The bot wrapper exited 0; both author and committer are b10x-bot[bot]. The commit changes 45 source/test files, 3128 insertions and 2009 deletions. The five coordinator-owned files remain the only dirty paths and were excluded. No push was performed. The implementor lease was released with exit 0; source and compiler activity are stopped pending checkpoint review. Per-file hashes verified after commit and mutation restoration cmp both passed.
