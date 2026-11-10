---
format: aep.planning-md/3
id: story:search-live-composition-open-at-attach
kind: story
status: draft
title: A composition open when the live search module attaches is not read
refs:
- provider: github
  reference: beyond10x/epistemic-knowledge-runtime#104
relations:
- serves: vision:o5
- informed_by: story:search-live-typing-waits-on-the-page
revision: 1
---
## Context

On 2026-11-10 the adversary pass on `story:search-live-typing-waits-on-the-page` (#104) found that a
composition the reader opens before the live search module attaches, and leaves idle, is read as a
partial query when the module attaches: the start-time search reads `input.value()`, which holds the
uncommitted text, and no DOM property says a composition is in progress. A composition continued after
attach is deferred since the integration of #104 (its next `compositionupdate` or composing `input`
event marks it). The case `adversary_a_composition_open_when_the_module_attaches_is_not_read` in
`crates/ekr/tests/search_live.rs` holds the residual and is ignored, naming this story.

## Build

- Decide where a composition opened before attach can be observed. The search page's policy is
  `script-src 'self' 'wasm-unsafe-eval'`, and anything that runs is Rust, so the answer is either a
  signal the module can read at attach or a start-time search that waits for one event that
  distinguishes an idle composition from committed text.
- Remove the `#[ignore]` from the adversary's case.

## Acceptance

- `adversary_a_composition_open_when_the_module_attaches_is_not_read` runs un-ignored and passes;
  `a_query_typed_before_the_live_module_attaches_is_searched_once_it_does` and every other `search_live` case still pass.
