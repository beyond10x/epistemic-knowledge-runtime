unit: story:apply-approved-refinement — historical receipt report regression
verdict: blocked
cases: not executed; Cargo lane belongs to coordinator
origin: n/a
wrote-outside-worktree: assigned private application-progress evidence directory
needs-coordinator: yes, tests.patch and module-registration.patch; compile and red/green execution

The only source addition is crates/ekr-kernel/src/application_progress/tests.rs. Shared application_progress.rs was read but not edited. The module registration is a separate coordinator patch.

The test older_receipt_reports_only_its_mapping_prefix_after_later_commits creates a real seeded and upgraded runtime on each provider, imports an immutable interpretation backed by actual retained HumanStatement bytes, approves two mappings into distinct optional properties, and executes the real application. It closes the runtime, captures actual retained history through a fresh trusted kernel authority, and cold-reconstructs replay state. No ReplayState, canonical publication or transaction outcome is fabricated.

From the two actual mapping commit revisions, it reconstructs the historical application receipt at the first mapping revision. This older snapshot uses the retained processing receipt identity; the application receipt identity itself is newly minted by the real snapshot projection. The test does not pretend that this older application receipt had been persisted before the second commit.

It requires the older report to preserve FactsInProgress (generated V2), one remaining qualified mapping item, one Integrated item and one Parked item (ProcessingDisposition V2) without a transaction/assertion. The retained complete receipt must instead produce both Integrated items and no remaining work. Clearing the older receipt's remaining items must refuse, testing the report's receipt validation separately. The two mappings share facts[0] and source bytes but have different mapping digests, exercising qualified item matching.

The old implementation that read items through the latest head should fail at the Parked assertion; this is a source-based prediction, not an observed red run. The corrected implementation has not been compiled or executed by this worker. Exact-file rustfmt --edition 2021 --check passed (exit 0). No Cargo, source implementation, ESS, AEP, commit or publication action occurred. The shared test fixture retains the existing fixture-only unused-helper/import allowance; no product warning suppression is introduced.

Files in the private assigned evidence directory: tests.patch, module-registration.patch, report.md. The source file SHA256 is f6ae63a24b2f2d9c673257e355d9794e90e0bfac811e752dbea3c9da3c483b95; tests.patch SHA256 is 87e274773a42bb64dbaff10bae91c6e31b71d16bc2df4a59a6ffcc17526f3332.
