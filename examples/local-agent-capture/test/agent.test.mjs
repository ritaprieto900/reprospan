import test from "node:test";
import assert from "node:assert/strict";
import { runLocalAgent } from "../dist/agent.js";

test("runs a local decision and tool failure without capturing payloads", () => {
  const bundle = runLocalAgent();
  assert.deepEqual(bundle.events.map((event) => event.kind), [
    "run.started", "model.request", "model.response", "tool.call", "tool.result", "run.failed",
  ]);
  assert.equal(bundle.events[4].output.result, "error");
  assert.equal(bundle.events[5].status, "error");
  const json = JSON.stringify(bundle);
  assert.equal(json.includes("synthetic.missing.policy"), false);
  assert.equal(json.includes("knownPolicyKeys"), false);
});
