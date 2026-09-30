---
format: aep.planning-md/3
id: task:two-tests-pass-under-load
kind: task
status: draft
title: Two tests pass under machine load
relations:
- serves: vision:o5
revision: 1
---
## Context

Two tests fail under machine load and pass when run alone, observed in waves checks-01, sdk-03 and
reads-04 (2026-09-29 and 2026-09-30):

- `crates/ekr/tests/adversary_page_stream_1.rs`
  (`a_stream_cut_before_its_end_line_is_failed_and_keeps_what_arrived`, "network error", at
  load 24–47);
- `crates/ekr/tests/adversary_sdk01_h_replaced_store.rs:118` ("precondition: a directory is handed
  the freed inode").

The second asserts an operating-system detail (inode reuse) as a precondition. A red run on CI would
block a release for a timing reason.

## Build

- Make each case independent of load: the page stream case waits on the event it asserts rather than
  a time; the replaced-store case skips, stating why, when the filesystem does not hand back the
  freed inode, instead of failing, and still asserts the reopen when it does.

## Acceptance

- 20 consecutive runs of each file pass while another full `cargo test` runs.
