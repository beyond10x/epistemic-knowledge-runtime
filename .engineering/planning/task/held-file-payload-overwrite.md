---
format: aep.planning-md/3
id: task:held-file-payload-overwrite
kind: task
status: draft
title: Investigate a held payload after an out-of-band file overwrite
relations:
- informed_by: task:held-bytes-notice-deleted-blobs
- informed_by: story:viewer-evidence-reuses-admitted-revision
- serves: vision:o2
revision: 1
---
## Observation

The evidence-cache implementor's compatibility baseline, retained at `<cache>/ekr-evidence-cache/unit/compat-baseline.log`, reproduced a pre-existing discrepancy before the viewer handler changed: warm a retained evidence read through a held File runtime, overwrite its immutable payload blob without a log or manifest publication, then request that evidence again. The held runtime returned the earlier verified bytes with status 200 where the probe expected refusal. The exact probe is retained at `<cache>/ekr-evidence-cache/unit/out-of-band-probe.rs.txt`.

This is not fixed or weakened by the viewer-cache correction. That correction keeps `Runtime::content` and the existing provider authority; its corruption coverage concerns a fresh or replaced store whose bytes are verified again. The discrepancy was directly reproduced against the unchanged handler, rather than inferred from code age.

## Contract and next step

The implemented `task:held-bytes-notice-deleted-blobs` points to the decision in `task:sqlite-store-replaced-in-place`: a live handle never answers bytes a fresh handle would refuse. Investigate whether the raw overwrite case violates that decision, distinguish out-of-band corruption from supported withdrawal publication, and restore the decided invariant with a deterministic regression if required. Do not silently narrow the prior decision or claim this observation is resolved by revision index reuse.

## Scope

Provider payload revalidation and its declared immutable-byte trust boundary; generic synthetic fixtures only. This is separate from the viewer handler performance patch and does not block that patch's unchanged provider semantics.
