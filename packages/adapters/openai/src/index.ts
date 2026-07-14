import type { ModelRequestInput, ModelResponseInput, ScalarAttribute } from "@reprospan/sdk";

/**
 * Minimal provider-side shape. This adapter only projects safe metadata;
 * it never records prompt bodies, raw tool arguments, or response
 * contents into canonical events.
 */
export function openaiToModelRequest(params: {
  parentEventId: string;
  model: string;
  operation: string;
  toolNames?: string[];
}): ModelRequestInput {
  const { parentEventId, ...rest } = params;
  return {
    parentEventId,
    provider: "openai",
    model: rest.model,
    operation: rest.operation,
  };
}

export function openaiToModelResponse(params: {
  parentEventId: string;
  model: string;
  operation: string;
  result: "ok" | "error";
  summary: string;
}): ModelResponseInput {
  return {
    parentEventId: params.parentEventId,
    provider: "openai",
    model: params.model,
    operation: params.operation,
    result: params.result,
    summary: params.summary,
  };
}

export function openaiSafeToolAttributes(args: {
  name: string;
  inputShape: string;
}): { name: string; inputShape: string } {
  return { name: args.name, inputShape: args.inputShape };
}

export function openaiSafeResultSummary(params: {
  toolName: string;
  ok: boolean;
  description: string;
  maxLength?: number;
}): string {
  const base = `${params.toolName}: ${params.ok ? "ok" : "error"} — ${params.description}`;
  const cap = params.maxLength ?? 512;
  if (base.length <= cap) return base;
  const trunc = base.slice(0, cap - 3);
  return `${trunc}…`;
}

export function openaiToolResultAttributes(params: {
  errorCode?: string;
}): Record<string, ScalarAttribute> {
  const attrs: Record<string, ScalarAttribute> = {};
  if (params.errorCode) {
    attrs.error_code = params.errorCode;
  }
  return attrs;
}
