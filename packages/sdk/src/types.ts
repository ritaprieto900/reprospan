export type ScalarAttribute = string | number | boolean | null;
export type Attributes = Record<string, ScalarAttribute>;

export type EventKind =
  | "run.started"
  | "run.completed"
  | "run.failed"
  | "model.request"
  | "model.response"
  | "tool.call"
  | "tool.result";

export type EventStatus = "pending" | "ok" | "error";
export type OutputResult = "ok" | "error";

export interface ArtifactRef {
  sha256: string;
  media_type: string;
  byte_length: number;
}

export interface RecordedOutput {
  result: OutputResult;
  summary: string;
  artifact_ref?: ArtifactRef;
}

export interface Event {
  schema_version: "reprospan.event.v1";
  event_id: string;
  run_id: string;
  parent_event_id?: string;
  sequence: number;
  occurred_at: string;
  kind: EventKind;
  status: EventStatus;
  name?: string;
  attributes: Attributes;
  output?: RecordedOutput;
  artifact_refs?: ArtifactRef[];
}

export interface Bundle {
  schema_version: "reprospan.bundle.v1";
  bundle_id: string;
  created_at: string;
  capture_policy: {
    mode: "metadata_only";
    redacted: true;
  };
  events: Event[];
  artifacts?: ArtifactRef[];
}

export interface Health {
  status: string;
  api_version: string;
  contract_version: string;
}
