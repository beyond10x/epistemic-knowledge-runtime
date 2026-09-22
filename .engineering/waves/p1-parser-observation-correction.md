# Parser observation correction

Continue the frozen parser correction in its existing managed unit. Read the
original implementation brief and the preserved review mechanism addendum.
The adopted scope is on task:bounded-transaction-document-parser; all original
limits, grammar and independent failing cases remain binding.

## Exact dependency boundary

Root authorizes an exact vendor copy of the currently pinned serde_yaml_ng
package at vendor/serde_yaml_ng, including its original source, tests, license,
README, normalized Cargo.toml and original manifest. Retain package checksum
and VCS provenance in a new UPSTREAM.md, with an explicit list of local changes.
Do not copy registry cache markers or upstream CI configuration. Root reviews
that provenance before publication. Keep upstream tests unchanged.

Add a safe opt-in inspection facade over the existing buffered loader events.
Reuse the existing scalar resolver and local-enum payload context. Expose only
the data needed for bounded traversal: decoded tags, scalar classification/text,
container structure, document/error status and safe resolved alias positions.
No raw pointers or unsafe EKR API, eager alias expansion, second lexer or second
YAML scalar resolver. Default deserializer/Value behavior remains unchanged.
Keep the library patch small and attributable; report a concrete obstacle before
expanding its API beyond this boundary.

The root Cargo.toml patch selector and Cargo.lock are within this unit's scope.
Exclude the vendor from ordinary workspace membership; retain its standalone
locked compatibility tests. Root owns Taskfile integration, DESIGN/ESS, planning,
commits and publication. The kernel must exercise the opt-in facade, so merely
adding an unused dependency API does not satisfy the original failing test.

## Counting and refusal order

Charge each expanded tag and typed string occurrence exactly once, preserving
local tag spelling and complete decoded global URI text. A tag ignored by typed
String decoding is metadata to count, not an invented enum container. Keep the
historical depth/node definitions and scalar classification. Keys retain actual
String coercion and typed-key uniqueness. Check duplicate keys before visiting
their second values and full containers before expanding the excess child.

Keep the shared credit correction where still useful. Do not count raw numeric
lexemes as strings unless the resolver or actual typed decoder produces String.
Aliases revisit at the current depth with one shared budget. Unknown, recursive
and excessive aliases refuse before application expansion. The raw byte cap
still precedes eager loader entry; no exact pre-loader allocation quota is claimed.

## Verification and handback

Keep the original independent source and YAML unchanged. Retain their red runs,
plus the tagged-key and exact-boundary controls from the partial correction.
Run a baseline of the copied upstream tests before changing dependency behavior,
then the same tests and new focused observation cases afterward. Include tag
directives, built-in/global/local scalar and collection tags, alias revisits,
typed numeric/Boolean/String parity, decoded duplicate keys, exact/one-over totals,
and existing allocation-order witnesses. Test metadata ignored by normal decoding
without changing normal decoding to expose it.

Run package tests, strict Clippy, rustdoc and formatting with the existing assigned
target and bounded build settings. No other worker may build into that target.
Report exact changed paths and dependency delta, command statuses and measured
counts. Leave source uncommitted and release your lease for independent review.
This remains correction R1; it has not passed a second independent attack and
has never been reported fixed. Do not claim writer or persistent-state coverage.
