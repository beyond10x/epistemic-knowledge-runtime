# Integrated conformance corrections

These are local execution records after integrating schema evidence at `fe9efea8a`.
They do not establish a passing full gate or completion of stories E/F.

The three retained full-gate attempts expose successive failures: mutation controls counted
Unsupported as a detected defect; the inode staging probe could not recycle tmpfs inodes; and the
kernel inventory contained ten unsupported knowledge command scenarios plus a stale suite digest.
No inventory, coverage floor, quarantine or scenario selection was relaxed.

The mutation controls now compare complete untampered status maps and require actual Failed
verdicts for negative controls. Both inode tests use the same optional `EKR_INODE_TEST_TMPDIR`;
the probe still requires actual inode reuse. The complete kernel adapter routes knowledge commands
to native providers with independently signed fixture inputs, preserving supplied request values.
Refusal setup changes real retained state or independently pinned policy; runtime results determine
the reported outcome. Existing authored scenarios still exercise persistence and replay.

The kernel suite is `ess-conformance/19` because the contract now declares typed fixture inputs.
The exact frozen suite is `kernel-suite.json`. All three dated executions on each provider report
72 passed, zero failed/error/unsupported/skipped. Final detailed runs are retained beside them.
The report clock dates evidence; fixture clocks and behavioral assertions remain deterministic.

Independent review found an additional observation gap: a refused upgrade could append a publication
without the adapter reporting it. The adapter now checks publication boundaries on refusals and
read-only commands. The negative control inserts a real proposal before returning the actual
upgrade refusal. Removing only that guard makes the scenario incorrectly pass and the control fail
(`refusal-publication-guard-red.log`); restoring it detects the publication on both providers.
The final bounded review approved this correction without claiming an independent test execution.

`inventory-final-focused.log` records 35 passing Rust tests across six targets, including both
native kernel suites, all authored knowledge selections, mutation controls and inode cases.
`inventory-final-clippy.log` records warnings-denied clippy over the CLI's targets.
`inventory-final-fresh.log` records fresh contract generation/comparison/compilation, specification
validation and exact regeneration of all seven committed suites. Generator limitations printed
there remain visible; this is not a claim that every generated capability is implemented.

The integration suite still reports 2 passed and 16 unsupported on each provider
(`inventory-integrate-pending.log`), covering missing adapter routes and unfinished schema proposal
implementation. The full gate remains red; D and the sole draft PR64 remain open. The next delivery
work is E/F with their generated contracts, native scenarios and end-to-end demonstrations, followed
by the complete gate. No external source publication or ESS release action is claimed here.

Only host path prefixes were sanitized in text logs. Structured reports and suite bytes are unchanged.
