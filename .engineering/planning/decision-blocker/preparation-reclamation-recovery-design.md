---
format: aep.planning-md/3
id: decision-blocker:preparation-reclamation-recovery-design
kind: decision-blocker
status: open
title: Preparation reclamation needs a compatible recovery design
relations:
- blocks: story:preparation-blobs-are-reclaimed
revision: 1
---
The story's dependencies have landed, but its own Not established section still lacks the
size-baseline harness and compatible receipt design. Current preparation::read_preparation follows
private attempt blobs, and inventory::prepared_occurrence reads the latest private preparation.
Removing those bytes without a replacement breaks retry or migration. Clear only with an accepted
design specifying compatibility, crash points, inventory/migration readers and a reproducible
baseline. This blocker prevents implementation dispatch, not read-only investigation.
