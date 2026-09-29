---
format: aep.planning-md/3
id: review-result:adversary-ingest-02-a-pass-1
kind: review-result
status: active
title: Adversary, wave ingest-02 unit A, pass 1
relations:
- reviews: story:add-evidence-operation
revision: 1
---
## Verdict

One red case: rendering a view costs time quadratic in the evidence entries AddEvidence keeps
adding. The kernel side of AddEvidence held under every other attack.

```
unit: story:add-evidence-operation (wave ingest-02, unit A); ekr-i2-a at 7a4d9117 plus 2 adversary test files
verdict: CONFIRMED
cases: executed 457→471, red 1
origin: introduced 1 / pre-existing 0 / undecided 2
```

## Cases added

`crates/ekr-views/tests/adversary_add_evidence_views.rs`: `rendering_costs_linear_time_in_the_evidence_commits_added`
(red: 400 entries 21.36 s, 51.1 times the 0.42 s of 50), and added evidence appearing retained in
the projection, describe_node and ChangesSince (green). `crates/ekr-kernel/tests/adversary_add_evidence_1.rs`:
12 cases (green) and one ignored measurement.

## Attacked and not broken

An assertion before its AddEvidence in one transaction; one payload under two ids, or equal to a
seed payload or to a Canonical object; evidence in a rejected or Stale transaction (nothing stored;
a retry stores once); two transactions adding the same id at one basis (one commits, one Stale);
checkpoints after AddEvidence under profiles v1 and v3 equal to full replay; 0-byte payloads and the
largest payload of /1 and /2; profiles v1–v3; identical roots on file and sqlite.

Cold open from a checkpoint on sqlite: 1.3 / 8.4 / 40.9 / 78.4 ms at 1 / 100 / 300 / 1000 evidence
commits (fastest of 3, debug build); full replay 4.5 ms / 225 ms / 981 ms / 4.42 s. Building 1000
commits took 1178 s with AddEvidence against 415 s without: every command re-reads each Provenance
object, linear per command.

```findings
- file: crates/ekr-views/src/lib.rs
  line: 256
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: load reads each evidence entry's retention through Runtime::content, one full history reload per entry, so rendering a view costs time quadratic in the evidence AddEvidence keeps adding (50 entries 0.42 s, 400 entries 21.4 s)
- file: docs/cli.md
  line: 1011
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the AddEvidence payload ceiling is the sequence_elements limit, 16384 bytes in /2 and 4096 in /1, and neither the new section nor ekr operations AddEvidence states it
- file: crates/ekr/tests/adversary_page_stream_1.rs
  line: 1244
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: "a_stream_cut_before_its_end_line_is_failed_and_keeps_what_arrived fails 3 of 3 on this tree with drawn [false,false] and 'the expansion failed - network error', while the other failures of that file are browser stalls under load"
```
