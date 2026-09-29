---
format: aep.planning-md/3
id: story:credential-redaction-before-model-input
kind: story
status: draft
title: Credentials are redacted before any model-facing projection
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
revision: 1
---
## Context

Roadmap P2 carries credential redaction before any model input (A6). A consumer instance ports a
credential-shape detector of its own (297 lines); the detector is generic (inferred), while which
fields of which source reach a model is instance policy.

## Build

Spec first in `ekr.observe`: a redaction step between an observation and any model-facing
projection, with the detected credential shapes named, each redaction recorded (shape and span,
never the value), and the observation itself left immutable. Exposed to consumers through the
library and the session.

## Acceptance

- Each credential shape in a fixture corpus is replaced in the model-facing text, and the stored
  observation's bytes are unchanged.
- The redaction record names shape and span and contains no credential bytes.
- A fixture with no credentials passes through byte-identical.
