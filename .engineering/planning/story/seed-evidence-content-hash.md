---
format: aep.planning-md/1
id: story:seed-evidence-content-hash
kind: story
status: active
title: The binary prints the content hash a seed evidence entry needs
relations:
- serves: vision:o5
- derived_from: task:agent-cannot-add-evidence
scope:
- confidence: inferred
  path: crates/ekr-core/src/hash.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/seed.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent.rs
- confidence: inferred
  path: crates/ekr/src/cli/examples/seed.yaml
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/exit.rs
- confidence: inferred
  path: crates/ekr/tests/adversary_p4_01_agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/agent_cli.rs
revision: 14
---
## Context

`task:agent-cannot-add-evidence`: an agent using only the binary cannot add evidence for a new claim. P1 admits evidence only through the seed, the seed entry needs a `content_hash` whose algorithm the binary never states, and the refusal names neither the algorithm nor the expected value. The proper route (observations) is P2; this story is the short-term half.

## Acceptance

- `ekr` prints the content hash the seed expects for a payload (file or stdin), and `ekr guide` says how a new evidence entry is added to a seed.
- `seed-evidence-payload-mismatch` names the expected and the found hash.
- A blind agent trial with only the binary adds a new evidence entry to a seed and cites it from a committed assertion.

## Scope

Derived 2026-09-25 by `story-scoper` (wave p1-15). Confidence: medium; the refusal site is cited, the new verb shape is open.

- hash: `ContentHash::of_bytes` (`crates/ekr-core/src/hash.rs:73`) = SHA-256 over `b"ekr.payload.v1"` (`PAYLOAD_DOMAIN`, :53) then the bytes, 64 lowercase hex (`to_hex`, :100); read-only use — cited
- `crates/ekr-kernel/src/seed.rs:197-204` raises `seed-evidence-payload-mismatch` via `invalid()` (:81) into `SeedError::Invalid(String)` (:62), no hashes carried — cited
- `crates/ekr/src/exit.rs:88` passes the reason into `ekr.kernel.InvalidSeed`; `kernel.yaml:892` already has `code` and `reason`, so no contract change — cited
- replay raises the same code at `crates/ekr-kernel/src/commit.rs:71` — cited; in scope only by judgement
- new verb: `Command` and `execute()` in `crates/ekr/src/cli/mod.rs` (config-free verbs at :248); guide `GUIDE` in `crates/ekr/src/cli/agent.rs:11` (evidence line :86, OUTPUT :95) — inferred
- tests: `crates/ekr/tests/agent_cli.rs:563` (a help row per verb), guide checks :198 and `guide_prose` :1166; `crates/ekr/tests/adversary_p4_01_agent_cli.rs:265` (config-free verb list); `crates/ekr-kernel/tests/seed.rs` :509, :870 match with `contains` — inferred
- `crates/ekr/src/cli/examples/seed.yaml:150-178` — inferred
