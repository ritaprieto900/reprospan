import { assertValidBundle } from "./validator.js";
import type { Bundle, Health } from "./types.js";

const defaultBaseUrl = "http://127.0.0.1:8787";

interface ErrorBody {
  code?: unknown;
  message?: unknown;
}

export interface LoopbackClientOptions {
  baseUrl?: string;
  fetch?: typeof globalThis.fetch;
}

export class ReprospanHttpError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ReprospanHttpError";
    this.status = status;
    this.code = code;
  }
}

export class LoopbackClient {
  #baseUrl: URL;
  #fetch: typeof globalThis.fetch;

  constructor(options: LoopbackClientOptions = {}) {
    this.#baseUrl = parseLoopbackUrl(options.baseUrl ?? defaultBaseUrl);
    this.#fetch = options.fetch ?? globalThis.fetch;
  }

  async health(): Promise<Health> {
    return this.#request<Health>("/healthz");
  }

  async ingest(bundle: Bundle): Promise<Bundle> {
    assertValidBundle(bundle);
    const result = await this.#request<unknown>("/v1/bundles/ingest", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(bundle),
    });
    assertValidBundle(result);
    return result;
  }

  async timeline(bundleId: string): Promise<Bundle> {
    const result = await this.#request<unknown>(
      `/v1/bundles/${encodeURIComponent(bundleId)}/timeline`,
    );
    assertValidBundle(result);
    return result;
  }

  async storeArtifact(sha256: string, mediaType: string, bytes: Uint8Array): Promise<void> {
    const response = await this.#fetch(
      new URL(`/v1/artifacts/${encodeURIComponent(sha256)}`, this.#baseUrl),
      { method: "PUT", headers: { "content-type": mediaType }, body: bytes },
    );
    if (!response.ok) {
      let code = "http_error";
      let message = response.statusText;
      try {
        const body = await response.json();
        if (typeof body?.code === "string") code = body.code;
        if (typeof body?.message === "string") message = body.message;
      } catch { /* keep defaults */ }
      throw new ReprospanHttpError(response.status, code, message.slice(0, 512));
    }
  }

  async #request<T>(pathname: string, init?: RequestInit): Promise<T> {
    const response = await this.#fetch(new URL(pathname, this.#baseUrl), init);
    const body: unknown = await response.json().catch(() => undefined);
    if (!response.ok) {
      const error = isRecord(body) ? (body as ErrorBody) : {};
      const code = typeof error.code === "string" ? error.code : "http_error";
      const message = typeof error.message === "string" ? error.message : response.statusText;
      throw new ReprospanHttpError(response.status, code, message.slice(0, 512));
    }
    return body as T;
  }
}

function parseLoopbackUrl(value: string): URL {
  const url = new URL(value);
  if (url.protocol !== "http:" || (url.hostname !== "127.0.0.1" && url.hostname !== "[::1]")) {
    throw new Error("Reprospan base URL must be an HTTP loopback address");
  }
  if (url.username !== "" || url.password !== "" || url.search !== "" || url.hash !== "") {
    throw new Error("Reprospan base URL must not contain credentials, query, or fragment");
  }
  url.pathname = url.pathname.replace(/\/?$/, "/");
  return url;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
