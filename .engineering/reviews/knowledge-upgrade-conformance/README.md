# Native upgrade conformance verification

Specification source digest: 147770d5ae58c39107c86a474a7d5a2a01b4a84701f3864bbd0d4874663d73ad.
Contract digest: b79e26b7335246ec2103e09912236dba575c2b8e1118ee2a0238152e64c2b4e6.
Authored source checkpoint: 36739a32eee65d70be5c0e46c6ac36af0c0c4df4; the adapter corrections and these results follow it.

## What executes

`conformance_knowledge` selects the three authored upgrade scenarios from `upgrade-suite.json` and the three retention scenarios from `knowledge-suite.json`. Every real upgrade command and view query reopens the file or SQLite provider with full replay. Commands return a consistency token identifying a verified revision record; a subsequent view must find that same record in its verified lineage. Unknown tokens, absent future revisions and different record hashes are refused by an executed control.

The fixture enters through historical seed admission. Independent input facts have stable IDs; expected competitor pairs, accepted claims, settled counts and historical-root comparisons come from authored ESS. The exact signed upgrade preview is supplied before scenario execution. The test-only human key grants no authority outside the disposable fixture stores. The semantic HumanDecision1 member is converted to its declared wire label without changing signature bytes.

The adapter reads real assertion assessments, settled claims, historical roots, attention and the durable authority event. Teardown compares original retained seed/receipt/evidence bytes and the original published-event prefix. The exclusions cover properties and relations independently: equal overlapping One values, different disjoint half-open One intervals, and different overlapping Many values. The conflict scenario checks symmetric competitors and exclusion from settled results. The history scenario checks the original revision and the new authority revision.

## Observed failures and corrections

The first compile failed because PublishedEvent is exported under ekr_kernel::runtime. The import was corrected; this was a compilation defect, not a behavioral red test.

The first runtime run failed all three upgrade scenarios on File because the adapter supplied no consistency token. Command and event checks passed, but the runner correctly refused to weaken read_your_writes to Current. SQLite positive execution was not reached in that first run. The adapter now supplies a verified boundary and checks it after reopen. The runner records both provider results before failing its outer test, so a failure on File no longer suppresses SQLite evidence. Expectations and consistency requirements were preserved.

## Local report counts

Three consecutive restored-source runs produced identical counts. The Rust target passed seven tests each run, including the exact-integer and consistency controls. Each provider separately reported:

| Selection / target | Passed | Failed | Error / unsupported / skipped |
| --- | ---: | ---: | ---: |
| A retention, actual CLI | 3 | 0 | 0 / 0 / 0 |
| A retention, discarded-state target | 0 | 3 | 0 / 0 / 0 |
| B upgrade, actual Runtime | 3 | 0 | 0 / 0 / 0 |
| B upgrade, inert target | 0 | 3 | 0 / 0 / 0 |

These are selected local conformance counts, not coverage of the generated scenarios outside the selection or a CI-image result. `measurements.json` reads the report counts and binds each report to its exact input bytes. The two suite-input documents are deduplicated only after comparing their SHA-256 digests. Final-run and mutation run documents retain the individual check diagnostics.

## Production mutations, observed on both providers

Each temporary production mutation made exactly its expected named scenario fail, while the other two B scenarios passed; there were no errors, skips or unsupported results. Both native providers were executed for every mutation.

| Mutation | Named scenario that failed |
| --- | --- |
| Ignore property contradictions | overlapping-single-value-claims-become-disputed |
| Ignore relation contradictions | overlapping-single-value-claims-become-disputed |
| Treat equal values as unequal | equal-disjoint-and-many-claims-do-not-conflict |
| Ignore interval separation | equal-disjoint-and-many-claims-do-not-conflict |
| Treat Many declarations as single-valued | equal-disjoint-and-many-claims-do-not-conflict |
| Runtime reads current state for a historical request | upgrade-preserves-historical-rules-and-hashes |

The property mutation added `matches!(predicate, Predicate::Property(_))` to the skip condition in `preview_contradictions`; its reports retain the observed failure. The other exact patches are included. Both production files were restored byte for byte before the three clean runs. This is a root-local mutation check, not independent review.

## Resource handling and remaining work

Persistent-disk pressure initially prevented compilation. Read-only inspection found a separate executable tmpfs with sufficient capacity, so one job with incremental compilation and debug symbols disabled used a private temporary target. Raw evidence stayed outside that disposable directory. No other owner's process or cache was modified, and no new persistent build cache was started.

The existing conformance task runs this Rust target; freshness now also checks the upgrade suite. It is ess-conformance/19, while A remains /13. The old ESS 0.36.0 gate prerequisite still needs coordinated adoption of a verified published generator; v0.52.0 was not yet published when checked. No version claim or release pin was substituted.

Workspace Clippy and the focused kernel/CLI upgrade-surface checks also passed; their logs and measured execution counts are retained in ../knowledge-upgrade-surface/. The job's CI image, final task check, native process-crash coverage, public disputed-viewer demonstration, C–F and both end-to-end demonstrations remain outstanding. These local conformance results do not establish PR64 completion.
