import { CaptureSession, type Bundle } from "@reprospan/sdk";
import { openaiSafeResultSummary } from "@reprospan/adapter-openai";
import type { PolicyLookupResult } from "./local-tool.js";

type ToolCallback = (name: string, input: Record<string, unknown>) => Promise<string>;

interface CompletionsClient {
  chat: {
    completions: {
      create(params: {
        model: string;
        messages: Array<{ role: string; content: string }>;
        tools?: Array<Record<string, unknown>>;
      }): Promise<{
        choices: Array<{
          finish_reason: string;
          message: {
            content?: string | null;
            tool_calls?: Array<{
              function: {
                name: string;
                arguments: string;
              };
            }>;
          };
        }>;
      }>;
    };
  };
}

export async function runOpenAIAgent(
  baseUrl: string,
  client: CompletionsClient,
  toolCallback: ToolCallback,
): Promise<Bundle> {
  const llm = "gpt-4.1-mini";
  const session = new CaptureSession({
    bundleId: `bundle_openai_${crypto.randomUUID()}`,
    runId: `run_openai_${crypto.randomUUID()}`,
    runName: "openai-policy-agent",
    attributes: { scenario: "local_refund_lookup_openai" },
  });

  const messages: Array<{ role: string; content: string }> = [
    {
      role: "system",
      content: `You determine which tool to call for a customer support query.
You have access to a tool named "lookup_policy" that takes a policy_key string.
Respond with a tool call to "lookup_policy" with an appropriate policy_key.`,
    },
    {
      role: "user",
      content: "A customer with order tier 'premium' requests a refund for a late delivery. What policy should we check?",
    },
  ];

  const reqId = session.recordModelRequest({
    parentEventId: session.startedEventId,
    provider: "openai",
    model: llm,
    operation: "chat_completion_with_tools",
  });

  let completion;
  try {
    completion = await client.chat.completions.create({
      model: llm,
      messages,
      tools: [
        {
          type: "function",
          function: {
            name: "lookup_policy",
            description: "Look up a refund policy by key",
            parameters: {
              type: "object",
              properties: { policy_key: { type: "string" } },
              required: ["policy_key"],
            },
          },
        },
      ],
    });
  } catch (openaiError) {
    session.recordModelResponse({
      parentEventId: reqId,
      provider: "openai",
      model: llm,
      operation: "chat_completion_with_tools",
      result: "error",
      summary: openaiSafeResultSummary({
        toolName: "",
        ok: false,
        description: `OpenAI API call failed: ${String(openaiError)}`,
      }),
    });
    session.fail({ parentEventId: reqId, reason: "model_call_failed" });
    return session.finalize();
  }

  const choice = completion.choices[0];
  const toolCalls = choice.message.tool_calls ?? [];
  const finishReason = choice.finish_reason;

  const decision = toolCalls.length > 0 ? toolCalls[0].function : null;

  session.recordModelResponse({
    parentEventId: reqId,
    provider: "openai",
    model: llm,
    operation: "chat_completion_with_tools",
    result: "ok",
    summary: openaiSafeResultSummary({
      toolName: decision?.name ?? "none",
      ok: true,
      description: decision
        ? `Model requested tool call: ${decision.name}`
        : `Model finished with: ${finishReason}`,
    }),
  });

  if (!decision) {
    session.fail({ parentEventId: reqId, reason: "no_tool_selected" });
    return session.finalize();
  }

  const toolInput: Record<string, unknown> = JSON.parse(decision.arguments);

  const callId = session.recordToolCall({
    parentEventId: reqId,
    name: decision.name,
    inputShape: String(Object.keys(toolInput).sort().join(",")),
  });

  const toolResultStr = await toolCallback(decision.name, toolInput);
  let toolResult: PolicyLookupResult;
  try {
    toolResult = JSON.parse(toolResultStr) as PolicyLookupResult;
  } catch {
    toolResult = { result: "error", summary: toolResultStr, errorCode: "PARSE_ERROR" };
  }

  const resultId = session.recordToolResult({
    parentEventId: callId,
    name: decision.name,
    result: toolResult.result,
    summary: openaiSafeResultSummary({
      toolName: decision.name,
      ok: toolResult.result === "ok",
      description: toolResult.summary,
    }),
    errorCode: "errorCode" in toolResult ? toolResult.errorCode : undefined,
  });

  if (toolResult.result === "error") {
    session.fail({ parentEventId: resultId, reason: "tool_result_error" });
  } else {
    session.complete({ parentEventId: resultId, reason: "tool_result_ok" });
  }
  return session.finalize();
}
