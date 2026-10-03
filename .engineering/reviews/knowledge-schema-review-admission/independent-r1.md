needs-revision

Owners: 1 new coordinator/kernel implementor finding; 0 delegated implementor findings.

Decision IDs are checked only against proposal reviews. Capture the UUID from a successfully recorded authority-upgrade proof, then sign a schema approval using that UUID and a different target. The new path admits it because the proposal-review identity index does not include canonical upgrade or answer decisions. Replay likewise lacks this cross-domain check.

The design explicitly requires decision IDs to be unique within the audience. Enforce that binding across decision kinds during admission, atomic publication, and replay. Add tests for upgrade→proposal reuse and the reverse direction, preserving legitimate exact retries.

The two acknowledged issues—historical-basis validation and concurrent identical retries—remain coordinator-owned work and are not counted again.

Limitations: preliminary source review only; no execution, builds, edits, or AEP changes. Scope is E review records; F application is absent.

```findings
- file: crates/ekr-kernel/src/schema_proposal_review.rs
  line: 97
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: Schema review admission and replay enforce decision identity uniqueness only within proposal-review storage, allowing a newly signed approval to reuse an authority-upgrade decision UUID despite audience-wide uniqueness; bind decision identities across review kinds atomically and cover cross-domain reuse while preserving exact retries.
```
