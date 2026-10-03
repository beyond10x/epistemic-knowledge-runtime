# Current-schema gap discovery checkpoint

This is the discovery increment of active story `propose-better-vocabulary`, based on
`6c50246e8`. It does not complete E, F or PR64. Proposal submission, preview, authenticated
reviews, application and the final integration gate remain pending.

The generated DiscoverSchemaGaps obligation now runs through the kernel, native file/SQLite
providers, CLI `schema-proposal discover`, session and typed SDK. It groups outstanding source
items against one verified current schema. Stable group handles include the original seed
occurrence and current kind/declaration. Group anchors refer to immutable import findings;
reclassification never changes historical records. No canonical revision or queue is written.

Verification, from each runner's own output:

- First kernel test: compilation failed on the absent runtime method, then one test passed
  with eight filtered. The complete retention target then reported nine passed.
- Independent review found that import-time blockers survived schema advancement. The new
  regression committed a real Project declaration and failed with Project instead of
  Project.health. After extracting the shared pure classifier, the complete retention target
  reported ten passed, zero failed/ignored. Both new cases run file and SQLite and full replay.
- CLI/SDK test: compilation failed on the absent SDK method. The existing end-to-end test now
  additionally checks the typed request against generated JSON Schema and compares it after
  reopening. Final knowledge_cli: two passed; docs_cli: eighteen passed. Counts did not increase
  because existing tests were extended. An earlier docs_cli run exposed the unclassified
  schema-evidence-not-cited refusal from D; it is now classified as evidence, not schema shape.
- Clippy for kernel, CLI and SDK, all targets with warnings denied: exit zero.
- Repository formatter check: exit zero.
- ESS specification validation: nine files valid. Fresh generated semantic and data artifact
  trees match, and the synthesized semantic workspace compiles.

Independent source/log review approved the corrected increment with zero new findings. The first
finding and its disposition are recorded as two immutable AEP review results. This is focused
behavior evidence, not complete ESS component conformance: the integrate adapter and E/F commands
still have known unavailable obligations. No floor was reduced and no case was quarantined.
Conformance suites need regeneration alongside the remaining E/F work before the full gate.

Logs retain the red and green outcomes. Only local path prefixes were sanitized.
