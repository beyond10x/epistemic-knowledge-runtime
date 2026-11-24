---
format: aep.planning-md/3
id: story:ekr-explain-evidence-fixes
kind: story
status: active
title: Explain and evidence fixes
summary: Explanations and quality samples return whole, pinned evidence
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T04:08:30Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T04:08:30Z", actor: "human:timo", revision: 3}
---
## Outcome

Explanations and quality samples return whole, pinned evidence.

## Work

Review of EKR 0.0.42, findings 19 20 21 23. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

19. The Rust SDK can return partial evidence as if the document ended there. For an 81,900-byte source, the CLI returned 65,536 bytes and marked the result truncated with its offset and full length. The typed SDK reader returned those same 65,536 bytes without any of the three fields. Expose the range and total length on the SDK result and support fetching the next range.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/read/kernel.rs#L429
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/read/mod.rs#L528
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/explain.rs#L403


20. The default MCP explanation can cut its own supporting quote in half. Format /2 selected a 65,536-byte window beginning at offset 8,056 and containing the whole 48-byte statement. Format /3, the MCP default, kept the leftmost 32,768 bytes at that same offset and ended midway through the statement. Preserve the match when shortening the window. A caller can retrieve it with /2 and an explicit range.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/explain.rs#L114
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/explain.rs#L242
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L348


21. An explanation cannot be pinned to a historical snapshot. An assertion had one source in the revision-1 snapshot. At head, explain included a second source that arrived at revision 2. The CLI refused --at, and the MCP tool rejected an at field. Honor a revision selector on both surfaces and include the revision used in the answer.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/explain.rs#L181
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/mcp.rs#L1044
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/runtime.rs#L129
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/explain.rs#L374


23. A quality sample omits evidence attached after the assertion was written. At the same revision, explain returned two sources for the assertion and sample gave the judge only the original one. Include the later evidence and its provenance when constructing the judged fact. Until then a reviewer needs a separate explanation for the complete evidence set.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-views/src/sample.rs#L368
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L1030
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2558


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
