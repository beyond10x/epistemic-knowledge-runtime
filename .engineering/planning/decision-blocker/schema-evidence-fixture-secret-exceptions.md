---
format: aep.planning-md/3
id: decision-blocker:schema-evidence-fixture-secret-exceptions
kind: decision-blocker
status: open
title: Synthetic historical fixtures require a reviewed Gates exception
relations:
- blocks: story:schema-transaction-cites-evidence
withholds: approval
revision: 1
---
The bot pre-commit hook refused the staged schema-evidence kernel checkpoint: common checks failed with 18 findings. Its pinned Gitleaks scanner identifies generic-api-key matches on synthetic native eventlog idempotency_key and key fields. These are generated deduplication identifiers in stores written by the previous kernel, not operator credentials. The exact native bytes are needed to verify historical replay; changing their encoding merely to avoid inspection is not authorized.

An exact repository/file/line/content-digest exception proposal and the redacted scanner report are retained at <cache>/ekr-knowledge-prereq-20261003/schema-evidence/gitleaks-review. The immutable raw native fixtures and their old-writer provenance are retained separately. No policy, hook or scanner has been changed, and the refused commit did not publish.

An asynchronous operator question requests approval for those exact exceptions. Until answered, leave the source and fixtures recoverable in managed tree ekr-schema-evidence-20261003 and continue the independent SDK and presentation implementation. This blocks publication of the retained fixtures, not all implementation progress. Clear only after the operator's policy decision is recorded and the same hook admits the exact candidate; do not infer approval from elapsed time.
