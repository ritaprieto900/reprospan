import test from "node:test";
import assert from "node:assert/strict";
import { CaptureSession } from "../dist/capture-session.js";
import { assertValidBundle } from "../dist/validator.js";

function fixtureSession() {
  const times = [
    "2026-07-14T00:00:00Z",
    "2026-07-14T00:00:00Z",
    "2026-07-14T00:00:01Z",
    "2026-07-14T00:00:02Z",
    "2026-07-14T00:00:03Z",
    "2026-07-14T00:00:04Z",
    "2026-07-14T00:00:05Z",
  ];
  const ids = ["evt_local_001", "evt_local_002", "evt_local_003", "evt_local_004", "evt_local_005", "evt_local_006"];
  return new CaptureSession({
    bundleId: "bundle_local_agent_001",
    runId: "run_local_agent_001",
    runName: "local-policy-agent",
    attributes: { scenario: "synthetic_policy_lookup" },
    now: () => times.shift(),
    nextEventId: () => ids.shift(),
  });
}

test("captures a deterministic metadata-only failed agent run", () => {
  const session = fixtureSession();
  const request = session.recordModelRequest({
    parentEventId: session.startedEventId,
    provider: "local",
    model: "deterministic-policy-router",
    operation: "select_tool",
  });
  const response = session.recordModelResponse({
    parentEventId: request,
    provider: "local",
    model: "deterministic-policy-router",
    operation: "select_tool",
    result: "ok",
    summary: "Selected local policy lookup tool",
  });
  const call = session.recordToolCall({
    parentEventId: response,
    name: "lookup_policy",
    inputShape: "synthetic_policy_key",
  });
  const result = session.recordToolResult({
    parentEventId: call,
    name: "lookup_policy",
    result: "error",
    summary: "No matching synthetic policy fixture",
    errorCode: "LOCAL_POLICY_NOT_FOUND",
  });
  session.fail({ parentEventId: result, reason: "tool_result_error" });

  const bundle = session.finalize();
  assertValidBundle(bundle);
  assert.deepEqual(bundle.events.map((event) => event.sequence), [0, 1, 2, 3, 4, 5]);
  assert.deepEqual(bundle.events.map((event) => event.event_id), [
    "evt_local_001", "evt_local_002", "evt_local_003", "evt_local_004", "evt_local_005", "evt_local_006",
  ]);
  assert.equal(bundle.events[4].status, "error");
  assert.equal(bundle.events[4].output.result, "error");
  assert.deepEqual(bundle.capture_policy, { mode: "metadata_only", redacted: true });
});

test("rejects unknown parents, duplicate ids, and mutation after terminal", () => {
  const unknownParent = fixtureSession();
  assert.throws(
    () => unknownParent.recordToolCall({ parentEventId: "evt_future", name: "tool", inputShape: "key" }),
    /parent event must already exist/,
  );

  const duplicate = new CaptureSession({
    bundleId: "bundle_duplicate",
    runId: "run_duplicate",
    runName: "duplicate-agent",
    now: () => "2026-07-14T00:00:00Z",
    nextEventId: () => "evt_duplicate",
  });
  assert.throws(
    () => duplicate.recordToolCall({ parentEventId: duplicate.startedEventId, name: "tool", inputShape: "key" }),
    /duplicated/,
  );

  const terminal = fixtureSession();
  terminal.fail({ parentEventId: terminal.startedEventId, reason: "stopped" });
  assert.throws(
    () => terminal.recordToolCall({ parentEventId: terminal.startedEventId, name: "tool", inputShape: "key" }),
    /already terminal/,
  );
});

test("only projects explicit metadata fields into canonical events", () => {
  const session = fixtureSession();
  const eventId = session.recordModelRequest({
    parentEventId: session.startedEventId,
    provider: "local",
    model: "router",
    operation: "select_tool",
    prompt: "secret prompt body",
    headers: "authorization token",
    env: "SECRET=value",
  });
  session.fail({ parentEventId: eventId, reason: "stopped" });
  const json = JSON.stringify(session.finalize());
  assert.equal(json.includes("secret prompt body"), false);
  assert.equal(json.includes("authorization token"), false);
  assert.equal(json.includes("SECRET=value"), false);
});
