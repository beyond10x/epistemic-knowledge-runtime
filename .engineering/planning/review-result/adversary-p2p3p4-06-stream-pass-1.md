---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-06-stream-pass-1
kind: review-result
status: active
title: Adversary pass 1 on the streamed ekr view server and engine
relations:
- reviews: story:view-streams-overview-and-expansion
revision: 1
---
unit: story:view-streams-overview-and-expansion — the `ekr view` stream server (crates/ekr/src/cli/view.rs) and the ekr-views query engine, at c2b97ba81a (branch adv/stream-pass-1) plus two untracked test files
verdict: confirmed — 4 red cases (3 introduced, 1 pre-existing), plus 1 judgement finding
cases: executed 127→132, red 4
origin: introduced 4 / pre-existing 1 / undecided 0
wrote-outside-worktree: 5 paths (part 6)
needs-coordinator: (a) the build ran in a tmpfs dir, not the assigned one, because `/` had 1.5–5.6 G free (the operator rule is no builds under 10 G); delete that dir when this pass is done. (b) F3 contradicts an assertion in view_stream.rs, so it needs a decision: the spec or the transport.

## 1. Diff

`git --no-pager diff --stat` is empty: nothing tracked changed. `git status --short`:

```
?? crates/ekr-views/tests/adversary_query_pass1.rs
?? crates/ekr/tests/adversary_stream_pass1.rs
```

Both are test files. No implementation file was touched.

## 2. Cases added (each run alone before the suite ran)

| File::case | Asserts | Now |
|---|---|---|
| crates/ekr/tests/adversary_stream_pass1.rs::an_http_1_0_request_is_not_answered_with_transfer_encoding | a `GET /expand … HTTP/1.0` gets no `Transfer-Encoding` (RFC 9112 § 6.1) | red |
| crates/ekr/tests/adversary_stream_pass1.rs::an_absent_revision_is_refused_before_a_node_that_is_no_node_id | `/node/not-an-id?revision=9` answers `RevisionNotFound` (views.yaml refusal order) | red |
| crates/ekr/tests/adversary_stream_pass1.rs::an_empty_seed_set_answers_an_empty_page | `/expand?seeds=&depth=1&limit=10` answers an empty page with node_total 0 (views.yaml ExpandNeighbourhood) | red |
| crates/ekr/tests/adversary_stream_pass1.rs::a_pinned_revision_pages_across_a_commit_with_nothing_lost_or_repeated | paging the star at revision=0 with a commit between pages and one commit during an open, unread stream: every page is the engine's, and together they hold the whole sequence once | green |
| crates/ekr/tests/adversary_stream_pass1.rs::sixty_four_concurrent_streams_each_deliver_the_whole_page | 64 simultaneous readers each get the whole framed stream, identical to the engine's | green |
| crates/ekr-views/tests/adversary_query_pass1.rs::every_page_of_generated_graphs_is_the_page_views_yaml_describes | an oracle written from GraphSliceV1's text, over 24 generated graphs × 12 requests (self-loops, parallel edges, isolated nodes, repeated seeds, depth 0–2, limits 1–6): same sequence, totals, every page, next, remaining, and paging delivers each record once | green |
| crates/ekr-views/tests/adversary_query_pass1.rs::a_revision_answers_the_same_bytes_after_unrelated_commits_but_for_meta_head | revisions 0 and 1 give the same bytes for all four formats under head 1 (file store) and head 4 (sqlite store), with meta.head masked | green |
| crates/ekr-views/tests/adversary_query_pass1.rs::every_detail_of_every_revision_carries_every_assertion_about_and_at_the_node | for every node of every revision of Evolved, Growth and Timeline: assertions, referencing, edges, each edge's assertions and neighbours match the graph, in id order | green |
| crates/ekr-views/tests/adversary_query_pass1.rs::search_tiers_fields_and_aliases_are_the_ones_views_yaml_names | the MatchTier and NodeMatch rules on İ, ẞ, ǅ, final sigma and mixed case: tier, field, alias, total and exact_total | green |
| crates/ekr-views/tests/adversary_query_pass1.rs::the_default_overview_of_ten_types_dated_weekly_for_ten_years_stays_under_300_kb | the default `/overview` of a store with 10 node types and one dated fact a week for 10 years stays under 300 KB | red |

Red output, verbatim, from each case run alone:

```
an_http_1_0_request_is_not_answered_with_transfer_encoding:
assertion `left == right` failed: GET /expand?seeds=00000000-0000-4000-8000-000000000301&depth=1&limit=10 HTTP/1.0 was answered with Transfer-Encoding
  left: Some("chunked")
 right: None

an_absent_revision_is_refused_before_a_node_that_is_no_node_id:
assertion `left == right` failed: {"message":"\"not-an-id\" is not a node id","refusal":"ekr.views.NodeNotFound"}
  left: String("ekr.views.NodeNotFound")
 right: "ekr.views.RevisionNotFound"

an_empty_seed_set_answers_an_empty_page:
assertion `left == right` failed: {"message":"the seed \"\" is not a node id","refusal":"invalid-query"}
  left: 400
 right: 200

the_default_overview_of_ten_types_dated_weekly_for_ten_years_stays_under_300_kb:
10 types and 5200 assertions: the default overview is 449729 bytes, 5200 timeline buckets (limit 1: 448629 bytes)
```

Dropped before the suite ran: one case about retracting an edge's assertion and then deleting the edge. The kernel refuses that delete (`unresolved-edge: edge …0140 is not in the graph`), whether the retract and the delete are in one transaction or two. So the `Inconsistent` guard in `Index::build` cannot be reached, and there is no finding.

## 3. Suite runs (after the cases existed)

Both runs used the tmpfs target, with `CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=8 RUST_TEST_THREADS=2 nice -n 19`. Full logs are in `<scratch>/suite-ekr-views.log` and `<scratch>/suite-ekr-view.log`.

`cargo test --locked --no-fail-fast -p ekr-views` → EXIT=101

```
adversary_pass2 3 ok · adversary_property_redeclaration 3 ok · adversary_query_pass1 4 passed 1 FAILED ·
conformance 3 ok · determinism 4 ok · document 6 ok · pure_render 1 ok · query 14 ok · reads_only 1 ok ·
replays 1 ok · shared_id 2 ok · lib 0 · doctests 0   → 43 executed
```

`cargo test --locked --no-fail-fast -p ekr --lib --bins --test view_stream --test view_page --test view_cli --test view_roles --test adversary_v_view --test adversary_v_view_r2 --test adversary_p_page --test adversary_p_page_2 --test adversary_stream_pass1` → EXIT=101

```
lib 43 ok · main 0 · adversary_p_page 3 ok · adversary_p_page_2 3 ok · adversary_stream_pass1 2 passed 3 FAILED ·
adversary_v_view 4 ok · adversary_v_view_r2 6 ok · view_cli 7 ok · view_page 15 ok (1 ignored) · view_roles 3 ok ·
view_stream 5 ok   → 94 executed
```

How the case counts were derived:

- Before: 127 cases. These are the same runs with my two binaries taken out (43 − 5 and 94 − 5). The binaries are independent, so leaving mine out changes no other count.
- After: 132 cases, of which 4 are red.
- Not run: the rest of `-p ekr`, meaning its conformance, CLI and adversary binaries for other stories. The load average was 26–35 on 20 cores.

## 4. Findings

**F1: `/expand` answers an HTTP/1.0 request chunked.** Confirmed, introduced, low.
- Where: crates/ekr/src/cli/view.rs:377 (`stream_head`), with parse_head at :445–477, which does not record the request's HTTP version.
- What was measured: the red case above, on the real binary.
- What reaches it: any HTTP/1.0 client, such as `curl --http1.0` or a 1.0 proxy. That client reads the chunk-size lines as body text, so the NDJSON no longer parses. The page uses fetch (HTTP/1.1) and is not affected.
- Fix: for a 1.0 request, write the body without chunk framing and close the connection to mark its end.

**F2: `/node/<id>` checks the id before the revision.** Confirmed, introduced, low.
- Where: crates/ekr/src/cli/view.rs:803–809.
- What happens: an id that does not parse is refused as `NodeNotFound` before the revision or the seed is looked up. views.yaml orders the refusals NotSeeded, then RevisionNotFound, then NodeNotFound.
- What was measured: `/node/not-an-id?revision=9` answers NodeNotFound. On an unseeded store it would also answer NodeNotFound instead of NotSeeded; I read that from the code and did not run it.
- What reaches it: any client. Both refusals are 404, so only the refusal name is wrong.
- Fix: parse the id after `indexes.index(...)`.

**F3: `/expand` refuses an empty seed set.** Confirmed, introduced, low.
- Where: crates/ekr/src/cli/view.rs:844–852.
- What happens: `seeds=` splits into `[""]` and is refused as `invalid-query`. views.yaml says "an empty set answers an empty page with node_total 0", and the engine does exactly that.
- Conflict: view_stream.rs::the_bounded_reads_refuse_as_whole_json_before_any_byte asserts the opposite. The coordinator has to pick which one is right.
- What reaches it: a direct HTTP client. The page never sends an empty `seeds`, because `topEdges` checks `seeds.length`.

**F4: nothing a request can set bounds the overview's size.** Plausible, pre-existing, medium.
- Where: crates/ekr-views/src/index.rs:563 (`timeline`) and query.rs:761 (`overview`).
- What happens: `limit` bounds only `top`. The timeline has one bucket per week per type once dated facts span more than 120 days, and schema.revisions has one entry per revision. So a store with a long dated history has a first load over the story's 300 KB acceptance.
- What was measured: 449,729 bytes at the default limit, and 448,629 bytes at limit=1.
- Origin: the same case, run against the base 0251272fe1 in a scratch copy, reports the same bytes.
- What reaches it: nothing found. The bench store's type count and date span were not shown to me, so I cannot say it goes past the cap. That is why this is plausible and not confirmed.

**F5 (judgement, no case): the module doc understates how long a client can hold a slot.** Confirmed, introduced, low.
- Where: crates/ekr/src/cli/view.rs:14–15.
- What the doc says: "a client that stalls holds one of the 64 places for at most 10 s".
- What the code does: a stream holds a place for up to the 5 s head deadline plus STREAM_TIMEOUT (60 s, :112). A client reading 1 byte every 4.9 s keeps its stream alive for all of that. Also, a connection waiting on the store thread (`reply.recv()`, :265) has no bound; that wait existed before this unit.
- Fix: correct the sentence.

## 5. Attacked and could not break

- Record order and paging: matches the spec oracle on 288 generated requests, including self-loops, parallel edges, duplicate seeds, and pages that stop between a node and its edges.
- Determinism of all four formats for one revision across unrelated commits and across providers, apart from meta.head (filed).
- Detail completeness and id order on every node of every revision of three fixtures.
- Search tiers, fields, the chosen alias, and lowercase folding on characters whose lowercase changes length.
- Paging a pinned revision across a commit, including a commit landing while a stream is open and unread. `ekr view` keeps serving while the CLI commits.
- 64 concurrent streams: all 64 are complete and identical.
- The index cache under a new head: the pinned-revision case reloads under head 1 and returns the same records. LRU eviction is already covered by query.rs.
- The kernel guard: deleting an edge that any assertion names is refused, so the index's refusal of that state cannot be reached.

## 6. Paths written outside the worktree

- `<shm>/ekr-adv-stream/target`: the build directory, about 2 G in RAM. It is not the assigned target directory, because `/` was at 1.5–5.6 G free. It is kept so the red cases can be reproduced; delete it after.
- `<shm>/ekr-adv-stream/tmp`: TMPDIR for test stores. It is empty.
- `<shm>/ekr-adv-stream/base` and `<shm>/ekr-adv-stream/base-target`: the base-commit copy used for F4's origin check. Both are already deleted.
- `<scratch>/suite-ekr-views.log`, `<scratch>/suite-ekr-view.log` and `<scratch>/adversary-pass-1.md`.

Here `<shm>` is the shared-memory tmpfs mount and `<scratch>` is the assigned scratch directory.

## 7. Findings block


## Coordinator routing (2026-09-28)

- F4 (pre-existing) → task:timeline-rows-are-subjects, which is reworking the timeline data: the overview's timeline is bounded (coarser buckets past a stated count), since per-subject rows move to ProjectTimeline.
- F1, F2, F3, F5 → one correction unit on view.rs after the timeline unit merges (both edit view.rs): HTTP/1.0 gets an unframed body and a close; /node checks the revision before the id; an empty seed set answers an empty page as views.yaml says (view_stream's refusal case changes); the module doc states the 65 s a stream may hold a place.

```findings
[
{"file":"crates/ekr/src/cli/view.rs","line":377,"category":"correctness","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"/expand sends Transfer-Encoding chunked to an HTTP/1.0 request, which RFC 9112 6.1 forbids"},
{"file":"crates/ekr/src/cli/view.rs","line":803,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"/node refuses an unparsable id as NodeNotFound before the revision is checked, reversing the views.yaml refusal order"},
{"file":"crates/ekr/src/cli/view.rs","line":847,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"/expand refuses seeds= as invalid-query where views.yaml says the empty seed set answers an empty page"},
{"file":"crates/ekr-views/src/index.rs","line":563,"category":"performance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"nothing a request can set bounds the overview; weekly timeline buckets reach 449,729 bytes at the default limit"},
{"file":"crates/ekr/src/cli/view.rs","line":15,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the module doc says a stalled client holds a place for at most 10 s, but a stream holds one for up to 65 s"}
]
```
