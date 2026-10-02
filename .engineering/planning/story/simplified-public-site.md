---
format: aep.planning-md/3
id: story:simplified-public-site
kind: story
status: implemented
title: Publish a simple EKR project documentation site
relations:
- serves: vision:o2
- serves: vision:o5
scope:
- confidence: inferred
  path: .github/workflows/b10x-docs-site.yml
- confidence: inferred
  path: .github/workflows/pages.yml
- confidence: inferred
  path: .gitignore
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: inferred
  path: website/
- confidence: cited
  path: xtask/Cargo.toml
- confidence: inferred
  path: xtask/src/bin/ekr-docs.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T17:50:56Z", actor: "agent:codex-ekr-public-docs", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T17:50:56Z", actor: "agent:codex-ekr-public-docs", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-02T18:31:17Z", actor: "agent:codex-ekr-public-docs", revision: 8, decided_on: {"recorded":{"test_result":1,"deployment_result":1}}}
---
## Outcome

Publish a concise, responsive project site at https://beyond10x.github.io/epistemic-knowledge-runtime/, following the repository-owned static site and pinned shared publisher used by Mantle and Codegate. The operator requested creation of similar simplified public website documentation.

## Content

Explain typed, evidence-backed, revisable knowledge; the propose, validate, commit membrane; file and SQLite stores; temporal reads and explanations; the CLI, Rust SDK and read-only MCP. Give the existing README first-run example and distinguish shipped functionality from future ingestion and schema discovery. Link to the source-owned detailed guides. Sources: README.md, docs/cli.md, docs/sdk.md and systems/ekr/. No product behavior or domain entity is introduced.

## Scope

website/index.html and website/styles.css; a Rust clap builder under xtask/src/bin/; Taskfile.yml site validation/build; README.md entry link; .gitignore build output; repository-owned Pages build and pinned publication callers. GitHub Pages configuration through the bot API is authorized by the public-site request. No release or shared Website/Atlas source change is needed.

## Acceptance

The authored page has working local anchors, the correct project asset base, mobile layout, keyboard focus and no scripts. Its quickstart runs against the actual ekr binary and commits then explains the example assertion. The Rust builder emits exact source bytes and a b10x-project-site/v1 manifest bound to the full Git commit. task check passes. The published HTML/CSS and both live provenance documents match the approved source and pinned publisher. Report the live URL only after verification.

## Delivery

Use a managed tree and bot commits, PR and merge. Existing required correctness/common checks remain in force. Configured Connectors adapters do not include GitHub; bot-authenticated Gates API is the declared integration fallback. Retain compact verification evidence in AEP. This is one bounded documentation story; no multi-item decomposition or critic panel is required.

## Implementation and verification in progress

Added a script-free landing page with an overview, a propose/validate/commit diagram, installation and first-run commands, capabilities, current limits and source-owned guide links. The Rust clap builder lives in xtask/src/bin/ekr-docs.rs; xtask retains default-run so cargo xtask doctor remains compatible. Task check validates the site. The existing CLI example executor now also reads the actual HTML quickstart and runs it on both storage providers.

Browser inspection at desktop and mobile widths found no horizontal page overflow, no missing local anchors and no scripts. Screenshots were visually inspected; every linked source guide exists in the candidate checkout. Rust builder tests passed. Full repository formatting and Clippy passed; the remaining task check stages are running and must pass before merge.

GitHub Pages was enabled through the bot API with build_type workflow. The source builder has contents-read only. The separate publisher admits successful bot main builds and pins the same shared project-site workflow commit as Mantle and Codegate: fb4024ef7846729e5456591b9070db3d48c87e64. Deployment remains pending source review and required checks.

## Published result

The site is live at https://beyond10x.github.io/epistemic-knowledge-runtime/. PR https://github.com/beyond10x/epistemic-knowledge-runtime/pull/61 merged as b081e387f47f09fafc67a6524a607db5523d5383. Its tree matches the candidate checked by all required PR gates. Author is b10x-bot[bot], with GitHub's verified web-flow merge signature. The repository homepage now points to the project site.

Required correctness run37045231499 completed successfully, including task check and the actual HTML quickstart on both file and SQLite providers. Documentation validation run37045231182 and shared security run37045232730 passed before merge. Main's site build run37047454771 and publisher run37047719370 passed. Deployed HTML and CSS were fetched and compared byte-for-byte with source. Both live provenance files identify the merged commit; the publisher is fb4024ef7846729e5456591b9070db3d48c87e64 and its artifact SHA256 is 0d03f927964d251fa4f86db870691189554eb828bae5d682fa89aca6741e5be4.

The local task check first encountered the existing inode-reuse test on tmpfs. The identical test binary passed on ext4; the rerun used an ext4 temporary directory. Once the complete required GitHub task check passed and the exact tree was verified, the redundant local rerun was stopped. No test assertion or runtime behavior was weakened. Local formatting, Clippy, builder tests, direct HTML quickstart and desktop/mobile browser inspection passed; the completed full-gate claim is specifically the GitHub run above.

This delivery record follows the already-published site and does not require a product release. The page and the detailed source guides remain repository-owned.
