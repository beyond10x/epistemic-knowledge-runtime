# CLI and explanation dispatch contract

Prepared against coordinator d147858 after the seed checkpoint, resolving the
read-only review in p1-cli-explain-contract-review.md. DESIGN 93 already owns the
durable write responses. Host transport and the remaining Snapshot/Explain
declarations are now active in kernel ESS in every participating tree. The
original adjacent patch, draft and review remain unchanged as preparation
evidence; do not apply that patch again.

Host input decoding is implemented and independently reviewed in
p1-cli-host-input-review.md. Snapshot, Explain and the complete CLI still await
their real-handler implementation and acceptance. Specification qualification
does not establish those commands' execution.

## Host configuration and entry points

The local operator deliberately selects trusted configuration with required
global --host <file>. This is a JSON document whose complete envelope is
CliHostConfigurationV1: format exactly "ekr.cli-host/1", tenant, context and
authority. context is the existing BootstrapContext; authority is the existing
AuthorityStateV1. Decode directly into strict typed carriers: reject unknown or
duplicate fields, duplicate decoded maps/sets, unsupported format and missing
required fields. No transaction or seed field supplies this configuration.

The CLI transport DTO and decoder live in crates/ekr/src/host.rs. Kernel ESS
declares its projection alongside the existing context/authority types; it does
not become a persisted record or a command input from an untrusted proposer.
The native provider namespace comes from host tenant. Required --store <path>
and --backend sqlite|file select the configured local provider. There is no
implicit anonymous registry, validator, actor override or replacement ontology.

The public kernel Runtime opens the selected provider with that trusted context
and authority, and derives ontology from verified retained seed input. CLI uses
the configured operator for Seed, Propose, Commit and reads; Validate runs the
distinct profile validator. Controlled library/conformance hosts pass the same
typed context/authority through the same Runtime entry points and inject their
clock. No raw-store dependency or alternate application path enters the CLI.

Malformed or unsupported host configuration is a configuration fault, exit 1.
Preserve the opener's typed distinction between valid-but-different host anchor
and corrupt state. Seed maps only the former to AlreadySeeded, exit 2, with no
state exposure or writes; other commands refuse anchor mismatch. Named kernel
refusals exit 2, provider/verification failures exit 1, and all declared command
outcomes, including recorded rejection/staleness, exit 0 with their actual record.
Clap's argument/help handling retains its normal CLI behavior.

Host time comes from the system clock, supplied lazily for a new decision.
Retained-result lookup must precede the callback. Replay receives recorded time.
For --valid-at accept canonical decimal milliseconds or exactly YYYY-MM-DD,
meaning midnight UTC. Use the existing time crate to validate the calendar;
do not broaden core Timestamp parsing or invent a second calendar algorithm.
The CLI adds only the already-declared serde and time workspace dependencies.
Seed/proposal documents remain their declared YAML formats; Propose hands its
reader directly to the shared bounded ingress path, without an unlimited pre-read.

## Snapshot and explanation

SnapshotResult retains the complete selected graph and root plus revision_id.
Optional valid_at and matching_assertions are jointly absent without a selector;
with one they carry that selector and the shared valid_at result in stable ID
order, including an empty list. A filtered view never claims the full root's hash.

Explain captures one verified revision/history boundary before selecting links;
ExplanationResult.at names it. It must not gather a newer lifecycle change from
a second head read. Start with the requested assertion. For each selected
assertion emit its actual assertion, origin admission (Seed or Proposal,
Validation, Commit), then its lifecycle changes through the captured revision
in revision order. An ordinary origin includes complete retained records, not
reconstructed substitutes. A seed origin contains its original result, actual
bootstrap context/profile and record address.

A supersession selects its actual replacement assertion for the same treatment,
including when the replacement was accepted in an earlier transaction. Process
the requested assertion first, then newly selected replacement IDs in stable
order; visit each assertion once. The verified history must establish the
replacement's acceptance, not merely supply a lifecycle pointer.

The final Evidence links are the union of each selected assertion's evidence and
the complete transaction evidence sets of included ordinary origin/lifecycle
receipts. Deduplicate by EvidenceId, order by that ID, and verify every selected
record and content payload at its actual address. This includes the replacement's
support even when it differs from the original assertion's support. An unrelated
unused seed payload does not become assertion support. HumanStatement terminates
directly; no Observation or model step is fabricated. Missing required data
refuses the chain. Explained.links equals the actual output length.

## Decisive acceptance

Keep the original generated obligations and add real-handler controls for:
the host decoding/refusal branches above; exact Seed/Commit retry without clock
sampling; both valid-time selections at one latest revision and an earlier
revision; a supersession whose assertions have distinct evidence; a replacement
accepted before the supersession; and corruption of required support. Hold one
captured Explain boundary while another valid commit advances the provider.
The host decoder controls are executed in the named bounded review. All remaining
command controls are unexecuted preparation.

Released ESS validation, compilation and declared-coverage synthesis exited
successfully for the exact adjacent proposal. No scenario was executed. Measured
suite counts follow:

- generated: 35; authored: 0; refused: 0; outside: 0.
