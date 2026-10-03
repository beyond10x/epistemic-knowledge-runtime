# Authored schema evidence conformance

The exact authored selection is ekr.kernel/authored/schema-change-exposes-supporting-evidence. Its literal expectations name the transaction, schema version and retained/inline evidence IDs independently of the returned state. The adapter supplies an SDK-built document, executes ordinary kernel proposal/validation/commit, and reads the production graph renderer. Every command and query opens the native provider again with full replay. Earlier revisions must expose no later citations, and retained-evidence queries verify the actual payload hashes.

Specification digest: 7edcefdeed5694edaa4ba44768d7a681da14fe2bb22dac6ba7c2b980c4d7f5eb. Each report has the identical complete parent lineage retained in suite-input.json. The runner writes its actual observation time; these files are unmodified runner output.

- file persisted: {"error":0,"failed":0,"passed":1,"skipped":0,"total":1,"unsupported":0}

- file no-op: {"error":0,"failed":1,"passed":0,"skipped":0,"total":1,"unsupported":0}

- file mutant: {"error":0,"failed":1,"passed":0,"skipped":0,"total":1,"unsupported":0}

- sqlite persisted: {"error":0,"failed":0,"passed":1,"skipped":0,"total":1,"unsupported":0}

- sqlite no-op: {"error":0,"failed":1,"passed":0,"skipped":0,"total":1,"unsupported":0}

- sqlite mutant: {"error":0,"failed":1,"passed":0,"skipped":0,"total":1,"unsupported":0}

The no-op target accepts commands but writes nothing and exposes no state. The second control changes production document.rs only: schema.supporting_evidence becomes an empty vector. Both provider reports then fail the named scenario at GraphProjected.supporting_evidence. Source was restored byte-for-byte before the complete knowledge-target rerun. Neither a failing compiler invocation nor an unsupported scenario counts as this mutation result.

The adjacent logs retain the actual test summaries for the restored knowledge target and the complete views suite, touched-target clippy, specification/suite freshness and fresh generated-contract comparison with synthesis compilation. These are coordinator executions. Independent review of the source and these logs is recorded separately in AEP; it does not claim an independent run.

The full component inventory and integrated task check remain separate obligations. This focused acceptance does not complete PR64 or the remaining A–F work.
