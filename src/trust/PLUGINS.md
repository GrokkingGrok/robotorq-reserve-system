# Trust Plugins — Integration Guide

This document describes the minimal plugin integration surface for `rust-trust`.
It is intentionally lightweight: plugins are external services that register with
the Trust by sending a manifest over NATS. Trust communicates with plugins via
NATS subjects (preferred) or HTTP webhooks (declared in the manifest).

Goals
- Provide a simple discovery and lifecycle model for plugins.
- Allow plugins to receive events about contracts and to be invoked for commands.
- Keep plugin execution out-of-process (safer) and use NATS for decoupling.

Core subjects
- `trust.plugin.register` (request) — plugin registers itself. Reply: success/error.
- `trust.plugin.unregister` (request) — plugin removes itself.
- `trust.plugin.list` (request) — request current registered plugins.
- `trust.plugin.command.<plugin_id>` (request) — invoke a command on a plugin.
- `trust.event.<event_type>` (publish) — Trust publishes lifecycle events.

Plugin manifest (overview)
Plugins MUST register a manifest JSON describing identity and capabilities. The
manifest can declare an NATS subject for incoming commands or an HTTP endpoint
for webhook-style invocation.

Minimal fields (explained in `config/PLUGIN_MANIFEST_SCHEMA.json`):
- `plugin_id` — unique id (slug).
- `name`, `version`, `author` — metadata.
- `endpoint_type` — `nats` or `http`.
- `endpoint_subject` — (for `nats`) subject the plugin listens on.
- `endpoint_url` — (for `http`) webhook URL.
- `capabilities` — list of capabilities (e.g., `ui:render`, `accounting:post`).
- `required_permissions` — declared permissions that must be granted.
- `signature` — optional manifest signature (admin approval recommended).

Event types (examples)
- `trust.event.contract.created` — payload: { contract_id, metadata }
- `trust.event.contract.requested` — payload: { contract_id, requester_id }
- `trust.event.contract.served` — payload: { contract_id, served_to }
- `trust.event.payment.made` — payload: { amount, from, to, contract_id }

Message examples

Register (request to `trust.plugin.register`):
```
{
  "plugin_id": "ui-3d-printer",
  "name": "3D Printer UI",
  "version": "0.1.0",
  "author": "Jonathan",
  "endpoint_type": "nats",
  "endpoint_subject": "plugin.ui.3d-printer",
  "capabilities": ["ui:render","contract:subscribe"],
  "required_permissions": ["read_contracts"],
  "signature": null
}
```

Invoke a plugin command (request to `trust.plugin.command.ui-3d-printer`):
```
{
  "command": "render_contract",
  "contract_id": "genesis-0001",
  "context": { }
}
```

Security and operational notes
- Require admin approval for plugins that handle money or sensitive actions.
- Prefer signed manifests. Store the manifest and signature in the plugin registry.
- Use NATS accounts/credentials and ACLs to control who can publish to admin subjects.
- Add per-plugin metrics and instrument calls (counters, errors, latency).

Developer ergonomics
- Provide an SDK (Rust, Python) with helpers for registration, command handling,
  and common logging/trace fields (`plugin_id`, `request_id`).
- Provide a `plugin-test-harness` that can publish events and assert plugin replies.

Roadmap (future)
- Consider WASM-based in-process plugins for low-latency extension, with sandboxing.
- Add plugin sandboxing (resource/time limits) and stricter manifests for signed trust.
- Add web UI plugin registry and admin approval workflow.

Files to edit/implement in the future
- `src/trust/store/plugin_store.rs` — registry persistence
- `src/trust/handlers/plugin_admin.rs` — handle `trust.plugin.*` admin subjects
- `src/trust/examples/plugin_echo.rs` — example plugin demonstrating registration

---
This doc is intentionally small and pragmatic. If you want, I can add the
`PLUGIN_MANIFEST_SCHEMA.json` next and an example plugin that registers itself.
