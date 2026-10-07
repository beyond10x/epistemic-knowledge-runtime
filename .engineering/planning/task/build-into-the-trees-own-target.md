---
format: aep.planning-md/3
id: task:build-into-the-trees-own-target
kind: task
status: active
title: Every tree builds into its own target/ and ends with finish --discard-cache --archive
relations:
- serves: vision:o2
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-07T02:15:41Z", actor: "agent:claude-ekr-controller", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-07T02:15:41Z", actor: "agent:claude-ekr-controller", revision: 3}
---
## What is wrong

`AGENTS.md` § The gate tells every tree of this repository to build into one shared directory,
`$HOME/.cache/b10x-target/epistemic-knowledge-runtime`. The same section then records, measured on
2026-09-21, that two checkouts sharing a build directory write the same `deps/` filenames, so one
tree's `cargo test` can run another tree's test binary and report the other tree's compile-time
path, and that `doc/<crate>` and `debug/ekr` are single shared paths. The instruction contradicts
the evidence written under it.

The operator's standing rule for every repository (2026-10-06) is the opposite: build into the
tree's own `target/`, never set `CARGO_TARGET_DIR`, and end every tree with
`worktree finish --discard-cache --archive <tree>`, which deletes only build cache it recognises by
structure and archives everything else. The shared directory also grew where no tree's cleanup
could see it.

## Build

Rewrite the build paragraph of `AGENTS.md` § The gate (line 127 on 2026-10-06):

- build into the tree's own `target/`; never set `CARGO_TARGET_DIR`;
- end every tree with `worktree finish --discard-cache --archive <tree>`;
- before a gate's result counts as evidence, check with `cargo test -- --list` that the tests the
  run printed exist in the gated tree.

Keep the dated observations of 2026-09-21 below it: they are why the rule exists. Replace the
section's last sentence, which asks a wave to give each unit its own directory and say so in its
page, with the fact that each tree's own `target/` already does that.

Dated wave pages under `.engineering/waves/` that name the shared directory are records of what a
wave did and are not rewritten.

## Acceptance

- `AGENTS.md` names no `b10x-target` path and no instruction to set `CARGO_TARGET_DIR`
  (`grep -n 'b10x-target\|Set .CARGO_TARGET_DIR' AGENTS.md` prints nothing).
- `AGENTS.md` § The gate states the tree-own `target/` rule, the
  `worktree finish --discard-cache --archive` ending and the `cargo test -- --list` check.
- The change touches `AGENTS.md` only, and `task check` exits 0 on the integration branch.

## Scope

- `AGENTS.md` — cited (§ The gate, lines 121–177 on 2026-10-07).
