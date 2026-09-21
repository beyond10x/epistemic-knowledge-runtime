---
format: aep.planning-md/1
id: task:public-surface-covers-types
kind: task
status: draft
title: The public-surface guard reads types, not only functions and constants
relations:
- derived_from: story:ontology-types-and-values
- serves: vision:o2
revision: 1
---
## Context

The workspace public-surface guard (`crates/ekr/tests/public_surface.rs`, lifted in wave p1-03) reads only `pub fn`, `pub const fn` and `pub const` as declarations. A `pub struct`, `pub enum`, `pub trait` or `pub type` is invisible to it, so a type can arrive with no case naming it and nothing turns red.

It was not widened in the same move because the two halves do not widen together. The use criterion is `::name` or `.name` — a method or path position. A *type* is used as `Type::…`, `&Type`, `: Type`, `Type {` or `-> Type`, and none of those match, so widening the declaration side alone would report every type in the workspace as untested and the guard would have to be switched off to land.

## Acceptance

A `pub struct` added to any crate and named by no case turns `no_public_item_in_any_crate_is_untested` red, and the case stays green on the workspace as it stands.

## Notes

The use side needs its own criterion for types: a bare occurrence not preceded by an identifier character and not inside a comment is probably enough, but it is looser than the method rule and should be measured against the current suite before it lands — a guard that reports nothing is the defect this whole line of work exists to remove.

## Scope

- `crates/ekr/tests/public_surface.rs`
