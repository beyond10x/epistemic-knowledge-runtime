---
format: aep.planning-md/3
id: review-result:search-entry-final-1
kind: review-result
status: active
title: Search entry independent route and history review
relations:
- reviews: story:search-first-viewer-entry
revision: 1
---
unit: story:search-first-viewer-entry — five-file candidate against 45e6a2e14348690e01ee0e6584706ebf1bbf23f5
verdict: nothing found
cases: executed 9→9, red 0; direct existing-test execution, no new cases
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 3 paths, listed below
needs-coordinator: coordinator integration and combined integration gate

## Diff and review boundary

The candidate's tracked `git diff 45e6a2e143 --stat` is:

```
 crates/ekr/src/cli/mod.rs  |   1 +
 crates/ekr/src/cli/view.rs | 202 ++++++++++++++++++++++++++++++++++++++++++---
 docs/cli.md                |  18 +++-
 3 files changed, 205 insertions(+), 16 deletions(-)
```

New files are crates/ekr/src/cli/search_page.rs and crates/ekr/tests/search_page.rs. All five files were authored by the search implementor. This reviewer made no source/test/AEP changes. The existing HTTP transport was authored by this reviewer in the prior unit; this review covers the search delta and its use of those existing seams, not a new independent review of the transport's implementation. Source diff whitespace check exited0.

Owners: 0 findings. Search implementation remains the search unit's ownership; final lint/freeze and coordinator integration are separate from this bounded review verdict.

## Cases and execution

No independent failing case was found or added. This pass combines static attack of the five-file delta with an independently invoked existing nine-case test executable; it does not claim newly written adversarial test coverage. The implementor reported9 cases before this pass; the runner executed9 afterward, none failed/ignored/filtered. Test executable and application binary SHA256:

- `/dev/shm/ekr-search-entry-target/debug/deps/search_page-2579e599866282fd`: 10329d706b9ec53301f94df4f1465bef2e890770afab897fdd7ffcddb521cc57
- `/dev/shm/ekr-search-entry-target/debug/ekr`: 2b3a7a7f3c886c4fc7e3ade7f9862efd15391503a02cb47f1810766864b83d9b

Coordinated with the author before execution. No Cargo/build/shared fixture lane was started. Direct command (generic path aliases denote the actual search worktree and assigned cache):

```
CARGO_MANIFEST_DIR=<worktrees>/ekr-search-entry/crates/ekr \
TMPDIR=<cache>/ekr-hosted-runtime/serving/search-review-tmp \
/dev/shm/ekr-search-entry-target/debug/deps/search_page-2579e599866282fd \
  --test-threads=1 --nocapture
```

Exit0; complete output in `<cache>/ekr-hosted-runtime/serving/search-review-tests.log`:

```

running 9 tests
test blank_no_match_and_unavailable_states_explain_the_next_action ... ok
test entry_has_an_accessible_get_form_without_scripts_or_external_assets ... ok
test historical_results_keep_authoritative_order_and_pin_every_link ... ok
test live::historical_search_keeps_its_results_and_retained_evidence_pinned_after_a_commit ... ok
test live::live_search_escapes_query_refuses_bad_bounds_and_matches_json_ranking ... ok
test live::search_entry_works_without_graph_libraries_and_keeps_response_protections ... ok
test live::unavailable_search_keeps_query_in_a_useful_html_page ... ok
test results_and_visible_untrusted_fields_have_hard_bounds ... ok
test untrusted_text_is_escaped_and_identity_link_parameters_are_encoded ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s

```

This exercised five renderer cases and four live viewer process cases. Broader compatibility/lint results belong to the implementor's separately recorded lane. The first inspected clippy log ended on RAM filesystem quota while emitting ess-conformance metadata, not a source diagnostic; no lint pass is inferred from that failure.

## Attack coverage and limits

- History/evidence authority: find obtains one IndexCache index at the requested revision and uses that index's search document, graph assertions, evidence metadata and retained hashes. It never switches to head while gathering links. Evidence IDs must exist in that revision and their payload hash must be retained before a link is emitted. Payload bytes are not fetched per result. The evidence endpoint now passes the requested revision into runtime.read; graph links carry the same revision in the existing viewer hash syntax. Live tests preserved old HTML after a commit and resolved retained evidence at explicit revision0, rejecting missing revision/id. The fixture's evidence already existed at seed, so this is not a separately constructed future-only evidence attack; source review of runtime.read(at) is the basis for that boundary.
- Ranking/bounds: the page calls existing Index::search with20, preserving Exact/Folded tier, degree and identity ranking. It derives total/revision from the authoritative document. Renderer independently caps20 results,3 evidence links each,256 displayed label characters and2048 query characters; parsed NodeId/EvidenceId identities become bounded local encoded link parameters. Underlying search cost still scans the existing name index; evidence collection scans the already loaded assertions once. No claim of constant graph-independent query cost is made.
- Escaping: query, name, alias and evidence label all pass the same text/quoted-attribute escape helper. Both generated identity parameters and latest-query links percent-encode UTF-8. Revision parameters are typed integers. There are no untrusted HTML, script, style or URL scheme interpolations. Existing renderer cases use quote/script/ampersand payloads and injected identity delimiters; the live query case exercises percent-decoding before HTML escaping. Parameter encoding uses local fixed URL prefixes.
- No-JS behavior: normal GET form, explicit label, search input, submit button, mobile viewport and responsive CSS; no script/external font/CDN asset. The initial empty form requests head only, without constructing a search index. New search CSP has script-src none, form-action self, base-uri none and anti-framing. Common response writing retains no-store, nosniff, content type and connection-close headers. The graph application remains separately reachable at root.
- Failures/cache: missing store retains the supplied query and explicit requested revision in a useful503 HTML form, without falsely displaying a served revision. Missing revisions produce404 HTML. Empty results are a distinct200 state. Invalid/repeated query fields, invalid UTF-8 and bounds are400. Held store replacement invalidates Memory/indexes through the existing Checked::Reopened path; find maps unavailable/replaced stores to its HTML unavailable state. Historical form submissions retain the explicit revision; unpinned submissions continue to latest. Existing IndexCache checks head before each cache lookup.
- Compatibility: existing JSON /search and graph root route implementations are unchanged. Evidence without a revision still resolves head; the new optional revision is additive, with malformed/unknown query parameters now refused. The existing graph hash parser consumes revision and node as emitted by the search renderer. Method/body checks for /find run in immediate transport handling, retaining405/413 before missing-store admission. Queue/connection busy responses may remain the common plain503 rather than search HTML, which is a transport overload response rather than a misleading empty search result.

No browser automation was performed by this reviewer; layout screenshots and browser checks, if reported, belong to their producer. No PostgreSQL/database fixture or credentials were accessed. This review is bounded to the generic source delta and the exact executed synthetic SQLite process tests.

## Candidate file identities

- crates/ekr/src/cli/mod.rs: a5fde06bf90f40b01dfec717ebfe4e82d1d27e69d693450bf9d5755228a35ba3
- crates/ekr/src/cli/view.rs: 556a1f113ab03bab5be5f8674e062bcaeb0136e307117d0b744a4362bf93c3f2
- crates/ekr/src/cli/search_page.rs: 995f3c0e969955bc5c7d2c012c19c82ff4513cb97f974848fc5cb876651e5fd6
- crates/ekr/tests/search_page.rs: 5e79187bbb57b9316d6bbd759a42c52f9d554c37f39d99bae8cad9d9a528b829
- docs/cli.md: 082c1acc9f04611465252e7d7a391f5519edfa69afcd3365a22a6eb7f12bc4f1

## Outside-tree paths

Only `<cache>/ekr-hosted-runtime/serving/search-review.md`, `search-review-tests.log` and `search-review-tmp/` were written, all under the coordinator-assigned scratch. Full local paths are `<cache>/ekr-hosted-runtime/serving/search-review.md`, `<cache>/ekr-hosted-runtime/serving/search-review-tests.log` and `<cache>/ekr-hosted-runtime/serving/search-review-tmp/`; personal cache prefix should be normalized before publishing this record. Test-created synthetic directories removed themselves. No build target was written by the reviewer.

## Final frozen delta

The author froze the complete five-file patch at SHA256 `9e51ce140c9864fbaa3f99180e33492ab4d93869a346ce37dbe666d128008382` in `<cache>/ekr-hosted-runtime/search/final-candidate.patch`. Independently re-read the only change since the direct9-case run: renderer empty-results guard `State::Results { matches, .. } if matches.is_empty()` became the equivalent slice pattern `State::Results { matches: [], .. }`. The final renderer SHA256 is `ff489d650ddd8ccb7de560c10ad6164a288acd414e68b5ae768758bd7339b786`; the other four file hashes above are unchanged. No behavior or assertion was removed. The inspected corrected clippy log finished successfully after the target moved from RAM to disk; the author reports its exit0 and fmt exit0. The exact final renderer's nine-case rerun remains the author's lane until its result is recorded. The direct execution above is explicitly of the previous equivalent guard spelling, not falsely labeled as the final binary.

At handoff, the author's final-search.log also contained the exact final renderer run's summary: `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s`. This is inspected author-produced evidence, separate from the reviewer-invoked run above.

Owners: 0 findings in the final frozen delta; coordinator owns integration and the combined full gate. The review lease is released at handoff, and no reviewer process uses either target.

```findings
[]
```
