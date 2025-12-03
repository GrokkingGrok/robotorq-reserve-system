# Message Envelope & Trace Propagation

This document specifies the minimal message envelope and header conventions used by the RoboTorq Reserve System for trace propagation and cross-service correlation.

Goals:
- Be transport-agnostic: header names are simple strings and map easily to NATS message headers, HTTP headers, or any key/value envelope.
- Follow W3C Trace Context where possible so standard tracers and collector components interoperate.

Recommended headers
- `traceparent`: W3C traceparent header (required when present). Example: `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`.
- `tracestate`: W3C tracestate (optional). Example: `vendor1=opaque,vendor2=more`.
- `rtq-message-id`: a RoboTorq message id (optional) — useful for debugging and log correlation.
- `rtq-origin-service`: the originating service name (optional).

Minimal envelope payload

- `headers` (map<string,string>): includes the propagation headers above.
- `body` (binary / JSON): the message body for application consumption.

Transport adaptation
- HTTP: map headers directly to HTTP request headers.
- NATS: use the NATS message headers map (or the subject plus headers) to carry `traceparent`/`tracestate` and the `rtq-*` keys.

Usage patterns
- Injecting trace context: when sending a message, call the propagation helper to write `traceparent` and `tracestate` into the envelope headers map before sending.
- Extracting trace context: on receive, extract `traceparent`/`tracestate` from the headers map and use the tracing API to resume or continue the trace.

Implementation notes
- The commons helper `crates/commons/src/util/tracing/propagation.rs` provides small, dependency-free functions that operate on `HashMap<String,String>` so adapters (HTTP, NATS) can call them without pulling heavy tracing dependencies into transport crates.
- Keep the envelope small: only carry what you need for correlation. Avoid embedding serialized spans or large diagnostic blobs into the message headers.

Security and privacy
- Treat `traceparent` like any other header: it is not a secret but may contain correlation identifiers. Do not put any sensitive data into headers.

Extensions
- If you need additional routing or application metadata add `rtq-*` prefixed headers to avoid collision with third-party headers.

Revision
- 2025-12-03: Initial draft added by automation during OTLP/propagation work.
