unit: released ESS0.29.0 preflight for the unchanged EKR writer activation draft
verdict: PASS — compiler adoption preflight only
cases: 33 generated, 0 refused, 0 outside, 0 authored; 0 runtime scenarios executed
origin: n/a
wrote-outside-worktree: only the assigned released-029-preflight output directory; full path/inventory retained privately
needs-coordinator: adoption, pinning, declaration activation and runtime implementation remain coordinator-owned

The verified released compiler accepts the unchanged ess/7 proposal and generates both retained-result scenarios plus complete wrong-state observations. Its IR and kernel suite are **byte-identical to the final development-r1 artifacts**. No new substantive mismatch was found. The activation patch applies cleanly to the coordinator checkout and reverse-checks cleanly against the unchanged proposal.

Owners: this independent preflight owns only the compiler invocations, artifact inspection and this evidence report. The coordinator owns the draft, adopted durable-record decisions, planning state, release trust, compiler pinning and activation. No checkout, proposal, planning record or store was modified by this preflight. The coordinator's separate completion-page edit appeared during inspection; all measured task input hashes remained unchanged.

## Verified compiler and inputs

The executable reports `ess 0.29.0`; its SHA-256 matches the supplied verified release:

```text
88a1e82774dd9e5a6913c7700416e3621bb82b57ace624531e8d21248b409c54
```

The coordinator supplied release tag commit `8bef63a21766c54f0d809b4decf1d9f7bd587118`. This preflight independently checked executable version/hash; it did not repeat release publication/provenance verification or rebuild the compiler.

Coordinator HEAD at the check was `e4cedbf97563f09de36b190e5de1bbdbd459de78`. The exact activation patch SHA-256 is `388d3a18e570fedbcbcffe3f470b23d7210839b007f330b8706193a240dcbf52`. The proposal contains the dated design amendment, ess/7 system, four domains and existing components. Its seven files passed before/after hash verification. The acceptance report, adopted decisions, active planning story revision38 and patch also passed unchanged-input checks.

## Commands and statuses

The following commands ran with the exact verified release executable. `PROPOSAL`, `OUT` and the full executable path are expanded in `commands-private.txt`; the public names below are placeholders only for those recorded paths.

```sh
ess --version
ess specify validate --path "$PROPOSAL/systems/ekr"
ess specify compile --path "$PROPOSAL/systems/ekr" --format json --out "$OUT/ir.json"
ess verify conform synthesize --path "$PROPOSAL/systems/ekr" --component ekr-kernel --suite-format 5 --target ir --format json --out "$OUT/kernel-suite.json"
git apply --check .engineering/waves/p1-writer-activation-ess7-draft.patch
```

Each exits **0**. Validation prints `ekr v1 — 6 file(s), valid`. Compile and successful synthesis stderr are empty. The CLI's latest selectable coverage family is `--suite-format 5`; admitted features select the actual resulting **ess-conformance/13** envelope. The proposal reverse patch check also exits0.

One preliminary invocation incorrectly used the domain spelling `--component ekr.kernel`; the compiler correctly refused the invalid component name, exit1. Its stderr/status remain as `synthesis.stderr`/`synthesis.status`. The real declared component is `ekr-kernel`; its successful invocation uses the separate `synthesis-kernel.*` files. This operator mistake is not a source defect or a synthesis refusal in the final inventory.

Eleven static generated-artifact checks are true (`artifact-checks.json`, jq exit0 and all-results check exit0). They inspect the retained artifacts; they are not EKR runtime cases. **No `verify conform run`, target callback, provider access, Cargo build or live-store operation ran.**

## Exact artifact comparison

| Artifact | Requested earlier development-corrected | Released0.29.0 | Final development-r1 comparison |
|---|---:|---:|---|
| IR bytes | 196574 | 196610 | byte-identical, cmp0 |
| Kernel suite bytes | 205705 | 268832 | byte-identical, cmp0 |
| Generated/refused/outside/authored | 33/0/0/0 | 33/0/0/0 | identical |
| Scenario identities | 33 | same33 | identical |
| Complete/legacy snapshots | 0/9 | 13/0 | identical |
| Retained-result checks | 2 | 2 | identical |
| Explicit zero-direct-event checks | 4 | 8 | identical |

Both comparisons with the requested earlier `development-corrected-*` artifacts return cmp1 because those artifacts predate the final observer correction. The **only IR difference** is compiler-minted `complete_refusal: true` on `ekr.kernel.Validate/wrong-state`. The suite changes reflect that source7 observation obligation, explicit complete typed snapshot pairs and the immediate replay eligibility observation. The retained final development-r1 files already contain exactly these changes; the released output adds no further difference.

Released byte SHA-256 values:

```text
ir.json           451437d212e0634fffbbc4c4f32469344306d4e8afd5ca905d1ac8f2f9b2f2df
kernel-suite.json 438acf7442184a36fe60700432dcb5e1c96d62dd3cc85607ce1b367ea345bd4a
```

Suite provenance digests are distinct from those file hashes:

```text
spec_digest     154bec6b6428118463d5c2c3f9ebd6556c43c39be37f609bd448d9358020fce0
contract_digest 415bd9382718706c9113b53f6f096ac9eac47fa534761bded66eda5267a21da8
```

The earlier requested artifacts have spec digest `b50b0ef928524cf2fa06fd00f03bd0db51ba765f81dc1c002a578fa2fbca283e` and contract digest `496bfb124fcc102153138b14848d04c36650e8ce786956f00f737b336619ee40`. The changes are expected from the recorded compiler correction; the proposal itself was not edited. `artifact-comparison.sha256`, `development-corrected-ir.diff`, comparison statuses and both summaries retain the exact comparison evidence.

## Generated obligations inspected

**Commit retained result:** the scenario proposes, validates, then commits the actual bound transaction. It captures the originating `committed` response as full `CommitReceiptV1`, binding identity to the original `transaction_id` input. It observes actual `Committed` state before retry. The retry sends the identical bound transaction and Committer actor, then compares its result against the captured origin. The complete Transactions snapshot descriptor has14 fields, including typed lifecycle state and full validation basis; its original and subsequent queries select the same transaction. No external-result configuration appears in this replay scenario.

**Seed retained result:** the originating Seed captures the identity from the actual `Seeded.revision_id` event and retains the full typed `SeedResultV1`. The second Seed sends the same `seed_document` and Operator actor; its response is checked against the captured result. A10-field complete Revisions descriptor includes revision identity/number/parent, all four roots, transaction linkage, committed time and lifecycle state. The additional3-field CurrentRevision observation is separate; it does not replace the complete Revisions coverage. No external-result configuration appears in this replay scenario.

**Wrong-state refusals:** all seven state-specific scenarios remain: Commit from Proposed/Rejected/Stale, and Validate from Validated/Committed/Rejected/Stale. Each requires `TransactionStateConflict`, zero direct events, a complete Transactions snapshot and an unchanged subsequent observation. The Proposed Commit case additionally observes PendingTransactions. Stale arrangement still reaches the original external Commit/stale branch; it is not a forced retained-result branch. The future real adapter must produce that state through the actual competing commit described by the adopted acceptance.

The generated replay witnesses are immediate retries. They do **not** establish restart persistence, retry after later head advancement, whitespace-equivalent seed matching, changed trusted-context refusal, actual atomic payload publication or durable root reconstruction. Those remain the existing runtime/authored/provider acceptance, unchanged by this preflight. The older acceptance document's references to an ESS0.28 declaration gap describe historical preparation; the verified0.29 compiler now closes that compiler-capability gap without establishing runtime completion.

## Preservation and handback

All writes were under the new assigned output directory. The proposal is unchanged, task input hashes are unchanged, and both patch directions were checked without applying either. No source dispatch or activation occurred. `private-inventory.md` and `written-paths.txt` enumerate the exact retained outputs; command stdout/stderr/status files retain the original evidence. There are no owned running processes or worktree leases from this read-only preflight.
