# Coordinator checks

Read this before opening a wave, and again before its closing commit. It is not advice. Every entry
is a check with a command, and every one exists because it was skipped and cost something named.

**Check 6 caught a number in this file's own first draft.** It said "twenty-eight coordinator
errors" across six waves, which nothing in the store can compute. Running the check found that no
review-result carries an owner at all, and that the most valuable review of this phase had never been
recorded. Both are fixed below and by that recording.

The failures below are all the coordinator's. Two counts, both measured:

- the independent review of the merged P1 core (`review-result:independent-review-p1-core`) returned
  **13 findings, 11 of them the coordinator's**, attributed by its own `owner` field;
- wave p1-05's six adversary passes returned **28 findings, 9 the coordinator's**, attributed by
  reading them.

No other review's findings carry an owner, so there is no total to quote and this file does not quote
one.

**And the store cannot hold one.** A findings entry is
`{file, line, category, severity, verdict, origin, message}` and nothing else; `aep plan artifact new`
refuses an `owner` key by name. This file's first draft required one anyway — a rule written without
checking the tool supports it, which is the same class of error as everything below it. What is
required instead is in check 6.

## 1. A sentence claiming a document and code agree names the case that executes it

**Five false sentences in six waves**, every one found by a sub-agent:

| wave | the sentence | found by |
|---|---|---|
| p1-03 | `ontology.yaml` claimed a carrier for all four compound value kinds; it carries two | the adversary |
| p1-04 | `graph.yaml` claimed a payload carrier for four validation states; there is one | the implementor |
| p1-05 | ADR 0005 said a float stays legal in the transient graph while its own decision made it unholdable | the adversary |
| p1-05 | `graph.yaml` said a `String` field holds the bytes content hashes are computed over | the coordinator, by accident |
| p1-06 | ADR 0008 said the kernel's reference validator holds ids from outside Rust; the only such path sits below the kernel | the adversary |

**The check.** Before a sentence asserting agreement goes into an ADR, a task or a `systems/`
comment, answer in the same edit: *which case fails if this stops being true?* If none, write
`unexecuted` in the sentence. `crates/ekr-graph/tests/domain_projection.rs` is the pattern for making
one executable — it binds each declaration to the Rust types that project it.

A sentence with no case is not thereby wrong. It is unchecked, and saying so costs one word.

## 2. An acceptance case that constructs the forbidden value cannot survive the fix

**Wave p1-06** set four cases as exit criteria. Two could never be green: the run-time membrane case
and the compile-fail case pass a bare id into one constructor argument, so **one compiles exactly
when the other does**, and the compile-fail case failing to compile *is* the acceptance. The
implementor proved it and delivered a tree that would not build rather than narrow a specification it
did not own.

**Wave p1-04** had the same class one level up: the acceptance named a read whose documented meaning
could not hold without a clock the runtime does not have.

**The check.** For each case in an acceptance set, write one line: *what does this assert after the
fix lands?* Three answers and only three — it asserts the invariant and survives; it stages the
defect and dies; it asserts something else and should say so. A set where every case survives is a
set nobody checked.

## 3. A `systems/` change lands in every tree that must agree with it, in one action

**Twice.** Wave p1-04 left a `graph.yaml` correction in the coordinator tree while the crate quoting
it sat in the unit's. Wave p1-05 declared an event in the coordinator tree only, so the unit's gate
**and its own projection guard** both ran against a document declaring nothing — which is why
nothing caught the crate writing that event with one field where the declaration named three.

**The check.** After editing anything under `systems/`, before anything else:

```console
$ for T in <coordinator-tree> <every-unit-tree>; do cp <file> "$T/<file>"; done
$ diff -q <coordinator-tree>/<file> <unit-tree>/<file>
```

A document and the code that must agree with it are never in two trees at once.

## 4. A rule this wave adds applies to this wave

**Wave p1-06.** The opening commit's parent banned locating the repository with
`env!("CARGO_MANIFEST_DIR")`, because a guard reading the wrong checkout passes while checking
nothing. The same wave then rested a repaired invariant on a guard that uses it. The ban and the
repair landed together and nobody joined them.

**The check.** After the opening commit and before the first dispatch, grep the wave's declared
surface for whatever that commit just forbade:

```console
$ git show --stat HEAD           # what did this commit add a rule about?
$ grep -rn '<the banned thing>' <the wave's surface>
```

## 5. Scan your own store writes before you commit them, not at push

**Twice.** Wave p1-04: a real personal identifier in a fixture, through two adversary passes, two
corrections and six gate runs. Wave p1-06: an absolute home path quoted into a task body and written
twice into the **append-only** journal, so the file had to be reset and the artifact recreated.

`task check` does not scan text. The push-time gate is the only thing that does, and by then a
journal entry cannot be edited.

**The check.** Immediately after any `aep plan artifact new` or `body`:

```console
$ b10x-gates scan-text .engineering/planning/journal.jsonl <the artifact file>
```

Elide every absolute path in a body before writing it. `<worktrees>/…` and `<home>/…` read fine.

## 6. Every count, path and version in an artifact is measured in the command that writes it

**Wave p1-06.** A task said nineteen files and thirty-eight uses. It is twenty and thirty-nine, and
the twentieth is not a test. An independent reviewer measured it; the coordinator had estimated.

**The check.** A number in an artifact comes from a command in the same shell invocation that writes
the body, or it does not go in. Not from a previous message, not from an agent's report unless the
report is quoted as its source.

**Every `review-result` body states the owner split in prose, in a line beginning `Owners:`.** The
findings schema has no `owner` key and refuses one, so the count lives in the body where a reader and
a `grep` can both find it:

```
Owners: 13 findings, 11 coordinator, 2 implementor.
```

Without it the store holds what was found and not who caused it, which is the number that says
whether the checks above are working. It cost nothing to add and it was not added for five waves.

## 7. What the sub-agents are for, and what it costs to overrule them

Across this phase, an implementor or adversary was right against the coordinator **every time they
disagreed**: two refusals to narrow a case they did not own, a measurement that disproved a mechanism
claim in a correction brief, a second measurement that disproved the replacement, an exhaustiveness
claim widened past what was reported, and a vacuous case flagged rather than satisfied.

**The check.** When a sub-agent contradicts something the coordinator wrote, verify the contradiction
before answering it, and record the verification. Not one of these has been wrong yet.
