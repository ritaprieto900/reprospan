import assert from "node:assert/strict";
import test from "node:test";
import { runOpenAIAgent } from "../dist/agent.js";

test("runs an OpenAI-based agent with a real tool failure without capturing prompt bodies", async () => {
  const mockClient = {
    chat: {
      completions: {
        async create() {
          return {
            choices: [{
              finish_reason: "tool_calls",
              message: {
                content: null,
                tool_calls: [{
                  function: {
                    name: "lookup_policy",
                    arguments: '{"policy_key":"synthetic.missing.policy"}',
                  },
                }],
              },
            }],
          };
        },
      },
    },
  };

  const bundle = await runOpenAIAgent(
    "http://127.0.0.1:8787",
    mockClient,
    async () =>
      JSON.stringify({
        result: "error",
        summary: "No matching synthetic policy fixture",
        errorCode: "LOCAL_POLICY_NOT_FOUND",
      }),
  );

  const kinds = bundle.events.map((event) => event.kind);
  assert.ok(kinds[0] === "run.started");
  assert.ok(kinds.includes("model.request"));
  assert.ok(kinds.includes("model.response"));
  assert.ok(kinds.includes("tool.call"));
  assert.ok(kinds.includes("tool.result"));

  const toolResultEvent = bundle.events.find((event) => event.kind === "tool.result");
  assert.ok(toolResultEvent);
  assert.equal(toolResultEvent.output.result, "error");
  assert.equal(toolResultEvent.status, "error");

  const serialized = JSON.stringify(bundle);
  assert.equal(serialized.includes("order tier"), false);
  assert.equal(serialized.includes("synthetic.missing.policy"), false);
  assert.equal(serialized.includes("sk-test"), false);
  assert.equal(serialized.includes("system"), false);
});
