# Reprospan

**Replay a failed agent run, change one step, and prove the fix.**

Reprospan is a local-first debugger for tool-using AI agents. It turns a failed run into a redacted, portable reproduction bundle that can be replayed offline, patched at one step, compared semantically, and promoted into a CI regression test.

> Status: pre-alpha. The repository is being built locally before its first public release.

## Product boundary

Reprospan is not another hosted LLM observability dashboard. The first release focuses on one workflow:

1. inspect a failed agent run;
2. freeze it into a safe replay bundle;
3. patch a recorded model or tool result;
4. replay without external side effects;
5. prove the change with deterministic evaluators.

The MVP does not claim deterministic re-execution of live models or tools.

## Development

Prerequisites:

- Node.js 24+
- pnpm 11.13+
- Rust 1.88 with rustfmt and clippy

```bash
pnpm install
pnpm check
pnpm test
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Run the offline failed-run → export → patch → diff → evaluation workflow:

```bash
cargo run -p reprospan-cli -- import \
  --db .reprospan/demo.sqlite \
  --bundle packages/contracts/fixtures/v1/failed-tool-run.bundle.json
cargo run -p reprospan-cli -- export \
  --db .reprospan/demo.sqlite \
  --bundle-id bundle_support_refund_001 > before.json
cargo run -p reprospan-cli -- patch \
  --bundle before.json \
  --patch packages/contracts/fixtures/v1/fix-tool-result.patch.json > after.json
cargo run -p reprospan-cli -- diff --before before.json --after after.json
cargo run -p reprospan-cli -- eval \
  --bundle after.json \
  --eval packages/contracts/fixtures/v1/fix-tool-result.eval.json
```

Commands write one JSON document to stdout, so shell redirection can compose the pipeline. A passed evaluation exits `0`; a completed evaluation with failed assertions still writes its JSON result and exits `1`. The compact built-in demo remains available:

```bash
cargo run -p reprospan-cli -- demo --db .reprospan/demo.sqlite
```

Run the loopback API:

```bash
cargo run -p reprospan-cli -- serve --db .reprospan/server.sqlite
```

The v1 API exposes `GET /healthz`, `POST /v1/bundles/ingest`, and
`GET /v1/bundles/{bundle_id}/timeline`.

## Provider-neutral TypeScript capture

`@reprospan/sdk` builds metadata-only canonical bundles and submits them to the
loopback API. It does not call a model provider or accept prompt bodies, tool
payloads, headers, environment variables, or credentials as capture fields.

Run the deterministic local Agent example against the server:

```bash
cargo run -p reprospan-cli -- serve --db .reprospan/local-agent.sqlite
pnpm --filter @reprospan/example-local-agent-capture build
pnpm --filter @reprospan/example-local-agent-capture start
```

The example performs a real local tool call, records a six-event failed run,
and ingests it through the existing Rust validation and SQLite path. The
`model.*` events describe the local decision step; they do not claim that a
live model was invoked. Replay remains recorded and side-effect free.

## License

Apache-2.0.
