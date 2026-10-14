---
format: aep.planning-md/3
id: decision-blocker:preparation-reclamation-recovery-design
kind: decision-blocker
status: cleared
title: Preparation reclamation needs a compatible recovery design
relations:
- blocks: story:preparation-blobs-are-reclaimed
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-14T12:20:58Z", actor: "agent:claude-ekr-controller", revision: 3}
---
The story's dependencies have landed, but its own Not established section still lacks the
size-baseline harness and compatible receipt design. Current preparation::read_preparation follows
private attempt blobs, and inventory::prepared_occurrence reads the latest private preparation.
Removing those bytes without a replacement breaks retry or migration. Clear only with an accepted
design specifying compatibility, crash points, inventory/migration readers and a reproducible
baseline. This blocker prevents implementation dispatch, not read-only investigation.

## Answer

Decided 2026-10-14 by the operator: a preparation's private blob is reclaimed once its publication is definite, and inventory and migration read the receipt's existing proposal copy instead; the receipt format does not change. A new receipt format is considered only if the measured result misses the story's target. This is option A; an architecture decision record amends ADR 0009's "missing or corrupt preparation refuses".
