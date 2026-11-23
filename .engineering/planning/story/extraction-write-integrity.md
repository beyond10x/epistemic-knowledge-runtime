---
format: aep.planning-md/3
id: story:extraction-write-integrity
kind: story
status: active
title: Extraction writes the same state in any batching
summary: Evidence id reuse, concurrent imports, batch order, alias replacement and single-valued adds keep one correct active value with its own evidence (review 0.0.42 findings 1-4, cardinality-One defect)
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-23T20:19:39Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-23T20:19:39Z", actor: "human:timo", revision: 3}
---
## Outcome

An extraction writes the same store state whether its facts arrive in one batch or one by one,
in one process or two, and every committed fact cites the evidence it was extracted from.

## Work

Review of 0.0.42, findings 1-4, plus one defect found by the cortex consumer:

- 1: a reused evidence id with a different payload attaches the old source to a new fact. Compare
  the proposed evidence fields and payload hash with the retained record before any write; an
  identical retry still succeeds, a differing one is refused by name.
- 2: two concurrent imports of the same facts leave twice the active assertions. Recheck claim
  identity against the revision being committed when another writer advanced the head.
- 3: a batch of ordered facts (add, replace, add) keeps a different replacement than the same
  facts submitted one by one. Preserve order for pending replacements and later writes to the same
  node and property.
- 4: two aliases of one node break a replacement within one document. Track earlier facts by the
  resolved node id and property.
- Cardinality One: an unmarked fact on a single-valued property is planned as an add, and the
  kernel's cardinality check lets it through, leaving two active values.

## Acceptance

For each item, a test red on main: a differing reuse of an evidence id is refused and an identical
retry holds; two concurrent imports (File and SQLite) end with one active assertion per claim; the
batch and the one-by-one order leave the same active values; a replacement under a second alias
of the node succeeds; a second value of a single-valued property supersedes the first.
