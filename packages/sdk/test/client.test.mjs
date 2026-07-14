import test from "node:test";
import assert from "node:assert/strict";
import { LoopbackClient, ReprospanHttpError } from "../dist/client.js";

const bundle = {
  schema_version: "reprospan.bundle.v1",
  bundle_id: "bundle_client_test",
  created_at: "2026-07-14T00:00:00Z",
  capture_policy: { mode: "metadata_only", redacted: true },
  events: [{
    schema_version: "reprospan.event.v1",
    event_id: "evt_client_001",
    run_id: "run_client_test",
    sequence: 0,
    occurred_at: "2026-07-14T00:00:00Z",
    kind: "run.started",
    status: "ok",
    attributes: {},
  }],
};

test("ingests validated bundles through the existing loopback route", async () => {
  let request;
  const client = new LoopbackClient({
    fetch: async (input, init) => {
      request = new Request(input, init);
      return Response.json(bundle, { status: 201 });
    },
  });

  assert.deepEqual(await client.ingest(bundle), bundle);
  assert.equal(request.url, "http://127.0.0.1:8787/v1/bundles/ingest");
  assert.equal(request.method, "POST");
  assert.equal(request.headers.get("content-type"), "application/json");
});

test("rejects non-loopback URLs and preserves stable server errors", async () => {
  assert.throws(() => new LoopbackClient({ baseUrl: "https://example.com" }), /loopback/);
  const client = new LoopbackClient({
    fetch: async () => Response.json(
      { code: "bundle_exists", message: "bundle already exists" },
      { status: 409 },
    ),
  });
  await assert.rejects(
    () => client.ingest(bundle),
    (error) => error instanceof ReprospanHttpError && error.status === 409 && error.code === "bundle_exists",
  );
});
