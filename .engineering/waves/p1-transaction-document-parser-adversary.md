# Independent bounded-parser review

Review task:bounded-transaction-document-parser after the implementor's source
handback. Read the implementation brief, adopted frozen profile, readiness report
and p1-transaction-parser-contract-corrections.md first. The latter preserves the
canonical ID grammar and corrects an impossible preparation premise; it does not
waive typed duplicate protection.

Use the same managed ekr-transaction-parser-20260922 tree only after its worker
has frozen the source and released its own lease. Acquire the distinct lease
codex-ekr-transaction-parser-adversary. Reuse that tree's target sequentially;
scratch is <cache>/ekr-completion-20260922/transaction-parser/adversary. Root owns
all planning, contracts, integration and publication. Read repository AGENTS.md.

## Review boundary

Find concrete admission, preservation, allocation-order or compatibility defects.
Add independent cases only in
crates/ekr-kernel/tests/adversary_transaction_document.rs. Do not edit implementation
or the original tests. Do not touch a real provider or operator store. Package
checks and explicit behavioral probes are appropriate; this is not a writer,
restart or conformance completion review.

Investigate original-byte typed semantics versus the bounded representation pass:
coerced String scalars and map keys, escaped-equivalent keys, enum tags and nested
payload depth, optional null versus required map/sequence syntax, Record keys
named like envelope fields, NaN/infinity proposals and ordinary quoted strings.
Check unsupported operations' nested carriers as rigorously as admitted ones.

Each alias occurrence must charge its actual expanded position and strings. Every
inclusive limit needs a positive control and a one-over refusal that another
limit does not mask. Check the allocation-order sentinels and error-priority
probes rather than inferring guarantees from a post-allocation count. The raw cap
bounds loader input; there is no claim of exact pre-loader allocation quotas.
Duplicates must refuse before their second values are decoded. Canonical UUID
syntax remains frozen; alternate UUID spellings are negative controls, not
supported aliases. Explicit empty inner List/Record and repeated sequence values
must remain data.

Follow the actual frozen legacy decoding paths when evaluating the shared map
helper; unchanged source text alone is not proof of unchanged semantics. Keep
admission and resource refusal distinct from a persisted Rejected state. A local
parser object must not claim it has proposed, validated or committed anything.

## Handback

Retain exact red inputs and original tests. State each finding's source, case,
observed result, intended result and owner attribution. Root will record findings
through AEP and route corrections. If nothing is found, describe the exercised
boundaries and limits without claiming universal correctness. Include command
exit statuses, actual changed test paths and case counts. Release only your own
lease; leave added cases uncommitted for root. Do not start a concurrent compiler
or another agent in this tree.
