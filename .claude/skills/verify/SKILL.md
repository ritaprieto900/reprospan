---
name: verify
scope: project
---

# Verify Reprospan

Use the real CLI and loopback HTTP surface with the project-pinned Rust toolchain:

```bash
cargo run --manifest-path C:/Users/34964/reprospan/Cargo.toml -p reprospan-cli -- demo --db <fresh-temp-dir>/demo.sqlite
cargo run --manifest-path C:/Users/34964/reprospan/Cargo.toml -p reprospan-cli -- serve --db <fresh-temp-dir>/server.sqlite --listen 127.0.0.1:<free-port>
```

Drive these flows:

1. Demo once: `imported=true`, `exported=true`, `changed_event_count=1`, `eval_passed=true`.
2. Demo again against the same temporary DB: `imported=false`, export/diff/evaluation still pass.
3. CLI pipeline: import → export `before.json` → patch `after.json` → diff → eval.
   - Exported bundle can be imported into a second fresh database.
   - Diff equals `packages/contracts/fixtures/v1/fix-tool-result.diff.json`.
   - Patched eval writes `passed=true` and exits 0.
   - Base eval writes valid `passed=false` JSON and exits 1.
   - Exporting the source DB again exactly matches `before.json`.
4. HTTP: health 200, fixture ingest 201, timeline 200 with `evt_001..evt_004` in order and top-level artifacts preserved.
5. TypeScript capture: build and run `@reprospan/example-local-agent-capture` against the loopback server.
   - Ingest returns 201 and the example reports six events.
   - SDK timeline and CLI export return the same metadata-only failed bundle.
   - The serialized bundle contains no prompt/tool bodies, authorization, credentials, environment values, fixture payload, or server URL.
   - Export can be imported into a second fresh database.
6. Probes: duplicate ingest 409, malformed JSON 400, semantically invalid sequence 400, self/future parent 400, unknown timeline 404, and a non-loopback SDK URL is rejected. Confirm patch/diff/eval were not added as HTTP routes.

Use a unique `mktemp -d` directory. Register an EXIT trap that stops the server and removes only that exact directory. Confirm no `reprospan-cli` process or temporary directory remains.
