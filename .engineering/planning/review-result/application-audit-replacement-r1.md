---
format: aep.planning-md/3
id: review-result:application-audit-replacement-r1
kind: review-result
status: active
title: Application audit preserves store replacement recovery
relations:
- reviews: story:apply-approved-refinement
revision: 1
---
approve

Owners: 0 new coordinator/store implementor findings; 0 delegated implementor findings. The coordinator's reproduced File-session recovery regression is corrected within this bounded review.

`crates/ekr-store/src/applications.rs:908-918` now preserves only the existing `eventlog::diverged` classification through `StoreError::from`. That shared predicate recognizes the File provider's specific diverged-history backend signal, and the existing conversion produces `StoreError::Diverged`. All other feed errors still become `application-audit-unavailable`. The audit remains mandatory: no error becomes success, no partial feed is returned, and tenant/order/pagination checks are unchanged.

The existing CLI session recovery path recognizes this typed refusal, reopens once, and retries through the normal reader. It retains the existing refusal to follow replacement while session-owned proposals remain open. The change restores error identity rather than adding another reader, disabling the audit or admitting stale cached state.

Inspected implementor evidence under `<retained-evidence>/schema-application-kernel/`:

- `application-audit-replacement-red.log`: the existing `a_session_follows_a_file_store_replaced_inside_its_directory` test fails in 0.05 seconds because audit error flattening reports `application-audit-unavailable` with no divergence signal.
- `application-audit-replacement-green.log`: the same test passes in 0.05 seconds. Its unchanged fixture replaces the real File history inside the same directory/inode, then requires the new revision and exactly one reopen.

The unreadable-feed controls in `crates/ekr-store/src/eventlog_reads.rs:543` and `:591` are unchanged and explicitly require `application-audit-unavailable` for a generic backend feed refusal, including repeated reads. Their broader rerun and the full CLI library rerun were reported as pending; no green result is claimed here. Source inspection confirms the corrected branch cannot match their generic "feed refused" error.

Inspected `crates/ekr-store/src/applications.rs` SHA-256: `9542a721718307cd5b1f0abbe925eb6868ce0862d90c2b6c07837de973bd4292`.

Limitations: source/log review only; the executions belong to the coordinator. This reviewer ran no builds, tests or mutations and made no source/AEP/publication changes. No separate viewer or SQLite replacement execution is claimed from the focused File-session test. Approval is limited to this error-classification correction, not whole F or the full gate.

```findings
[]
```
