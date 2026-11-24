---
format: aep.planning-md/3
id: story:ekr-extraction-validation-fixes
kind: story
status: active
title: Extraction and validation fixes
summary: Extraction and strict validation follow the schema in the SDK and the CLI
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T04:08:30Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-24T04:08:31Z", actor: "human:timo", revision: 3}
---
## Outcome

Extraction and strict validation accept what the schema allows and refuse what it forbids, the same way in the SDK and the CLI.

## Work

Review of EKR 0.0.42, findings 5 6 7 8 9 10 11 13 14. Each finding below is quoted from the review packet
(the review packet); the cited links point at the 0.0.42 source.

5. An inherited property can be unwritable through extraction. A leaf type inherited a nearby string declaration and a more distant integer declaration with the same name. The extraction reader and the writer selected different ancestors. Neither a string nor an integer could be extracted. An ordinary assertion using the nearby property’s ID committed. Use the same effective-property lookup in preflight and application.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/sdk.md#L195
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-integrate/src/extraction.rs#L1106
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-integrate/src/extraction.rs#L1121
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/document/ontology.rs#L546


6. Placement of an alias changes strict validation after a node changes type. An old Person resolved to a surviving Description. Putting both aliases on the fact allowed a Description-only property to commit. Putting one alias on the entity and its connected alias on the fact made it look undeclared. In a second fixture, the indirect form committed an unrelated node before rejecting a value of the wrong type. The direct form refused before writing. Resolve the full alias group before checking the survivor’s schema.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L432
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L451
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-integrate/src/extraction.rs#L797
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L680
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L707
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L805


7. The SDK and CLI leave different state for the same invalid extraction. A normal SDK caller parsed the YAML with ExtractionDocument::from_yaml and used ProcessSession. For a wrong property type or missing evidence, strict SDK application committed a node before rejecting the fact. The CLI refused both strict inputs with zero writes. Both routes committed the good control. Put the shared store-aware check before either route creates nodes.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L488
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/tests/extraction_cli.rs#L165
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/document/extraction.rs#L238
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L419
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr/src/cli/extraction.rs#L74


8. An unused duplicate type name can stop unrelated ingestion. A legal seed contained two types named Person and one named Organization. A document referring only to Organization failed at revision 0. Removing the unused duplicate let the same input commit. Resolve the names used in each document and report ambiguity at the reference that needs a choice.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2219
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/document/ontology.rs#L405
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/document/ontology.rs#L428
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L1052


9. A supplied required value cannot initialize a node through extraction. Extraction committed a type with a required label. Its row supplied label=provided, but node creation failed for a missing required property, and the fact had no subject. Creating the node with that same value through CreateNode allowed the original extraction to commit. Pass initial required values into node creation or reject the unrepresentable document before committing its schema.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2234
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2817
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L1659
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/resolve.rs#L124
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/validate/cardinality.rs#L76


10. Widening an enum can stop an unchanged older extractor. One producer declared green. A later schema added blue. The original producer’s next green row failed as value-type-narrowed, even though no row used blue. Omitting its redundant declaration allowed the row to commit. Provide an additive compatibility check for repeated extractor declarations, with explicit schema narrowing handled separately.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L429
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/document/ontology.rs#L707
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-ontology/src/evolve.rs#L585


11. The validator accepts an already-superseded replacement despite the CLI reference. Of three observations at 1000, 2000 and 3000 ms, the third first superseded the second. A later transaction successfully named that already-superseded second observation as the replacement for the first. The reference requires an active replacement. Align the validator and the temporal contract, and test already-closed and same-transaction replacements separately.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2491
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-kernel/src/validate/lifecycle.rs#L124

Cortex ingestion, comparison and store operations


13. A document accepted by the emitted JSON Schema fails at the write endpoint. The released example was converted to the JSON object form advertised by ekr schema ekr.extraction-document/1. A Draft 2020-12 validator found zero errors. Feeding those bytes to apply-extraction --strict exited 2 at facts[0], expecting a YAML tag. The equivalent tagged YAML reached revision 3. ESS declares a kind discriminator, while Cortex translates tagged JSON objects to YAML before it can write. Publish one directly ingestible interchange shape or ship the conversion with EKR and test the generated example through it.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-integrate/src/extraction.rs#L622
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/systems/ekr/domains/integrate.yaml#L245
https://github.com/beyond10x/cortex/blob/91142ab784594b37807fbad068ab52fc2eb8c721/AGENTS.md#L195
https://github.com/beyond10x/cortex/blob/91142ab784594b37807fbad068ab52fc2eb8c721/src/extract.rs#L909


14. One extra evidence byte can leave a new node with zero facts. A 16,384-byte payload committed its fact. With 16,385 bytes, the same strict command exited 0, committed the node at revision 1 and returned a rejected fact with a sequence_elements error. Cortex clips documents and skips structured records whose mapped values exceed this bound. Check the size before committing entities and provide a bounded way to cite a larger retained document across extractions.
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/docs/cli.md#L2510
https://github.com/beyond10x/epistemic-knowledge-runtime/blob/f40d8d958be4192f6de9af574ca538283df69ff5/crates/ekr-sdk/src/extraction.rs#L48
https://github.com/beyond10x/cortex/blob/91142ab784594b37807fbad068ab52fc2eb8c721/src/evidence.rs#L38
https://github.com/beyond10x/cortex/blob/91142ab784594b37807fbad068ab52fc2eb8c721/src/run.rs#L2411


## Acceptance

For each finding, a test that is red on main and green after the change, run on both the File and
SQLite stores where the finding concerns a store; no existing test weakened.
