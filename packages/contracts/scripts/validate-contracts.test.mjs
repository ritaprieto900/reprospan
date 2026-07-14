import test from "node:test";
import assert from "node:assert/strict";
import { validateContracts } from "./validate-contracts.mjs";

test("v1 schemas accept the shared synthetic fixtures", async () => {
  assert.deepEqual(await validateContracts(), {
    schemas: 5,
    fixtures: 5,
    events: 10,
  });
});
