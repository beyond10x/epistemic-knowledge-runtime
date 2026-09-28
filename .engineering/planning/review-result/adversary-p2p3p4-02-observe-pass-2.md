---
format: aep.planning-md/3
id: review-result:adversary-p2p3p4-02-observe-pass-2
kind: review-result
status: active
title: Adversary pass 2 on unit F (fixture records become observations)
relations:
- reviews: story:fixture-records-become-observations
revision: 1
---
unit: story:fixture-records-become-observations, worktree ekr-wave-a-f at head 1577b3e7 plus one untracked adversary test file
verdict: plausible (1 red case, public-API drift nothing outside the crate reaches); correction of pass 1 holds
cases: executed 9→13, red 1
origin: introduced 5, pre-existing 0, undecided 0
wrote-outside-worktree: 5 paths under <wave-root>/f/scratch (part 6)
needs-coordinator: no

## 1. Diff proof

`git --no-pager diff --stat`: empty (nothing tracked changed). `git status --short`:

```
?? crates/ekr-observe/tests/adversary_pass2.rs
```

That is the only path touched in the worktree, and it is a test file. No implementation file was edited; mutants ran only on a scratch copy.

## 2. Cases added (`crates/ekr-observe/tests/adversary_pass2.rs`)

| case | asserts | now |
|---|---|---|
| `observe_line_given_a_terminated_line_agrees_with_observe_jsonl_or_refuses` | public `observe_line(b"<record>\n", 1)` either refuses or returns the observation `observe_jsonl` returns for that line | **red** |
| `empty_input_maps_to_no_observations` | `observe_jsonl(b"")` is `Ok(vec![])` (doc: "Empty input has no lines") | green; kills M1 |
| `a_line_without_text_is_refused` | a line missing `text` is `NotARecord{line:1}` (lib.rs:77 comment promises it) | green; kills M3 |
| `a_line_with_an_unknown_field_is_refused` | an extra field gives `NotARecord{line:1}` | green; kills M2 |

Red run, case file alone, before the suite (`cargo test -p ekr-observe --locked --test adversary_pass2`, EXIT=101):

```
test observe_line_given_a_terminated_line_agrees_with_observe_jsonl_or_refuses ... FAILED
thread 'observe_line_given_a_terminated_line_agrees_with_observe_jsonl_or_refuses' (1230705) panicked at crates/ekr-observe/tests/adversary_pass2.rs:23:28:
assertion `left == right` failed: the same record line gave two observations with two ids depending on the entry point
  left: Observation { id: ObservationId(168438763091644154751569194352865327662), source: "fixture-source-a", source_native_id: Some("record-0001"), content: FeedItem(ContentHash([245, 195, 74, 26, ...])), captured_at: Timestamp(1767225600000) }
 right: Observation { id: ObservationId(25075726985277763671792932100346645560), source: "fixture-source-a", source_native_id: Some("record-0001"), content: FeedItem(ContentHash([37, 99, 219, 145, ...])), captured_at: Timestamp(1767225600000) }
test result: FAILED. 3 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Mutants, scratch copy only (`scratch/p2-mut`, own target dir). Each survived the existing 9 cases and was killed by a pass-2 case:

| mutant | existing suite (`adversary_observe` 4 + `fixture_observations` 5) | pass-2 case that kills it |
|---|---|---|
| M1 `if bytes.is_empty()` → `if false` (lib.rs:148) | 9/9 ok | `empty_input_maps_to_no_observations` FAILED |
| M2 delete `#[serde(deny_unknown_fields)]` (lib.rs:72) | 9/9 ok | `a_line_with_an_unknown_field_is_refused` FAILED |
| M3 `#[serde(default)]` on `text` (lib.rs:77-79) | 9/9 ok | `a_line_without_text_is_refused` FAILED |

## 3. Suite run (after part 2 existed)

`cargo test -p ekr-observe --locked --no-fail-fast`, EXIT=101:

```
Running tests/adversary_observe.rs     test result: ok. 4 passed; 0 failed
Running tests/adversary_pass2.rs       test result: FAILED. 3 passed; 1 failed
Running tests/fixture_observations.rs  test result: ok. 5 passed; 0 failed
Doc-tests ekr_observe                  test result: ok. 0 passed
```

`before` = 9 is the same run with my file (`adversary_pass2`) left out. `cargo fmt -p ekr-observe -- --check` exit 0; `cargo clippy -p ekr-observe --all-targets --locked -- -D warnings` exit 0.

## 4. Findings (tree: 1577b3e7 plus untracked adversary_pass2.rs)

| id | file:line | what was measured | what reaches it | verdict | origin |
|---|---|---|---|---|---|
| F5 | crates/ekr-observe/src/lib.rs:116 | public `observe_line` with the line's `\n` still on it maps it, with a hash over the `\n` and a second id for the same record; the module doc (:10) says the hash leaves the `\n` out | nothing found: no caller outside the crate; `observe_jsonl` strips the `\n` before it calls | plausible | introduced |
| F6 | crates/ekr-observe/src/lib.rs:148 | M1 survives the existing suite | documented behaviour ("Empty input has no lines"); killed by the added case | confirmed | introduced |
| F7 | crates/ekr-observe/src/lib.rs:72 | M2 survives the existing suite | a fixture line with an extra field; killed by the added case | confirmed | introduced |
| F8 | crates/ekr-observe/src/lib.rs:77 | M3 survives; the comment promises a refusal nothing tested | a fixture line without `text`; killed by the added case | confirmed | introduced |
| F9 | crates/ekr-observe/src/lib.rs:60 | doc says ids collide only "if SHA-256 collides in its leading 128 bits"; `Builder::from_custom_bytes` overwrites 6 of them (version nibble and variant bits), so the id holds 122 bits of the hash | anyone reading the doc as a guarantee | confirmed (judgement, no case) | introduced |

Fixes, named and not applied: F5 — make `observe_line` refuse a line containing `\n`, or make it private. F6–F8 — none needed; the added cases are the fix. F9 — change the doc to "its leading 122 bits".

## 5. Attacked and not broken

- Pass-1 correction: the golden hash `2563db91…4f5a` matches `sha256sum` of `ekr.payload.v1` + line 1, recomputed on its own. The timestamp doc (lib.rs:32-34) matches the adversary case.
- Key encoding: the key is tagged and length-prefixed (`canonical.rs` rule 2), so field boundaries cannot collide.
- Blank/whitespace lines, `\n`-only input, line numbering, duplicate fields: already covered and holding.
- `captured_at` as a float or out of range: refused by `Timestamp` (transparent `i64`).
- Fresh-process determinism: pinned by the golden id.

## 6. Paths written outside the worktree

- <wave-root>/f/scratch/p2-mut/ (Cargo.toml, Cargo.lock, src/lib.rs, tests/*.rs — mutated copy, left unmutated)
- <wave-root>/f/scratch/p2-mut-target/ (build dir of the scratch copy)
- <wave-root>/f/scratch/p2-mutants.sh
- <wave-root>/f/scratch/adversary-pass-2.md (this report)
- <wave-root>/f/target/ (the assigned unit build dir, reused)

## 7. Findings block


## Coordinator routing (2026-09-27)

- F5 → back to the implementor: `observe_line` refuses a line that contains a newline, so the two entry points cannot give one record two ids.
- F6, F7, F8 → no-op: the adversary cases kill the mutants and stay.
- F9 → back to the implementor: the id doc says 122 bits of the hash.
- Second pass: the correction that answers it is verified by the coordinator, with no third attack.

```findings
[
{"file":"crates/ekr-observe/src/lib.rs","line":116,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"public observe_line hashes a trailing newline the module doc says is excluded, so one record gets a second id through that entry point; no caller reaches it today"},
{"file":"crates/ekr-observe/src/lib.rs","line":148,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"deleting the empty-input branch survived the existing suite"},
{"file":"crates/ekr-observe/src/lib.rs","line":72,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"dropping deny_unknown_fields survived the existing suite"},
{"file":"crates/ekr-observe/src/lib.rs","line":77,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"making text defaulted survived the existing suite despite the comment promising a refusal"},
{"file":"crates/ekr-observe/src/lib.rs","line":60,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"id doc claims 128 bits of hash, but the version-8 builder overwrites 6, leaving 122"}
]
```
