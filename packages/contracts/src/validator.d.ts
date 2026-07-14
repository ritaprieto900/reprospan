import type { ErrorObject } from "ajv";

export declare class ContractValidationError extends Error {
  readonly errors: ErrorObject[];
  constructor(message: string, errors?: ErrorObject[]);
}

export declare function createContractsAjv(): import("ajv").default;
export declare function assertValidBundle(document: unknown): void;
