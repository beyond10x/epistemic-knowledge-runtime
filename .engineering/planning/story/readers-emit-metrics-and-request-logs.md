---
format: aep.planning-md/3
id: story:readers-emit-metrics-and-request-logs
kind: story
status: draft
title: The MCP and view readers count each tool call and log each request with its incoming ids
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
- depends_on: story:read-surface-served-over-http
revision: 1
---
## Context

A consumer serving `ekr mcp-http` and `ekr view` behind its own proxy reads request metrics from that
proxy, because the readers emit none. It asks for per-tool metrics and one log line per request
carrying the incoming request id and trace context, so a request can be followed from the caller
through the reader. It made the same request earlier; nothing in the store recorded it.

`epic:p6-maintenance-observability` promises a metrics surface for epistemic health (design § 61);
operational metrics of the read servers are not in it, and no story under it covers them.
`story:read-surface-served-over-http` puts both readers on one dispatcher keyed by wire name, which is
the one place a per-tool count can be taken for both, so this story depends on it.

Requested:

- `ekr_mcp_tool_calls_total{tool,outcome}`, `ekr_mcp_tool_duration_seconds{tool}`,
  `ekr_mcp_result_bytes{tool}`, `ekr_http_in_flight`;
- one log line per request with the incoming `X-Request-Id` and `traceparent`.

## Build (spec first)

1. `systems/ekr/domains/views.yaml` first, green under `ess specify validate --path systems/ekr`
   before any code: the metric names, types, labels and the closed set of `outcome` values (one per
   named refusal class plus success), the endpoint that serves them and its admission (the same
   Host allowlist as every other route, off unless asked for), and the request log record's fields.
2. The log line carries the request id, trace context, wire name, outcome, duration and result
   size. It carries no argument values, no result text and no credential (invariant 9). An incoming
   `X-Request-Id` or `traceparent` that is malformed or oversized is replaced by a generated one and
   the replacement is logged, never echoed raw.
3. `systems/ekr/domains/cli.yaml` and `docs/cli.md`: the option that enables the metrics endpoint and
   the log destination on `ekr mcp-http` and `ekr view`.

## Acceptance

- `tool_call_counted_with_outcome`: after one successful and one refused call to the same tool, the
  metrics endpoint answers `ekr_mcp_tool_calls_total` with one per outcome for that tool.
- `result_bytes_match_the_answer`: `ekr_mcp_result_bytes` for a call equals the byte length of the
  result document returned.
- `duration_recorded_per_tool`: each call adds one observation to `ekr_mcp_tool_duration_seconds` for
  its tool; the test counts observations and asserts nothing about their values.
- `in_flight_returns_to_zero`: with a call held open the gauge reads one, and after it completes zero.
- `request_log_carries_incoming_ids`: a request with `X-Request-Id` and `traceparent` produces exactly
  one log line carrying both unchanged; a request without them produces one carrying generated ones.
- `malformed_trace_header_is_replaced`: an oversized or malformed `traceparent` is not echoed.
- `log_line_holds_no_arguments_or_results`: a call whose arguments and result contain a marker
  string produces a log line that does not.
- `metrics_endpoint_off_by_default_and_host_checked`: without the option the path answers 404; with
  it, a Host outside the allowlist is refused as on every other route.
- `view_and_mcp_count_alike`: the same call through `ekr view` and `ekr mcp-http` moves the same
  counter for the same wire name.

## Out of scope

Epistemic health metrics of design § 61; tracing spans exported to a collector; metrics of the stdio
MCP server; authentication of the metrics endpoint beyond the existing Host and Origin checks.
