# Schema evidence SDK checkpoint

The existing builder rejected inline schema evidence with MixedSchemaTransaction before the fix (sdk-red.log). TransactionBuilder now cites inline additions automatically and accepts explicitly selected retained support through with_schema_evidence. Schema changes still refuse other data operations; data transactions refuse explicitly selected schema support.

The document_drift target passes all its cases (sdk-green.log), including kernel parsing and the public JSON document schema for both support paths. This checkpoint does not claim store admission through the SDK session, read presentation, named conformance or final story acceptance. Kernel provider admission and historical replay evidence is retained separately in knowledge-schema-evidence-kernel.

This is implementation evidence collected by the coordinator, not independent review.
