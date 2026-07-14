import { assertValidBundle } from "./validator.js";
import type {
  Attributes,
  Bundle,
  Event,
  EventKind,
  EventStatus,
  OutputResult,
  RecordedOutput,
} from "./types.js";

const identifier = /^[a-zA-Z0-9][a-zA-Z0-9._:-]{0,127}$/;

type Clock = () => string;
type EventIdGenerator = () => string;

export interface CaptureSessionOptions {
  bundleId: string;
  runId: string;
  runName: string;
  attributes?: Attributes;
  now?: Clock;
  nextEventId?: EventIdGenerator;
}

interface ParentInput {
  parentEventId: string;
}

export interface ModelRequestInput extends ParentInput {
  provider: string;
  model: string;
  operation: string;
}

export interface ModelResponseInput extends ParentInput {
  provider: string;
  model: string;
  operation: string;
  result: OutputResult;
  summary: string;
}

export interface ToolCallInput extends ParentInput {
  name: string;
  inputShape: string;
}

export interface ToolResultInput extends ParentInput {
  name: string;
  result: OutputResult;
  summary: string;
  errorCode?: string;
}

export interface RunTerminalInput extends ParentInput {
  reason?: string;
}

export class CaptureSession {
  readonly startedEventId: string;

  #bundleId: string;
  #runId: string;
  #runName: string;
  #createdAt: string;
  #events: Event[] = [];
  #eventIds = new Set<string>();
  #now: Clock;
  #nextEventId: EventIdGenerator;
  #terminal = false;

  constructor(options: CaptureSessionOptions) {
    assertIdentifier(options.bundleId, "bundleId");
    assertIdentifier(options.runId, "runId");
    assertNonEmpty(options.runName, "runName");

    this.#bundleId = options.bundleId;
    this.#runId = options.runId;
    this.#runName = options.runName;
    this.#now = options.now ?? (() => new Date().toISOString());
    this.#nextEventId = options.nextEventId ?? (() => `evt_${crypto.randomUUID()}`);
    this.#createdAt = this.#timestamp();
    this.startedEventId = this.#append(
      "run.started",
      "ok",
      options.runName,
      options.attributes ?? {},
    );
  }

  recordModelRequest(input: ModelRequestInput): string {
    return this.#append(
      "model.request",
      "ok",
      undefined,
      {
        provider: input.provider,
        model: input.model,
        operation: input.operation,
      },
      input.parentEventId,
    );
  }

  recordModelResponse(input: ModelResponseInput): string {
    return this.#append(
      "model.response",
      statusFor(input.result),
      undefined,
      {
        provider: input.provider,
        model: input.model,
        operation: input.operation,
      },
      input.parentEventId,
      outputFor(input.result, input.summary),
    );
  }

  recordToolCall(input: ToolCallInput): string {
    return this.#append(
      "tool.call",
      "ok",
      input.name,
      { input_shape: input.inputShape },
      input.parentEventId,
    );
  }

  recordToolResult(input: ToolResultInput): string {
    const attributes: Attributes = {};
    if (input.errorCode !== undefined) {
      assertNonEmpty(input.errorCode, "errorCode");
      attributes.error_code = input.errorCode;
    }
    return this.#append(
      "tool.result",
      statusFor(input.result),
      input.name,
      attributes,
      input.parentEventId,
      outputFor(input.result, input.summary),
    );
  }

  complete(input: RunTerminalInput): string {
    return this.#terminate("run.completed", "ok", input);
  }

  fail(input: RunTerminalInput): string {
    return this.#terminate("run.failed", "error", input);
  }

  finalize(): Bundle {
    if (!this.#terminal) {
      throw new Error("capture session must be completed or failed before finalize");
    }
    const bundle: Bundle = {
      schema_version: "reprospan.bundle.v1",
      bundle_id: this.#bundleId,
      created_at: this.#createdAt,
      capture_policy: { mode: "metadata_only", redacted: true },
      events: this.#events.map((event) => structuredClone(event)),
    };
    assertValidBundle(bundle);
    return bundle;
  }

  #terminate(kind: "run.completed" | "run.failed", status: EventStatus, input: RunTerminalInput): string {
    const attributes: Attributes = {};
    if (input.reason !== undefined) {
      assertNonEmpty(input.reason, "reason");
      attributes[kind === "run.failed" ? "failure_reason" : "completion_reason"] = input.reason;
    }
    const eventId = this.#append(
      kind,
      status,
      this.#runName,
      attributes,
      input.parentEventId,
    );
    this.#terminal = true;
    return eventId;
  }

  #append(
    kind: EventKind,
    status: EventStatus,
    name: string | undefined,
    attributes: Attributes,
    parentEventId?: string,
    output?: RecordedOutput,
  ): string {
    if (this.#terminal) {
      throw new Error("capture session is already terminal");
    }
    if (parentEventId !== undefined && !this.#eventIds.has(parentEventId)) {
      throw new Error(`parent event must already exist: ${parentEventId}`);
    }
    const eventId = this.#nextEventId();
    assertIdentifier(eventId, "eventId");
    if (this.#eventIds.has(eventId)) {
      throw new Error(`event id is duplicated: ${eventId}`);
    }
    if (name !== undefined) {
      assertNonEmpty(name, "name");
    }
    const event: Event = {
      schema_version: "reprospan.event.v1",
      event_id: eventId,
      run_id: this.#runId,
      sequence: this.#events.length,
      occurred_at: this.#timestamp(),
      kind,
      status,
      ...(parentEventId === undefined ? {} : { parent_event_id: parentEventId }),
      ...(name === undefined ? {} : { name }),
      attributes,
      ...(output === undefined ? {} : { output }),
    };
    this.#events.push(event);
    this.#eventIds.add(eventId);
    return eventId;
  }

  #timestamp(): string {
    const value = this.#now();
    if (Number.isNaN(Date.parse(value))) {
      throw new Error(`clock returned an invalid timestamp: ${value}`);
    }
    return value;
  }
}

function outputFor(result: OutputResult, summary: string): RecordedOutput {
  assertNonEmpty(summary, "summary");
  if (summary.length > 512) {
    throw new Error("summary must contain at most 512 characters");
  }
  return { result, summary };
}

function statusFor(result: OutputResult): EventStatus {
  return result === "ok" ? "ok" : "error";
}

function assertIdentifier(value: string, field: string): void {
  if (!identifier.test(value)) {
    throw new Error(`${field} is not a valid Reprospan identifier`);
  }
}

function assertNonEmpty(value: string, field: string): void {
  if (value.length === 0) {
    throw new Error(`${field} must not be empty`);
  }
}
