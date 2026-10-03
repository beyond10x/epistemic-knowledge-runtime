---
format: aep.planning-md/3
id: review-result:adversary-evidence-cache-1
kind: review-result
status: active
title: Independent evidence HTTP cache boundary review
relations:
- reviews: story:viewer-evidence-reuses-admitted-revision
revision: 1
---
unit: story:viewer-evidence-reuses-admitted-revision candidate 557937e5b9a40f847b75205f494233473901a237; test commit 6955d8ded0d830cf5f66836cd4cfcf9addd9a2e1
verdict: nothing found
cases: executed 24→27, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 2 assigned roots (review scratch and sequential shared target); absolute inventory supplied separately
needs-coordinator: full repository gate, integration and release; no unresolved finding from this pass

```text
 crates/ekr/tests/view_cli.rs | 218 +++++++++++++++++++++++++++++++++++++++++++
 1 file changed, 218 insertions(+)
```

Owners: 0 findings, 0 coordinator, 0 implementor.

## 1. Test-only boundary

Only `crates/ekr/tests/view_cli.rs` changed. No existing case was deleted, rewritten, weakened or
ignored. No implementation, documentation, dependency or planning-store edit occurred. The
initial implementation base was `f41275c0554dcdd2a91ebced2149b2f06aedaf84`; the candidate diff and
acceptance statement were read before writing cases. Review output is agent judgement, not a
human approval or a claim of verifier independence.

## 2. New cases, first execution

All three cases were written before any tests ran in this pass. They use the real HTTP viewer
process and execute both File and SQLite. Synthetic evidence is added only through the public
kernel propose/validate/commit membrane. Timeouts bound stalled process I/O, never assert cost.

| Case | File:line | Assertion / result |
|---|---|---|
| `adversary_evidence_http_tracks_new_head_and_membership_after_cache_eviction` | `crates/ekr/tests/view_cli.rs:1059` | Evidence is the first HTTP read after each commit; exact current bytes, past membership refusal, four added revisions, historical reload beyond the documented three-entry cache, exact head 5 and unchanged event count after reads. Green. |
| `adversary_evidence_http_recovers_same_revision_replacement_without_old_payload` | `crates/ekr/tests/view_cli.rs:1114` | Warm revision 2, remove its path: evidence/readiness 503 while health is 200; replace with another revision-2 store holding the same evidence id with different bytes: exact new bytes and recovery, old historical membership still 404. Green. |
| `adversary_evidence_http_preserves_bytes_and_refusals_without_poisoning_cache` | `crates/ekr/tests/view_cli.rs:1172` | NUL/invalid-UTF-8 and HTML/header-shaped payloads remain exact bodies with safe MIME, length and headers; malformed/duplicate/overflow/unknown queries, wrong Host and POST refuse, then valid cached reads still succeed; canonical head/events unchanged. Green. |

Command (jobs 2, test threads 2, no debug/incremental, direct compiler, nice 19, assigned TMPDIR):

```text
cargo test -p ekr --locked --test view_cli adversary_evidence_http -- --nocapture
   Compiling serde_yaml_ng v0.10.0 (<worktree>/vendor/serde_yaml_ng)
   Compiling ekr-core v0.0.28 (<worktree>/crates/ekr-core)
   Compiling ekr-ontology v0.0.28 (<worktree>/crates/ekr-ontology)
   Compiling ekr-graph v0.0.28 (<worktree>/crates/ekr-graph)
   Compiling ekr-sdk v0.0.28 (<worktree>/crates/ekr-sdk)
   Compiling ekr-store v0.0.28 (<worktree>/crates/ekr-store)
   Compiling ekr-kernel v0.0.28 (<worktree>/crates/ekr-kernel)
   Compiling ekr-integrate v0.0.28 (<worktree>/crates/ekr-integrate)
   Compiling ekr-views v0.0.28 (<worktree>/crates/ekr-views)
   Compiling ekr v0.0.28 (<worktree>/crates/ekr)
    Finished `test` profile [unoptimized] target(s) in 18.66s
     Running tests/view_cli.rs (<target>/debug/deps/view_cli-9a8d9c979bd28133)

running 3 tests
test adversary_evidence_http_preserves_bytes_and_refusals_without_poisoning_cache ... ok
test adversary_evidence_http_recovers_same_revision_replacement_without_old_payload ... ok
test adversary_evidence_http_tracks_new_head_and_membership_after_cache_eviction ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 15 filtered out; finished in 2.88s

exit: 0
```

No red output exists: the first execution passed all three cases. No compilation failure,
mutation or failing probe is represented as a product finding. Formatting afterward changed
only layout in the newly added test code.

## 3. Affected target suite after additions

The before count is the implementor's retained `unit/affected-test.log`: `view_cli` 15 and
`search_page` 9. It is not a suite run performed before this pass wrote its cases. The after
count below is 18 + 9 = 27, no failures, no ignored tests and no skipped target. The separate
three-case first run overlaps this suite and is not added to its total.

```text
cargo test -p ekr --locked --test view_cli --test search_page -- --nocapture
   Compiling ekr v0.0.28 (<worktree>/crates/ekr)
    Finished `test` profile [unoptimized] target(s) in 1.09s
     Running tests/search_page.rs (<target>/debug/deps/search_page-09a19670a5cbdb3f)

running 9 tests
test blank_no_match_and_unavailable_states_explain_the_next_action ... ok
test entry_has_an_accessible_get_form_without_scripts_or_external_assets ... ok
test historical_results_keep_authoritative_order_and_pin_every_link ... ok
test live::live_search_escapes_query_refuses_bad_bounds_and_matches_json_ranking ... ok
test live::search_entry_works_without_graph_libraries_and_keeps_response_protections ... ok
test live::unavailable_search_keeps_query_in_a_useful_html_page ... ok
test live::historical_search_keeps_its_results_and_retained_evidence_pinned_after_a_commit ... ok
test untrusted_text_is_escaped_and_identity_link_parameters_are_encoded ... ok
test results_and_visible_untrusted_fields_have_hard_bounds ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.22s

     Running tests/view_cli.rs (<target>/debug/deps/view_cli-9a8d9c979bd28133)

running 18 tests
test adversary_evidence_http_preserves_bytes_and_refusals_without_poisoning_cache ... ok
test adversary_evidence_http_recovers_same_revision_replacement_without_old_payload ... ok
test ekr_view_answers_a_malformed_or_oversized_head_with_its_own_400 ... ok
test adversary_evidence_http_tracks_new_head_and_membership_after_cache_eviction ... ok
test ekr_view_answers_from_a_sqlite_database_copied_over_its_file ... ok
test ekr_view_answers_from_a_store_replaced_by_rename ... ok
test ekr_view_binding_is_explicit_and_defaults_to_loopback ... ok
test ekr_view_opens_an_existing_store_only ... ok
test ekr_view_answers_busy_over_64_connections_and_serves_again_after_their_deadline ... ok
test ekr_view_refuses_a_body_and_survives_any_announced_length ... ok
test ekr_view_serves_only_a_request_whose_host_names_it ... ok
test ekr_view_serves_the_changes_since_as_ekr_views_reads_them_across_a_commit ... ok
test ekr_view_serves_the_head_at_head_and_a_past_revision_unchanged_across_a_commit ... ok
test no_read_route_copies_the_head_graph_through_runtime_snapshot ... ok
test rust_source::tests::chars_escapes_and_lifetimes_keep_distinct_boundaries ... ok
test rust_source::tests::comments_and_all_string_forms_are_not_code_identifiers ... ok
test rust_source::tests::identifiers_do_not_join_across_comments_or_split_at_unicode ... ok
test ekr_view_serves_the_page_the_projection_and_evidence_and_writes_nothing_on_both_providers ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 11.11s

exit: 0
```

Formatting check and focused lint also passed:

```text
cargo fmt --all --check
exit: 0
cargo clippy -p ekr --locked --test view_cli -- -D warnings
    Checking serde_yaml_ng v0.10.0 (<worktree>/vendor/serde_yaml_ng)
    Checking ekr-core v0.0.28 (<worktree>/crates/ekr-core)
    Checking ekr-ontology v0.0.28 (<worktree>/crates/ekr-ontology)
    Checking ekr-sdk v0.0.28 (<worktree>/crates/ekr-sdk)
    Checking ekr-graph v0.0.28 (<worktree>/crates/ekr-graph)
    Checking ekr-store v0.0.28 (<worktree>/crates/ekr-store)
    Checking ekr-kernel v0.0.28 (<worktree>/crates/ekr-kernel)
    Checking ekr-integrate v0.0.28 (<worktree>/crates/ekr-integrate)
    Checking ekr-views v0.0.28 (<worktree>/crates/ekr-views)
    Checking ekr v0.0.28 (<worktree>/crates/ekr)
    Finished `dev` profile [unoptimized] target(s) in 7.44s
exit: 0
```

## 4. Findings

Nothing found in this bounded pass.

The coordinator's separately recorded `task:held-file-payload-overwrite` was not re-executed,
fixed or cleared by this pass. Its existing status is not counted as a new finding. No full
workspace, PostgreSQL, private deployment or external-client execution is claimed here.

## 5. Attack limits

- Actual HTTP evidence requests across head advances and historical cache eviction did not leak later membership into earlier revisions.
- Same-revision store replacement after a missing-path refusal recovered without returning the old cached payload.
- Binary and markup-shaped payloads stayed exact retained bodies; invalid requests did not poison later reads.
- Static caller inspection covered evidence dispatch, `IndexCache::index`, held-store reopening and kernel content reads. No new replay-count claim is inferred from the HTTP process tests; the implementor's deterministic counter regression remains separate.

## 6. Outside-worktree inventory and handoff

All writes outside the managed checkout are within `<review-scratch>` (this report, raw logs,
exit files, patch, scans, handoff metadata and a synthetic TMPDIR) or the coordinator-assigned
`<shared-target>`. The separate private `outside-paths.txt` lists their absolute locations; do
not paste that private inventory into public source. Temporary test stores were RAII-cleaned.
No scratch implementation copy, provider fixture or company-derived data was created.

Candidate patch SHA-256: `457b12c7e66863397d27b6c88c0fb688de70076954dfe6c15cbfb98d482f2322`.
Bot-authored test commit: `6955d8ded0d830cf5f66836cd4cfcf9addd9a2e1`; both author and committer
were checked as `b10x-bot[bot]`. Branch `review/evidence-cache` is the handoff branch. Public-text
scan and publication results are retained separately. All Cargo/test processes have exited;
the coordinator may use the shared target. The review lease is released on final handoff;
worktree/target cleanup belongs to the coordinator.

```findings
[]
```
