---
format: aep.planning-md/2
id: review-result:adversary-p2p3p4-03-resolve-pass-1
kind: review-result
status: active
title: Adversary pass 1 on unit S (ekr resolve)
relations:
- reviews: story:ekr-resolve-verb
revision: 1
---
unit: story:ekr-resolve-verb, worktree ekr-wave-b-s at b7966ce7 (unit diff wave/p2p3p4-03...5ce93aff) plus one untracked adversary test file
verdict: red (2 confirmed findings, each with a red case; 1 test gap closed by a green case)
cases: executed 255→258, red 2
origin: introduced 3, pre-existing 0, undecided 0
wrote-outside-worktree: 3 paths under <scratch>/ (listed in part 6)
needs-coordinator: no

## 1. Diff stat

`git --no-pager diff --stat`: empty, because the only change is one untracked file. `git status --short`:

```
?? crates/ekr/tests/adversary_s_resolve.rs
```

It is a test file. No implementation file was touched.

## 2. Cases added (crates/ekr/tests/adversary_s_resolve.rs)

| case | asserts | now |
|---|---|---|
| `an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit` | a 100 161-byte reference (one 100 000-byte anchored alias plus 20 `*a`) decodes to 2.1 MB, twice `LIMIT`, so it must be refused with exit 1 and `ekr: typed reference bomb.yaml: ` | red |
| `the_schema_and_the_reader_agree_on_scalar_aliases` | for aliases `123`, `~`, `!Name acme`, `007` and a null `aliases:`, the reader (`serde_yaml_ng::from_str::<TypedReference>`, the call at resolve.rs:32) accepts exactly when `ekr schema typed-reference` accepts the JSON projection, which is what the schema's description says it validates | red |
| `a_document_over_the_limit_is_refused_even_when_its_prefix_decodes` | a document of `LIMIT`+5 bytes whose first `LIMIT`+1 bytes decode as a reference is refused, exit 1, `over 1048576 bytes`. If the check at resolve.rs:29 is deleted, the truncated prefix resolves. No existing case covers that line | green |

Red run, this file alone, before the suite (`cargo test -p ekr --locked --test adversary_s_resolve`):

```
running 3 tests
test a_document_over_the_limit_is_refused_even_when_its_prefix_decodes ... ok
test the_schema_and_the_reader_agree_on_scalar_aliases ... FAILED
test an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit ... FAILED

---- the_schema_and_the_reader_agree_on_scalar_aliases stdout ----
thread 'the_schema_and_the_reader_agree_on_scalar_aliases' (1950898) panicked at crates/ekr/tests/adversary_s_resolve.rs:132:5:
the typed-reference schema and its reader disagree:
a numeric alias: reader accepts (Ok(["123"])), schema refuses on {"aliases":[123],"type_id":"00000000-0000-4000-8000-000000000201"}
a null alias: reader accepts (Ok(["~"])), schema refuses on {"aliases":[null],"type_id":"00000000-0000-4000-8000-000000000201"}
a tagged alias: reader accepts (Ok(["acme"])), schema refuses on {"aliases":[{"!Name":"acme"}],"type_id":"00000000-0000-4000-8000-000000000201"}
aliases written as null: reader accepts (Ok([])), schema refuses on {"aliases":null,"type_id":"00000000-0000-4000-8000-000000000201"}

---- an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit stdout ----
thread 'an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit' (1950828) panicked at crates/ekr/tests/adversary_s_resolve.rs:89:5:
assertion `left == right` failed: a 100161-byte document decoding to 2100000 bytes of aliases was resolved (stdout 100107 bytes):
  left: Some(0)
 right: Some(1)

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.75s
```

Before I wrote the case, I measured the amplification with the built binary: a 105 KB document with 1 000 `*a` peaked at 211 528 KB RSS and exited 0. Scaled to the 1 MiB input the reader admits (a 500 KB anchor plus about 100 000 aliases), it decodes to about 50 GB.

## 3. Suite run (after the cases existed)

`CARGO_TARGET_DIR=<build-dir> CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 CARGO_INCREMENTAL=0 nice -n 19 cargo test -p ekr --locked --no-fail-fast` gave exit status 101. Summed `test result` lines: 256 passed, 2 failed, 0 ignored. The only failing target:

```
     Running tests/adversary_s_resolve.rs
test a_document_over_the_limit_is_refused_even_when_its_prefix_decodes ... ok
test the_schema_and_the_reader_agree_on_scalar_aliases ... FAILED
test an_anchor_repeated_by_aliases_does_not_decode_past_the_input_limit ... FAILED
test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s
error: test failed, to rerun pass `-p ekr --test adversary_s_resolve`
```

`<before>` = 258 − 3, which is the same run with the `adversary_s_resolve` target excluded. The full log is `<scratch>/suite-pass1.log`.

## 4. Findings

| id | file:line | verdict | origin | what was measured | what reaches it |
|---|---|---|---|---|---|
| F1 | crates/ekr/src/cli/resolve.rs:32 | confirmed | introduced | `LIMIT` (resolve.rs:16, :29) bounds bytes read, not what decodes. YAML anchors expand, so 100 KB decodes to 2.1 MB and exits 0 (adversary_s_resolve.rs:89). At 1 MiB the amplification is about 50 000x | any `ekr resolve <file>` or `ekr resolve -` (stdin). An agent or a pipe supplies the document. Fix: refuse anchors and aliases in the document (the vendored `serde_yaml_ng::observation` facade exposes alias positions), or cap the total decoded alias bytes at `LIMIT` |
| F2 | crates/ekr/src/cli/resolve.rs:77 | confirmed | introduced | the schema's description (resolve.rs:115-119) says it validates the document "read as YAML and written as JSON". The reader accepts `aliases: [123]`, `[~]` (as `"~"`), `[!Name acme]` (drops the tag) and `aliases:` (null, read as `[]`), and the schema refuses all four (adversary_s_resolve.rs:132) | an agent that checks its reference with `ekr schema typed-reference` and has a numeric alias (a code, a number) sees it refused, although `ekr resolve` accepts it. Also, `~` resolves as the literal alias `"~"`. Fix: either the reader refuses non-string scalars, tags and a null list (decode through the observation facade), or the schema and docs state the scalar coercion |
| F3 | crates/ekr/src/cli/resolve.rs:29 | confirmed | introduced | no case in the unit's suite reaches the over-limit branch. If the branch is deleted, `take(LIMIT + 1)` truncates silently and a truncated prefix resolves. The new green case catches that mutant, shown by its own prefix-decodes assertion. I did not build a mutated binary | a document over 1 MiB given to `ekr resolve`. No fix needed. The gap is now covered by a green case |

## 5. Attacked and not broken

| attack | result |
|---|---|
| Store bytes | sha256 of every file was identical after resolve on file and sqlite, after a commit, with `--at 0`, `--at 1`, a refused reference and a missing revision |
| `--at` | `u64::MAX` and `1` on a fresh store give `ekr.kernel.RevisionNotFound`, exit 2. `-1` and `abc` are clap usage errors, exit 2, which is the same for every verb |
| Duplicate keys | `type_id` or `aliases` written twice is refused, exit 1 (`duplicate field`). Merge key `<<` is refused as an unknown field. Multiple documents are refused |
| Non-UTF-8, BOM, empty document, a directory | exit 1, the fault names the document |
| Refusal order and stdout/stderr separation | matches docs/cli.md (checked through the unit's cases) |
| The example reference | against the example seed it gives `ProposeNew` for the `CreateNode` example's node |
| Worked-example step 8 | Resolved and ProposeNew as declared |
| Guide text | matches `docs/cli.md` |

## 6. Paths written outside the worktree

- `<scratch>/probe1/` (probe store, host, seed and probe documents)
- `<scratch>/probe2/` (two-backend byte-identity probe)
- `<scratch>/suite-pass1.log`
- `<scratch>/adversary-pass-1.md` (this report)
- Test build output under `<build-dir>/` (the unit's own build dir)

## 7. Findings block


## Coordinator routing (2026-09-27)

- F1 → back to the implementor: the typed-reference reader refuses YAML anchors and aliases, the way the other document readers of `ekr` do where they already do, so the decoded size is bounded by the input size.
- F2 → back to the implementor: the reader refuses what the schema refuses (a non-string alias scalar, a tag, a null `aliases`), so the schema and the reader agree.
- F3 → no-op: the adversary case covers the over-limit branch and stays.

```findings
[
{"file":"crates/ekr/src/cli/resolve.rs","line":32,"category":"security","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the 1 MiB input limit bounds bytes read, not decoded size; YAML anchors let a document under the limit decode to about 50000 times it"},
{"file":"crates/ekr/src/cli/resolve.rs","line":77,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the reader coerces numeric, null and tagged aliases and a null aliases list, all of which the typed-reference schema refuses on the JSON projection it says it validates"},
{"file":"crates/ekr/src/cli/resolve.rs","line":29,"category":"test-gap","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"no case reached the over-limit refusal, so deleting it (and silently resolving a truncated prefix) survived the suite; now covered by a green case"}
]
```
