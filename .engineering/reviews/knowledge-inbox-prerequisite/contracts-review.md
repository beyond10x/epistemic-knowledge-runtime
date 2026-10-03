# Knowledge contracts: independent targeted review

Reviewer: scope_retention sub-agent, 2026-10-03. Final verdict: APPROVED for source contract inspection only. No implementation, conformance or digest verification was claimed by the reviewer. The coordinator independently recorded author validation and compiled digests.

Three concrete findings were raised and corrected before contract commit 07604bf1a5e6c19ef86b78846dc99a603f3d2bfb:

1. Mapping records lacked retained proposal/source bytes. integrate.yaml:1002 now references proposal, source-document and mapping StoredObjects; design amendment §105 requires verified Provenance-or-stronger retention during admission/replay independently of live incubation roots.
2. Settled projections did not exclude disputes. views.yaml:141 now specifies exclusion of disputed claims and their derived property/edge values, preserving complete inspection and historical rendering.
3. Public show APIs listed history IDs without a detail read path. integrate.yaml:1197 and :1280 now return typed blocker/processing and review/application snapshots; result payloads at :1449/:1469 agree.

Approval clarification: integrate.yaml:1393 and :1399 require the selected review to belong to the exact proposal/digest, be Approved and remain valid before every canonical write. Latest means durable append order, not timestamp. Later rejection stops new/resumed writes and preserves committed progress.

Replay clarification: kernel.yaml:1313 and :1336 retain the exact target profile bytes by digest; design §105 requires digest agreement and reconstruction independent of current host configuration. Historical authority boundaries and pending transaction revalidation are preserved.

Coordinator additionally identified and author corrected self-referential interpretation document hashing by separating input coordinates from computed digest envelopes; missing mapping digests on processing receipts; observation list/show access after rejected interpretation; and proposal observation support independent of canonical evidence admission.

The earlier supplied plan's four local critic perspectives were not an independent panel. Their individual reports were not provided and are not recreated here.
