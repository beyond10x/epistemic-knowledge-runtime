---
format: aep.planning-md/3
id: decision-blocker:schema-evidence-fixture-secret-exceptions
kind: decision-blocker
status: cleared
title: Synthetic historical fixtures require a reviewed Gates exception
relations:
- blocks: story:schema-transaction-cites-evidence
withholds: approval
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-03T15:32:58Z", actor: "agent:codex-ekr-knowledge", revision: 3, decided_on: {"recorded":{"approval":1}}}
---
The bot pre-commit hook refused the schema-evidence kernel checkpoint because its pinned scanner matched synthetic native eventlog idempotency_key and key fields as generic-api-key. These generated deduplication identifiers are not credentials. Preserving their native bytes is necessary for historical replay verification.

The operator explicitly answered "Approve exact fixture exceptions" to the question carrying the proposed repository/file/line/content-hash exceptions. The exact proposal and redacted scanner report remain at <cache>/ekr-knowledge-prereq-20261003/schema-evidence/gitleaks-review. The policy CLI applied that proposal in managed tree ekr-fixture-exceptions-20261003. A structural comparison verified the complete added set against the approved proposal and verified that no other policy fields or prior exceptions changed. Neither wildcard-line nor wildcard-content exceptions were authorized or added.

Private gates-policy commit a74216535fa81038e25e9227a99bdea81e245582 was authored, committed and published by the bot; the local consumer policy was fast-forwarded to that published commit. The original EKR bot pre-commit hook then admitted the preserved candidate as kernel commit dded81ab56. The SDK regression test remains outside that checkpoint. The decision and hook admission clear this fixture blocker; SDK, presentation, independent review, conformance and the complete delivery gate remain unfinished.
