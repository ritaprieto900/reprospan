# Local Agent capture example

This example runs a deterministic TypeScript decision step and a real local tool function. It records only bounded metadata and safe summaries, then sends the failed run to Reprospan's loopback ingest API.

It does not call a live model provider, capture prompt or tool bodies, or re-execute anything during replay.

```bash
cargo run -p reprospan-cli -- serve --db .reprospan/local-agent.sqlite
pnpm --filter @reprospan/example-local-agent-capture build
pnpm --filter @reprospan/example-local-agent-capture start
```

Pass a different loopback URL as the first argument when the server uses another port:

```bash
pnpm --filter @reprospan/example-local-agent-capture start -- http://127.0.0.1:9876
```
