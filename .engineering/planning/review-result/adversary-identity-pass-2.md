---
format: aep.planning-md/2
id: review-result:adversary-identity-pass-2
kind: review-result
status: active
title: Adversary, story:kernel-identity-and-hashing, pass 2
tags:
- adversary:aep-drive:adversary
relations:
- reviews: story:kernel-identity-and-hashing
revision: 1
---
```
unit: story:kernel-identity-and-hashing (story revision 11) — uncommitted working tree home-path:sha256:ebcb5c7535cf87d3081780c3bb65de3318669ebe2e287bd0c05ee853ec531466, base 2f2acec
verdict: NEEDS-CHANGE
cases: executed 79→89, red 3
origin: introduced 4 / pre-existing 0 / undecided 0
wrote-outside-worktree: 9 paths (part 6)
needs-coordinator: no
```

## 1. Scope of what I touched

`git --no-pager diff --stat` is not scope proof here — the implementation is uncommitted and three of its four source files are untracked, so the diff shows only the implementor's tracked edit:

```
 crates/ekr-core/src/lib.rs | 30 ++++++++++++++++++++++++++++++
 1 file changed, 30 insertions(+)        <- the implementor's, not mine
```

The proof is a byte comparison against a copy taken before any probe ran, plus mtimes (my session began 13:12):

```
diff -r  <scratch>/crates/ekr-core/src            <worktree>/crates/ekr-core/src            -> src IDENTICAL
diff -r -x 'adversary2_*'  <scratch>/tests        <worktree>/tests                          -> tests IDENTICAL

crates/ekr-core/tests/adversary2_encoding_vector.rs            13:12:49   added by me
crates/ekr-core/tests/adversary2_public_surface.rs             13:12:57   added by me
crates/ekr-core/tests/adversary2_revision_number_text_form.rs  13:14:05   added by me
```

Every path I added is a test file. No implementation file was edited; every mutation was on a copy in scratch, and every copy was restored and re-diffed.

## 2. The cases I added, and their red output when written

**`crates/ekr-core/tests/adversary2_encoding_vector.rs`** — 6 cases, **4 green by design** (they pin the ten tag bytes no vector reaches), **2 red**. Run alone, before any suite run:

```
running 6 tests
test a_content_hash_encodes_as_its_published_bytes ... ok
test a_content_address_over_the_remaining_tags_matches_its_published_digest ... ok
test a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed ... FAILED
test a_raw_payload_never_shares_an_address_with_a_canonical_value ... FAILED
test an_id_encodes_as_its_published_bytes ... ok
test the_remaining_tags_match_their_published_bytes ... ok

---- a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed stdout ----
panicked at crates/ekr-core/tests/adversary2_encoding_vector.rs:147:5:
assertion `left == right` failed: the writer's order reached the bytes, which rule 3 says it cannot
  left: "0a0000000000000002 06...61 04...01 06...61 04...02"
 right: "0a0000000000000002 06...61 04...02 06...61 04...01"

---- a_raw_payload_never_shares_an_address_with_a_canonical_value stdout ----
panicked at crates/ekr-core/tests/adversary2_encoding_vector.rs:168:5:
assertion `left != right` failed: a raw payload took a structured value's content address
  left: ContentHash([0, 216, 211, 150, 12, 0, 130, 49, ...])
 right: ContentHash([0, 216, 211, 150, 12, 0, 130, 49, ...])

test result: FAILED. 4 passed; 2 failed
```

The four green ones are hand-derived — `01 02 03 05·ff×16 07… 09… 0b 0c… 0d… 0e…`, digest `9aeb0c0b…e1b3` from `sha256sum` outside the crate. They matched the implementation byte for byte on the first run.

**`crates/ekr-core/tests/adversary2_public_surface.rs`** — 1 case, **RED**:

```
running 1 test
test every_public_item_is_used_by_a_case_and_not_only_named_in_prose ... FAILED

panicked at crates/ekr-core/tests/adversary2_public_surface.rs:111:5:
assertion `left == right` failed: public items no case uses; `no_public_item_is_untested` passes
for each of them on the strength of an English word in a doc comment or an assertion message
  left: ["list"]
 right: []

test result: FAILED. 0 passed; 1 failed
```

**`crates/ekr-core/tests/adversary2_revision_number_text_form.rs`** — 3 cases, **all green**. I wrote them expecting `serde_json::from_str::<RevisionNumber>("-0")` to be accepted while `"-0".parse()` is refused. It is not — serde refuses it. The theory died on first run and I am not reporting it. The file stays because `a_leading_zero_is_refused_at_every_length` closes a live mutant gap (part 4, finding 4).

## 3. The suite run — after the cases in part 2 existed

```
$ cd <worktree> && CARGO_TARGET_DIR=home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe task check
...
failures:
    a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed
    a_raw_payload_never_shares_an_address_with_a_canonical_value

test result: FAILED. 4 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p ekr-core --test adversary2_encoding_vector`
task: Failed to run task "check": task: Failed to run task "test": exit status 101
EXIT=201
```

`task test` is fail-fast, so it stops before `adversary2_public_surface`. Run alone: `fmt-check` **EXIT=0**, `clippy` **EXIT=0**, `doc-check` **EXIT=0**, `spec-check` **EXIT=0**, `plan-check` **EXIT=0**. The only red step is `test`, and the only red cases are mine.

Counts from `cargo test --workspace --locked --no-fail-fast` (EXIT=101), run after my cases existed: **89 executed, 3 failed**. Deselecting my three test targets (`adversary2_encoding_vector`, `adversary2_public_surface`, `adversary2_revision_number_text_form`, 10 cases) gives **before = 79**.

## 4. Findings

Covering the uncommitted working tree above. `git show 2f2acec:<path>` reports every file named below as absent at the base, so origin is `introduced` with certainty.

| # | file:line | carried / new | What is wrong | What was measured | What reaches it | Verdict | Origin |
|---|---|---|---|---|---|---|---|
| 1 | `crates/ekr-core/tests/public_surface.rs:13` | **new** (residue of pass-1 finding 2) | The check added to close finding 2 claims "a *new* public item cannot arrive untested without turning a case red". Its criterion is that the name appears as a word anywhere in `tests/*.rs`, so any item whose name is an ordinary English word is satisfied by prose. The tree already contains one: `Encoder::list` is called by no case, and the word "list" in three doc comments and one assertion message is what passes it. `the_encoder_writes_each_shape_it_publishes` also omits `list`, `map` and `set` from the twelve writers it enumerates | `adversary2_public_surface.rs:111`, red, `left: ["list"]`. Scratch probe: adding `pub fn count` to `Encoder`, exercised by nothing, leaves `no_public_item_is_untested` green (the word "count" is in a doc comment). A second probe: a public item in `src/probe/mod.rs` named nothing in the suite is also green — `rust_sources` does not recurse into `src/` subdirectories | the check is a gate case and it is the crate's only machine guard against an untested public item; the next story that adds one to `ekr-core` is the caller | CONFIRMED | introduced |
| 2 | `crates/ekr-core/src/canonical.rs:197` (`map`) vs `:16` (rule 3) | **new** | The unit wrote both halves in one round and they disagree. Rule 3: "two maps with the same entries encode identically whatever order they were built in". `Encoder::map`: "the sort is stable, so entries sharing a key keep the order they were handed in." For a repeated key the writer's order is the encoding, which is the thing rule 3 says cannot happen — the same class the correction round was opened to fix | `adversary2_encoding_vector.rs:147`, red — two entries, one key, two orders, two byte strings | **nothing found.** I built the duplicate-key iterator. A `HashMap` or `BTreeMap` cannot produce one; the doc mentions duplicates because the implementor anticipated a caller over an association list, and no such caller exists | INFEASIBLE (constructed state; no caller) | introduced |
| 3 | `crates/ekr-core/src/hash.rs:32` (`of_bytes`) vs `canonical.rs:12` (rule 1) | **new** | Rule 1 exists so "a string and a number that read the same do not encode the same". `ContentHash::of_bytes` writes no tag and shares one address space with `ContentHash::of`, so a raw payload whose bytes are some value's canonical encoding takes that value's address. `content_hash.rs:40` asserts the composition as desirable — "hashing a value is hashing its canonical bytes and nothing else" — which is the same statement read the other way round | `adversary2_encoding_vector.rs:168`, red — `ContentHash::of_bytes(&"knowledge".canonical_bytes()) == ContentHash::of(&"knowledge")`, identical 32 bytes | `of_bytes` is public and the story's Notes name it as the path for observations and evidence, which are externally supplied payloads; no in-tree caller composes the two, and the bytes must be chosen to collide | CONFIRMED | introduced |
| 4 | `crates/ekr-core/tests/adversary_encoding_vector.rs:31` | **new** (residue of pass-1 finding 2) | The published vector pins `MAP`, `LIST`, `STRING`, `UNSIGNED`. The other ten tags are unpinned, including `tag::ID` and `tag::HASH` — the encodings of the two types this crate exists to publish. Separately, `RevisionNumber::from_str`'s leading-zero guard is `text.len() > 1`, and no case reaches `"00"` | scratch copies: `tag::ID 0x0d→0x7d` → all 67 implementor cases green; `tag::HASH 0x0e→0x7e` → all 67 green; `from_str` `> 1 → > 2` (so `"00"` parses to `SEED`) → all 67 green. My four green cases in `adversary2_encoding_vector.rs` catch the first two, `a_leading_zero_is_refused_at_every_length` catches the third | any future change to `canonical.rs`'s constants or that guard — a suite blind spot, not a live defect | NEEDS-CHANGE | introduced |
| 5 | `crates/ekr-core/tests/public_surface.rs:25` (`rust_sources`) | **new** | The scan is documented as reading "the crate's own source" and reads only `src/*.rs`, non-recursively | scratch probe above: `src/probe/mod.rs::zarquon_untested_probe`, a name appearing nowhere in the suite, leaves `no_public_item_is_untested` green | **nothing found** — `src/` is flat today; a later module directory reaches it | INFEASIBLE (no such state in the tree) | introduced |

**Named fixes** (I applied none): (1) require a call-shaped occurrence (`.name` / `::name`) rather than a bare word, and add `out.list/map/set` to `the_encoder_writes_each_shape_it_publishes`; (2) either drop the duplicate-key sentence and sort duplicates by encoded value, or qualify rule 3 to say the entries must be a map; (3) domain-separate — give `of_bytes` its own tag, or fold the question into `task:canonical-newtype-discriminant`, which is the same address-space question one level down; (4) keep my four vector cases and my leading-zero case; (5) recurse in `rust_sources`.

### My five pass-1 findings, restated

| pass-1 finding | now |
|---|---|
| 1, `FromStr` accepts five further spellings | **resolved.** `adversary_id_text_form.rs` green; all six of its spellings refused for `FromStr` and serde. The fifteenth member, `RevisionNumber`, is fixed too and I could find no second spelling it accepts on either path |
| 2, nothing pins the canonical encoding | **partly resolved.** The vector and `to_hex` case are kept and green; the class is still open for ten of fourteen tags and for `no_public_item_is_untested` itself — findings 1 and 4 |
| 3, `Encoder::map`/`set` do not impose their order | **resolved.** `adversary_encoder_order.rs` green; both sort. Finding 2 is the one corner the new doc exempts itself in |
| 4, vacuous acceptance | **resolved.** With `mint() -> Self(0)` all five `rename_stability.rs` cases go red (measured), where the old pair went green. The restated acceptance is stated by `two_values_that_share_a_name_never_share_an_id` and is mutation-sensitive |
| 5, structural encoding | **accepted as documented.** `canonical.rs` rule 5 names `task:canonical-newtype-discriminant`; I read the task in the coordinator tree and it exists at revision 1 and does `blocks story:commit-and-revision-lineage`. Finding 3 is the same question for the untagged `of_bytes` entry point and belongs with it |

### The edit to my pass-1 case — I agree with the coordinator's reading

`adversary_id_text_form.rs::a_second_spelling_does_not_round_trip_to_itself`'s premise was `expect("accepted today")` on the urn form; once `FromStr` is strict that premise cannot hold, and the case could not have been left as it was. The replacement asserts the urn form is refused *and* keeps the round-trip assertion over the surviving spelling. Nothing was removed or weakened. No finding.

## 5. Attacked and could not break

- **The strict parser.** `is_canonical_uuid_text` is exact: 36 bytes, hyphens only at 8/13/18/23, `0-9a-f` elsewhere. Uppercase, simple, braced, urn, non-ASCII, Unicode digits, leading/trailing whitespace, a 36-character non-UUID, an embedded NUL — all refused. No accept-regression: `Uuid::nil()`, `Uuid::max()` and the implementor's all-bits proptest round-trip.
- **`RevisionNumber`.** `0`, `u64::MAX`, and `u64::MAX + 1` behave; `+7`, `007`, `-0`, ` 7`, `7 `, `7\n`, `""`, `0e0`, `7.0` refused on both paths; `FromStr` and `Deserialize` agree on every text I could construct.
- **The sort.** `map` sorts stably by `K::cmp`, `set` by `T::cmp`; both agree with `BTreeMap`/`BTreeSet` iteration for every `Canonical + Ord` type in the crate. I found no pair of equal maps that encode differently and no pair of different maps that encode identically, outside the duplicate-key corner (finding 2) and the documented structural identity (rule 5). Sorting by key rather than by encoded bytes holds up: `Ord` is total and Eq-consistent for every key type the crate publishes.
- **The restated acceptance.** The proptest quantifies over two independent `".{0,32}"` draws and does include the equal-name case — deterministically, via the persisted `cc 1f4360d4… # shrinks to left_name = "", right_name = ""` regression. `no_id_type_mints_a_constant` enumerates all fourteen by name and does not sample; `every_ess_id_type_exists_in_the_crate` holds the sibling enumeration to the three ESS domain files.
- **Determinism and ambiguity of the encoding.** Unchanged from pass 1 and re-walked against the sort: no float, clock, pointer, allocation or hash-seed path reaches the bytes; the format stayed prefix-free across all fourteen tags.

## 6. Paths written outside the worktree

All under my assigned scratch `home-path:sha256:56b6888c170eec36decef94ef3c81ba9f30641c94e0de9a0afcf3cccd02d7e9b`:

```
adv2-mutant/                 mutable copy of crates/ekr-core + Cargo.toml + systems/ekr
adv2-mutant/canonical.rs.orig, adv2-mutant/identity.rs.orig    pre-probe copies, for the restore diff
adv2-vector.hex.txt          the hand-derived vector, before sha256sum
adv2-gate.log  adv2-fulltest.log  adv2-before.log  adv2-case-c.log
adv2-fmt-check.log  adv2-clippy.log  adv2-doc-check.log  adv2-spec-check.log  adv2-plan-check.log
```

`adv2-mutant/crates/ekr-core/src` is byte-identical to the worktree (all five mutations reverted, verified with `diff -r`). `home-path:sha256:1c595caf81ea2047fb7bccc54c5a0e02d8e12f5f9745a81d6a63f51975f15d10` (scratch build dir) was created and **deleted by me**; `/` is at 61G free. The shared `home-path:sha256:b75f9a061b971f23993e7f85d8bf63ed6eff15c82c05ab13d3f33e64656bc3fe` was used as the brief directs and left in place. No planning-store write, no `aep plan artifact` write verb, no commit, no branch or worktree command. Lease `ekr-adversary-identity-2` acquired at start and released.

```findings
- file: crates/ekr-core/tests/public_surface.rs
  line: 13
  category: mutant
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: "no_public_item_is_untested claims a new public item cannot arrive untested without turning a case red, but its criterion is a bare word anywhere in the suite text, and Encoder::list already passes it on the strength of the English word list in three doc comments and one assertion message while no case calls it."
- file: crates/ekr-core/src/canonical.rs
  line: 197
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "Rule 3 says two maps with the same entries encode identically whatever order they were built in and Encoder::map says entries sharing a key keep the order they were handed in, so for a repeated key the writer's order is the encoding; no caller in this tree can produce a duplicate key, so the red case constructs the state."
- file: crates/ekr-core/src/hash.rs
  line: 32
  category: property
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "ContentHash::of_bytes writes no tag and shares one address space with ContentHash::of, so a raw payload whose bytes are a value's canonical encoding takes that value's content address - the collision rule 1 of canonical.rs exists to prevent, on the entry point the story's Notes name for observation and evidence payloads."
- file: crates/ekr-core/tests/adversary_encoding_vector.rs
  line: 31
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "Ten of the fourteen tag bytes are pinned by no vector, including tag::ID and tag::HASH - changing either leaves all 67 implementor cases green while every recorded address containing an id or a sub-hash moves - and RevisionNumber's leading-zero guard survives len > 1 becoming len > 2, which makes 00 a second spelling of the seed, also with the suite green."
- file: crates/ekr-core/tests/public_surface.rs
  line: 25
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "rust_sources is documented as reading the crate's own source and reads only src/*.rs without recursing, so a public item in a module directory is invisible to the coverage check; src/ is flat today, so nothing reaches it yet."
```
