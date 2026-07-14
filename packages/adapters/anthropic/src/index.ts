import type { ModelRequestInput, ModelResponseInput, ScalarAttribute } from "@reprospan/sdk";

/**
 * Projects safe metadata from an Anthropic API tool-use interaction
 * into canonical events. This adapter never records prompt bodies, raw
 * tool arguments, system prompts, or response text.
 */

export function anthropicToModelRequest(params: {
  parentEventId: string;
  model: string;
  operation: string;
  toolNames?: string[];
}): ModelRequestInput {
  return {
    parentEventId: params.parentEventId,
    provider: "anthropic",
    model: params.model,
    operation: params.operation,
  };
}

export function anthropicToModelResponse(params: {
  parentEventId: string;
  model: string;
  operation: string;
  result: "ok" | "error";
  summary: string;
}): ModelResponseInput {
  return {
    parentEventId: params.parentEventId,
    provider: "anthropic",
    model: params.model,
    operation: params.operation,
    result: params.result,
    summary: params.summary,
  };
}

/** Describes which tool was called without recording its arguments. */
export function anthropicSafeToolName(name: string): string {
  return name;
}

/** Builds a bounded, stable result summary that never leaks raw output. */
export function anthropicSafeResultSummary(params: {
  toolName: string;
  ok: boolean;
  description: string;
  maxLength?: number;
}): string {
  const base = `${params.toolName}: ${params.ok ? "ok" : "error"} — ${params.description}`;
  const cap = params.maxLength ?? 512;
  if (base.length <= cap) return base;
  return `${base.slice(0, cap - 3)}…`;
}

/** Projects an optional error classification without raw tool payload. */
export function anthropicToolResultAttributes(params: {
  errorCode?: string;
}): Record<string, ScalarAttribute> {
  const attrs: Record<string, ScalarAttribute> = {};
  if (params.errorCode) {
    attrs.error_code = params.errorCode;
  }
  return attrs;
}
