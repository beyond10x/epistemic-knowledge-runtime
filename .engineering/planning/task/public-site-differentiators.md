---
format: aep.planning-md/3
id: task:public-site-differentiators
kind: task
status: active
title: Explain shipped EKR capabilities and the next epistemic loops
relations:
- informed_by: story:simplified-public-site
- serves: vision:o2
- serves: vision:o5
- serves: vision:o6
- decomposes: story:simplified-public-site
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T19:09:11Z", actor: "agent:codex-ekr-features", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-02T19:09:11Z", actor: "agent:codex-ekr-features", revision: 3}
---
## Outcome

Make the public landing page explain what EKR adds beyond graph storage: governed acceptance, retained support, temporal correction and inspectable quality. Distinguish shipped behavior from planned epistemic loops without claiming exclusive inventions or automatic truth verification. Requested by the operator on 2026-10-02.

## Scope

website/index.html and minimal website/styles.css presentation. Preserve the tested quickstart, Rust builder and publisher. No product behavior or specification changes.

## Acceptance

Available-now copy names evidence attachment to held assertions (0.0.27), repeat-safe extraction, typed identity resolution with ambiguity, two time axes, explicit schema evolution, and reproducible sampled fact assessment with caller-supplied judges. Planned copy identifies incubation/dispute integration, evidence-led schema discovery, budgeted observation/frontier scheduling, and maintenance. Readers can distinguish these from existing manual operations. Explain that a graph records relationships while EKR also governs acceptance, support and revision; do not imply other graph systems cannot do these things.

Sources of truth: README.md status, CHANGELOG.md 0.0.27/0.0.26, docs/cli.md, docs/sdk.md, docs/overview.md and docs/roadmap.md P2–P6. Current README/CLI take precedence over outdated overview diagrams. A claim being accepted is not proof of truth; properties are not evidence-bearing assertions; extraction is not one atomic transaction; connectors and model judges belong to consumers.

## Verification and delivery

Run existing Rust site builder tests/check/build and inspect desktop/mobile rendering and anchors. Full required Repository correctness, common security and documentation checks must pass before bot merge. Verify live HTML/CSS and provenance against the merged source. Retain delivery evidence in this store. No version release is requested.
