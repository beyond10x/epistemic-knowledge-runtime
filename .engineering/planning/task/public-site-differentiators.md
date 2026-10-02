---
format: aep.planning-md/3
id: task:public-site-differentiators
kind: task
status: implemented
title: Explain shipped EKR capabilities and the next epistemic loops
relations:
- informed_by: story:simplified-public-site
- serves: vision:o2
- serves: vision:o5
- serves: vision:o6
- decomposes: story:simplified-public-site
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T19:09:11Z", actor: "agent:codex-ekr-features", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-02T19:09:11Z", actor: "agent:codex-ekr-features", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-02T22:00:11Z", actor: "agent:codex-ekr-features", revision: 5, decided_on: {"recorded":{"test_result":2,"deployment_result":1}}}
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

## Published result

The page now leads with governed acceptance, retained support and revisable history, with a concrete temporal correction example. Six shipped capabilities link to their reference pages; four planned loops are explicitly labelled. The tested quickstart remains unchanged. Desktop/mobile browser inspection and the three existing Rust builder tests passed. No product behavior or release version changed.

PR https://github.com/beyond10x/epistemic-knowledge-runtime/pull/62 merged as 502a160630521b7203b3e167b395dcfbf8ba2634. Another reviewed change reached main during the initial wait, so the branch was merged forward and all required checks reran. The final candidate 1d0db584a65595f6fe0e52ae04d304c1f6dc7b10 passed full Repository correctness (37068141302), common security (37068137550) and documentation build (37068141207). The bot-authorized GitHub merge has the exact candidate tree dd9b1cfbcd8a993e8df9745e6e0ef91981446656, bot author and verified web-flow committer.

Main documentation build 37069803602 and publisher 37069941578 succeeded. Live HTML and CSS at https://beyond10x.github.io/epistemic-knowledge-runtime/ were compared byte-for-byte with source. Both live provenance documents identify merge 502a160630521b7203b3e167b395dcfbf8ba2634; the publisher runtime is fb4024ef7846729e5456591b9070db3d48c87e64 and the artifact SHA256 is 1172e69d371b1abf046221ee7304fb0316bec631568f9eb43fac7c1bbe7daf88. This record follows the already-verified publication and requires no additional product release.
