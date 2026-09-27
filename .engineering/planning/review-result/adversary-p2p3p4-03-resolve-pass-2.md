---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-03-resolve-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit S (ekr resolve)
relations:
- reviews: story:ekr-resolve-verb
revision: 1
---
unit: story:ekr-resolve-verb (unit S), head 886a2d9f plus one untracked adversary test file; correction attacked = git diff 5ce93aff..886a2d9f
verdict: red
cases: executed 259→266, red 1
origin: introduced 1, pre-existing 1, undecided 0
wrote-outside-worktree: 12 paths, all under <scratch>/ (part 6)
needs-coordinator: no

## 1. `git --no-pager diff --stat`

Prints nothing: the only change is one untracked test file. `git status --short`:

```
?? crates/ekr/tests/adversary_s_resolve_pass2.rs
```

No implementation file was touched. Nothing was mutated in the tree.

## 2. Cases added — crates/ekr/tests/adversary_s_resolve_pass2.rs (7)

| case | asserts | now |
|---|---|---|
| `deep_nesting_under_a_key_is_refused_in_bounded_time` | 60 000 nested `[`…`]` under `extra:` (120 071 bytes, about 1/8 of LIMIT) is a fault, exit 1, within 5 s | **red** |
| `a_typed_reference_written_as_a_sequence_is_refused_as_the_schema_refuses_it` | `[id, [alice]]` and the block form are refused, exit 1 | green |
| `very_deep_nesting_is_a_fault_not_a_crash` | 200k top-level depth, 5k depth under a key and under `aliases`, 1k block mappings: exit 1, no crash | green |
| `several_documents_are_a_fault` | 2 valid docs, alias in doc 2, number in doc 2, empty first doc: exit 1 | green |
| `empty_and_unreadable_input_are_faults` | empty, comment-only, non-UTF-8 stdin: exit 1 with the typed-reference prefix; missing file: exit 1 naming the file | green |
| `json_flow_and_anchored_forms_the_schema_accepts_still_resolve` | JSON, an anchored flow mapping with no alias, a 900 KB alias: exit 0 | green |
| `the_printed_outcome_decodes_back_through_the_strict_aliases_decoder` | the verb's printed `ProposeNew` JSON and `serde_json` `from_str`/`from_value` decode through the new `strings` decoder | green |

Red run of the case alone (`cargo test -p ekr --locked --test adversary_s_resolve_pass2 -- deep_nesting_under_a_key_is_refused_in_bounded_time`):

```
thread 'deep_nesting_under_a_key_is_refused_in_bounded_time' (587513) panicked at crates/ekr/tests/adversary_s_resolve_pass2.rs:221:5:
a 120071-byte document took 26.177641827s to refuse
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 6 filtered out; finished in 26.48s
```

Order deviation, stated: the first run of this file had an earlier version of `empty_and_unreadable_input_are_faults`. It was red for a reason I then withdrew: it required a missing file to carry the `typed reference <file>:` prefix, but the read path prints `ekr: opening missing.yaml: …`. `read`'s doc says "a fault naming it", and that message does name the file. I relaxed my own assertion to exit 1 plus the file name. I also cut the block-mapping depth in the crash case from 20 000 to 1 000, because at 20 000 my fixture was about 200 MB. The timing case above was written after that run, and was then run alone before anything else.

Probes (binary run directly, scratch fixtures):

| input | base b7966ce7 | head 886a2d9f |
|---|---|---|
| 60k-deep flow nesting under `extra:` (120 KB) | 9.63 s, exit 1 | 21.56 s, exit 1 |
| 200k-deep under `extra:` (400 KB) | not run | 401.9 s, exit 1 |
| 200k-deep under `aliases:` (400 KB) | not run | 166.6 s, exit 1 (strict refuses after 1 load) |
| 200k-deep at top level (400 KB) | not run | 1.38 s, exit 1 |

The cost grows quadratically: 60k gives 12.2 s under `aliases:`, 200k gives 166.6 s, and (200/60)^2 = 11. `perf record` puts 99.6% of the time in `unsafe_libyaml::scanner::yaml_parser_fetch_more_tokens`, 63% of it in `yaml_parser_stale_simple_keys`. The base binary was built from `git archive b7966ce7` in scratch, with its own target dir. 5ce93aff alone does not build (`serde_yaml_ng` arrived in merge b7966ce7), and `crates/ekr/src`, `crates/ekr-integrate/src` and `vendor` are identical between the two commits.

## 3. Suite run (after part 2)

`CARGO_TARGET_DIR=<build-dir> CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 CARGO_INCREMENTAL=0 nice -n 19 cargo test -p ekr --locked --no-fail-fast`, exit **101**. Summed over every `test result` line: 265 passed, 1 failed, 266 executed. `<before>` = 266 − 7 = 259: my file is the only addition, and I took the count from this same run, not from a separate deselected run.

```
     Running tests/adversary_s_resolve_pass2.rs (<build-dir>/debug/deps/adversary_s_resolve_pass2-b26330ac96150182)
test deep_nesting_under_a_key_is_refused_in_bounded_time ... FAILED
thread 'deep_nesting_under_a_key_is_refused_in_bounded_time' (1103932) panicked at crates/ekr/tests/adversary_s_resolve_pass2.rs:221:5:
a 120071-byte document took 24.357296156s to refuse
test result: FAILED. 6 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 34.49s
```

Every other binary is `ok`: resolve_cli 8, adversary_s_resolve 3, schema_cli 12, story_contract 13. `cargo test -p ekr-integrate --locked` exits 0: adversary_resolve 5, domain_projection 4, manifest 3, resolve 14, doc-tests 1. `rustfmt --check --edition 2021` on the new file exits 0.

## 4. Findings (they cover 886a2d9f)

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr/src/cli/resolve.rs:33 | CONFIRMED | pre-existing | A 120 KB document (60k nested flow sequences under a key) takes 9.6 s to refuse at base. A 400 KB one takes 402 s at head. The loader's scan is quadratic in flow depth inside a block mapping, so `LIMIT` (1 MiB) bounds the bytes and not the work: about 500k depth is admitted, which extrapolates to tens of minutes of CPU. | Any `ekr resolve <file>` or `ekr resolve -`: the document is read and loaded before any store is opened. Found through the documented verb. |
| F2 | crates/ekr/src/cli/resolve.rs:46 | CONFIRMED | introduced | `strict` loads the whole document through `Documents::from_str`, and then `serde_yaml_ng::from_str` at :33 loads it again. The same 120 KB document goes from 9.6 s at base to 21.6 s at head. | Every document `strict` passes, i.e. every accepted one and every one the decoder refuses. |

Fix, named and not applied: before either load, refuse any document whose flow or block nesting depth goes past a small bound. The decoder already stops at depth 128, so a single linear byte pre-scan in `read` loses nothing. Either reuse `strict`'s loaded document for the typed decode, or accept the second load once depth is bounded.

## 5. Attacked and not broken

- Flow-style mapping, JSON input, an anchor on a map or a list with no alias: all accepted, as the schema accepts them.
- Top-level sequence `[id, [aliases]]`: refused ("invalid type: sequence, expected struct TypedReference"), even though `strict` skips non-mapping shapes.
- Nested sequences or maps inside `aliases`: refused.
- Tags on scalars, lists and maps: refused.
- Stack overflow: 200k-deep top level and 5k-deep under keys exit 1 with no crash. Block mappings 1k deep also exit 1.
- Multi-document: refused whichever document holds the alias or number, including an empty first document.
- A 900 KB scalar is accepted; over LIMIT is refused (pass 1).
- Strict decoder vs other callers: the verb's own printed JSON (internally tagged `ResolutionOutcome`), `serde_json::from_str` and `from_value` all still decode. ekr-integrate tests are green. The only other deserialization path is `schema_cli.rs:101`, which is green.
- Mutant, dropping the `type_id` string check in `strict`: equivalent. `TypeId` accepts only hyphenated lowercase UUID text, which YAML can never read as a number or a boolean, so only the error message changes. A null still fails the UUID parse.
- Exit codes: empty, non-UTF-8 and missing input all exit 1. A missing `--host` exits 2 as usage before the document is read, which is the CLI-wide `Usage => 2` convention (`crates/ekr/src/exit.rs:55`).

## 6. Paths written outside the worktree

All under `<scratch>/`: `p2-base/` (source export of b7966ce7), `p2-base-target/` (its build dir, 904M), `p2-num.yaml`, `p2-seq.yaml`, `p2-host.json`, `p2-deep.yaml`, `p2-d.yaml`, `p2-d60.yaml`, `p2-x60.yaml`, `p2-perf.data`, `p2-suite.log`, and this report `adversary-pass-2.md`. The unit build dir `<build-dir>` was used and nothing in it was deleted.

## 7. Findings block


## Coordinator routing (2026-09-27)

- F1 → back to the implementor although it is pre-existing in the YAML loader: `ekr resolve` reaches it before any store opens, and the fix is one linear pass. Before either load, a byte scan refuses a document nested deeper than 64 levels (flow or block), exit 1 naming the document. The same exposure in the seed and propose readers is added to `task:seed-document-bounds-alias-expansion`.
- F2 → back to the implementor: the strict walk and the typed decode share one load.
- Second pass: the correction is verified by the coordinator, no third attack.

```findings
[
{"file":"crates/ekr/src/cli/resolve.rs","line":33,"category":"security","severity":"warning","verdict":"NEEDS-CHANGE","origin":"pre-existing","message":"LIMIT bounds bytes but not work; flow nesting inside a block mapping makes the YAML loader quadratic, 120 KB takes 9.6 s at base and 400 KB takes 402 s at head before the refusal"},
{"file":"crates/ekr/src/cli/resolve.rs","line":46,"category":"performance","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"strict loads the whole document and the typed decode loads it again, doubling the quadratic cost (9.6 s to 21.6 s on the same document)"}
]
```
