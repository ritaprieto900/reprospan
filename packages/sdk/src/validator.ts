import { assertValidBundle as assertContractBundle } from "@reprospan/contracts/validator";
import type { Bundle } from "./types.js";

export function assertValidBundle(value: unknown): asserts value is Bundle {
  assertContractBundle(value);
}
