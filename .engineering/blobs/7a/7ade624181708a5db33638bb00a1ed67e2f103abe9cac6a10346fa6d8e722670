---
format: aep.planning-md/1
id: task:gates-historical-merge-delivery
kind: task
status: implemented
title: Repair verified historical merge delivery before publishing P1 work
relations:
- derived_from: story:kernel-validated-seed
- serves: vision:o2
revision: 5
---
## Reproduced publication blocker

The integrated seed gate passes, but b10x-gates publish rejects the already
published App-created merge 250aecaa0d3598a6354d22cd7a0f86b491468424 because
GitHub is its committer. A fresh common check signed the candidate successfully.
All newly introduced commits have exact bot author and committer.

The workspace rule permits an authenticated App-created GitHub merge after exact
authority, merge-action and candidate-tree verification. Gates currently applies
the strict direct-commit check to every ancestor since adoption. The upstream
story:published-merge-delivery now owns a bounded shared publish/pre-push verifier.

Keep the adoption baseline, protections, hooks and scanners intact. Clear this
task only after the reviewed/released binary and coordinated hooks verify the
actual published merge, then EKR publication and required checks succeed.
Pure frozen-format work can continue without claiming main integration.

## Published and adopted

The reviewed implementation is on main at 9466d1fe3aca3eeeace91547496fef5a46c21503,
through https://github.com/beyond10x/gates/pull/16. Required correctness and shared
checks pass for the candidate, main and annotated release tag. The published release
is https://github.com/beyond10x/gates/releases/tag/0.1.6, authored by the organization
bot. The downloaded static binary and checksum asset match the built artifacts;
the binary reports the release version and its SHA-256 is
f4f4974cdd13b896e875574f159d6c07fa7ba9a143577a9e3c36c5695fbe2d51.

Adoption updated the CLI and coordinated hooks for Gates, Eventlog and EKR while
preserving prior hook-chain and retirement digests. Other repository hooks are
unchanged. EKR's actual seed publication then passed both publication and pre-push
verification over the historical GitHub merge. Its required correctness and
shared checks passed, and https://github.com/beyond10x/epistemic-knowledge-runtime/pull/8
is merged at a4b21d54e3838efbf924cb60666073b2bf493b67. This closes the reproduced
delivery blocker without moving the adoption baseline or weakening branch rules.

Retained evidence: release-published.json, release-verified/, hook-chain before
and after files, and the consumer publication receipt in the completion run's
private scratch. Source review and mutation evidence remain linked above.
