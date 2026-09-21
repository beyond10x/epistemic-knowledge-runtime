---
format: aep.planning-md/1
id: task:resolve-ignores-its-marker-parameter
kind: task
status: draft
title: Both resolve functions return a node whatever their marker parameter says
relations:
- serves: vision:o2
revision: 1
---
## What is wrong

`TransientGraph::resolve<T: CanonicalTarget>` and `CanonicalGraph::resolve<R: CanonicalDependency>`
both always return a **node**, whatever `T` or `R` is. So resolving a reference to an edge, or to a
piece of support, compiles and yields a `Node` or nothing.

The marker parameter discriminates **slots** — it keeps a reference to one kind of thing out of a
slot that wants another, which is what its doc claims and what the `trybuild` cases hold. It does
not discriminate **resolution**. `Resolved`, added in wave p1-05, restates that in a new public type
without making it worse.

Found by the adversary of wave p1-05, unit 0, pass 2, and reported as a line rather than a finding
because it is pre-existing and outside that unit's diff.

## Why it has not bitten

Nothing above `ekr-graph` calls `resolve` yet. `CanonicalGraph` holds four maps and only the node map
is reachable through it, so the wrong answer is `None` rather than a wrong object.

## What closes this

Either an associated output type on `CanonicalTarget` so that resolving a reference to an edge
returns an edge, or an explicit statement in both doc comments that resolution is node-only in P1
and the marker is a slot discriminator. The first is the real fix and is a small trait change; the
second is honest and costs nothing.

Do not leave it as it is: a generic function whose return type ignores its parameter is the shape a
reader will assume is polymorphic.
