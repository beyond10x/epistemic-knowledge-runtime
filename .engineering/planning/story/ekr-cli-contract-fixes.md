---
format: aep.planning-md/3
id: story:ekr-cli-contract-fixes
kind: story
status: active
title: CLI schema read, JSON output and docs match the release
summary: The CLI's schema read, its JSON output and the published docs match what the release does
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T08:05:01Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T08:05:01Z", actor: "human:timo", revision: 3}
---
## Outcome

The CLI's schema read, its JSON output and the published docs match what the release does.

## Work

Review of EKR 0.0.42, findings 33 35 43 44. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

33. The CLI schema read omits declared operations and their arguments. The released seed declares Organization.record_review(note). Ontology, snapshot and overview did not return this operation. The native kernel can read the full definition. Offer a complete CLI schema read for callers that need to discover operation signatures and lifecycle transitions.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/examples/seed.yaml#L35
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/ontology.rs#L82
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/runtime.rs#L159
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-ontology/src/schema.rs#L164


35. The README promises whole-document extraction and replay behavior the CLI does not provide. In File and SQLite, one good relation and one bad evidence reference advanced revision 0 to 2 by default. Strict mode refused that reader error at revision 0. A separate strict input reached kernel validation, committed a new endpoint, then rejected its only relation. Repeating that input recorded another rejection. Document when partial writes occur and which report fields callers must check.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/README.md#L57
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L419
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/domains/integrate.yaml#L493
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/extraction_cli.rs#L1154
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L447
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/extraction_cli.rs#L1646


43. The deployed homepage points users to the previous release. The homepage labels 0.0.41 the current release. Its deployment record points to the main commit reviewed here, while GitHub’s latest release is 0.0.42. Generate the version from release metadata or identify 0.0.41 as the release that introduced merge.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/website/index.html#L84
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/xtask/src/bin/ekr-docs.rs#L102
https://beyond10x.github.io/epistemic-knowledge-runtime/#capabilities
https://beyond10x.github.io/epistemic-knowledge-runtime/.well-known/b10x-site.json
https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.42


44. Three bootstrap commands contradict the README’s JSON contract. A successful guide and operations returned text, and example ekr-seed/2 returned YAML. The schema command returned JSON as documented. Name these output formats in the README so bootstrap clients choose the correct decoder.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/README.md#L13
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L139
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/mod.rs#L960


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
