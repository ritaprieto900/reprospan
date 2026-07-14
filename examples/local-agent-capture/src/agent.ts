import { CaptureSession, type Bundle } from "@reprospan/sdk";
import { lookupPolicy } from "./local-tool.js";

interface Decision {
  tool: "lookup_policy";
  policyKey: string;
}

export function runLocalAgent(): Bundle {
  const session = new CaptureSession({
    bundleId: `bundle_local_${crypto.randomUUID()}`,
    runId: `run_local_${crypto.randomUUID()}`,
    runName: "local-policy-agent",
    attributes: { scenario: "synthetic_policy_lookup" },
  });

  const requestId = session.recordModelRequest({
    parentEventId: session.startedEventId,
    provider: "local",
    model: "deterministic-policy-router",
    operation: "select_tool",
  });
  const decision: Decision = {
    tool: "lookup_policy",
    policyKey: "synthetic.missing.policy",
  };
  const responseId = session.recordModelResponse({
    parentEventId: requestId,
    provider: "local",
    model: "deterministic-policy-router",
    operation: "select_tool",
    result: "ok",
    summary: "Selected local policy lookup tool",
  });
  const callId = session.recordToolCall({
    parentEventId: responseId,
    name: decision.tool,
    inputShape: "synthetic_policy_key",
  });

  const toolResult = lookupPolicy(decision.policyKey);
  const resultId = session.recordToolResult({
    parentEventId: callId,
    name: decision.tool,
    ...toolResult,
  });

  if (toolResult.result === "error") {
    session.fail({ parentEventId: resultId, reason: "tool_result_error" });
  } else {
    session.complete({ parentEventId: resultId, reason: "tool_result_ok" });
  }
  return session.finalize();
}
