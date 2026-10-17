---
format: aep.planning-md/3
id: story:readers-emit-metrics-and-request-logs
kind: story
status: implemented
title: The MCP and view readers count each tool call and log each request with its incoming ids
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T10:05:36Z", actor: "agent:claude-ekr-controller", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-17T10:05:36Z", actor: "agent:claude-ekr-controller", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-17T17:18:05Z", actor: "agent:claude-ekr-controller", revision: 5, decided_on: {"recorded":{"test_result":1},"asserted":{"test_result":1}}}
---
## Context

A consumer serving `ekr mcp-http` and `ekr view` reads request metrics from its own proxy, because the
readers emit none, and scrapes Prometheus metrics with a ServiceMonitor or a Pushgateway. It asks for
requests by tool and outcome, a latency histogram, payload bytes, the served head revision and store
query errors, and for one log line per request with the incoming request id and trace context.

`epic:p6-maintenance-observability` promises a metrics surface for epistemic health (design § 61);
operational metrics of the read servers are not in it. Both readers already pass every HTTP request
through `crates/ekr/src/cli/http.rs`, and each dispatches a read by its wire name; the count is taken
at each reader's dispatch, under one registry and one set of names, so this story does not wait for
the shared dispatcher of `story:read-surface-served-over-http`.

## Build (spec first)

1. `systems/ekr/domains/views.yaml` first, green under `ess specify validate --path systems/ekr`
   before any code: the metric names, types, labels and the closed set of `outcome` values (one per
   named refusal class plus success), the `/metrics` endpoint in the Prometheus text exposition
   format, its admission (the same Host allowlist as every other route, off unless asked for), and
   the request log record's fields.
2. Metrics: `ekr_read_requests_total{tool,outcome}`, `ekr_read_duration_seconds{tool}` (a
   histogram with fixed buckets), `ekr_read_result_bytes{tool}` (a histogram of the bytes
   answered), `ekr_read_in_flight`, `ekr_store_head_revision` (the head revision the reader
   serves), `ekr_store_query_errors_total{tool}` (reads that failed on the store, not refusals).
   `tool` is a wire name from the closed set the reader serves, never request text.
3. The log line carries the request id, trace context, wire name, outcome, duration and result
   size. No metric and no log line carries an argument value, result text, document content, a
   prompt or a credential (invariant 9). An incoming `X-Request-Id` or `traceparent` that is
   malformed or oversized is replaced by a generated one and the replacement is logged, never
   echoed raw.
4. `systems/ekr/domains/cli.yaml` and `docs/cli.md`: the option that enables the metrics endpoint
   and the log destination on `ekr mcp-http` and `ekr view`. `CHANGELOG.md`.

## Acceptance

- `tool_call_counted_with_outcome`: after one successful and one refused call to the same tool,
  `/metrics` answers `ekr_read_requests_total` with one per outcome for that tool.
- `result_bytes_match_the_answer`: the result-bytes sum for a call equals the byte length of the
  result document returned.
- `duration_recorded_per_tool`: each call adds one observation to the duration histogram for its
  tool; the test counts observations and asserts nothing about their values.
- `in_flight_returns_to_zero`: with a call held open the gauge reads one, and after it completes zero.
- `head_revision_is_the_served_revision`: after a commit the reader serves, the gauge reads the new
  head.
- `store_query_error_counted`: a read that fails on the store moves the error counter and not the
  refusal outcome.
- `request_log_carries_incoming_ids`: a request with `X-Request-Id` and `traceparent` produces
  exactly one log line carrying both unchanged; a request without them produces one carrying
  generated ones.
- `malformed_trace_header_is_replaced`: an oversized or malformed `traceparent` is not echoed.
- `metrics_hold_no_arguments_or_results`: a call whose arguments and result contain a marker string
  produces metrics and a log line that do not.
- `metrics_endpoint_off_by_default_and_host_checked`: without the option the path answers 404;
  with it, a Host outside the allowlist is refused as on every other route.
- `view_and_mcp_count_alike`: the same read through `ekr view` and `ekr mcp-http` moves the same
  counter for the same wire name.

## Out of scope

Epistemic health metrics of design § 61; tracing spans exported to a collector; metrics of the stdio
MCP server; a Pushgateway client (the consumer pushes or scrapes); authentication of the metrics
endpoint beyond the existing Host and Origin checks.
