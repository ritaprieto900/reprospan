# Architecture

Reprospan is a local-first debugging tool, not a hosted observability service.

## Boundaries

- TypeScript owns canonical bundle construction, the provider-neutral capture SDK, local examples, and future provider adapters and web UI.
- Rust owns event normalization, storage, replay, diffing, evaluation, HTTP APIs, and the CLI.
- TypeScript and Rust communicate through versioned JSON over loopback HTTP.
- The current TypeScript implementation includes the provider-neutral capture SDK and a deterministic local Agent example. Provider-specific adapters and the web UI remain planned surfaces.
- Large or raw payloads are content-addressed artifacts; the canonical event model stores references.
- OTLP is an ingestion protocol. OpenTelemetry GenAI conventions are mapped through versioned adapters and are not the database ABI.

## Replay language

- **Playback:** render recorded events in their original order.
- **State reconstruction:** rebuild the recorded agent context without executing it.
- **Simulated replay:** return recorded or explicitly mocked model/tool results.
- **Active re-execution:** invoke live models or tools; outside the MVP and disabled by default.
