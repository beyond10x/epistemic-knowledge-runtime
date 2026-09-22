# Original format vectors

`vectors.json` is immutable evidence from commit
`73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0`. Before capturing, the original
graph, ontology, core, kernel transaction/seed, and store snapshot/log source
was compared against that commit and had no differences.

The `json` string in each vector contains the exact original serializer output
bytes as UTF-8. `payload_hash` addresses those exact bytes in `ekr.payload.v1`.
Where the original type had a canonical encoder, `canonical_hex` contains its
exact output and `value_hash` addresses it in `ekr.value.v1`. A graph document,
seed input, or seed envelope had no whole-document canonical value encoder, so
no such encoding is invented here.

The seed envelope was captured by the original kernel's actual seed validation
and replay, using a synthetic runtime fixture and distinct operator/validator
identities. The input assertion remains Proposed in persisted bytes; replay
changes it to Accepted with the bootstrap validator. The accepted assertion and
resulting knowledge/evidence roots were captured from that replay. No provider
was opened during capture. These are synthetic data, with no customer content.

The corpus covers ten value variants including nested List/Record values,
scalar node and edge properties including nonempty and empty Lists, all seven combined assertion states plus
subject/object/time alternatives and the bootstrap acceptance, six evidence
sources, six revision facts, eleven transaction operations including transitive
ontology definitions, an ordered transaction vector, the original graph
document, the seven-field revision root, and seed input/envelope.

The validation vector is the original transaction canonical bytes followed by
the revision encoding, hashed in the **payload** domain. Its operation vector
encoding is order-sensitive even though validation/application treats a
transaction as an atomic set. These facts are retained without historical repair.

Verification tests only consume these constants through frozen legacy types.
They must never regenerate expectations using current production codecs.
Additional historical fixtures need separately established provenance. Address
reproduction does not establish complete history, authenticate recorded actor
identities, revalidate a schema or grant commit authority.
