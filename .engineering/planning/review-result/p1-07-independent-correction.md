---
format: aep.planning-md/2
id: review-result:p1-07-independent-correction
kind: review-result
status: active
title: Independent verification of the decoder correction
relations:
- reviews: story:refuse-discarded-ontology-semantics
revision: 1
---
# Independent verification of the membrane checkpoint correction

Disposition of `review-result:p1-07-independent-checkpoint`: **addressed in the inspected correction**, pending integration and the coordinator's full gate.

The uncommitted decoder change on `ed9aff4` adds `deny_unknown_fields` to `OperationDefinition` and the surrounding ontology input records/envelopes. Unsupported `sets` now fails decoding as `OntologyError::Syntax` naming the field, before information can disappear ahead of kernel validation. No template language or assignment semantics were invented.

I read the complete implementation diff and the added tests. `a_serialized_lifecycle_invokes_but_unsupported_sets_refuse_at_decode` first loads the supported YAML and successfully validates its declared lifecycle invocation, then adds only a nonempty `sets` member and requires the named decode refusal. The ontology cases cover the surrounding input-record class and compound value envelopes. Existing test changes are additions only; no existing assertion was removed, weakened, ignored or disabled.

Evidence distinction: this verification ran no compiler or tests. I directly inspected the implementor's retained `red-kernel.log`, `red-ontology.log` and `green.log`: the new serialized-input cases fail before the annotations and pass afterward. The green log records the supported invocation control passing within the regression and no failed or ignored package cases. Runtime execution belongs to the implementor; my verification is source inspection plus review of those recorded executions. The concurrently running unit adversary is separate and is not assessed here.

Owners: the original finding was a pre-existing implementation defect; the decoder implementor supplied the correction. No new findings from this bounded verification.

```findings
[]
```

This closes the reported lost-`sets` admission path. It does not claim completion of seed validation, durable application, migration, remote CI or the broader project.

