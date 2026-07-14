import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { createContractsAjv } from "../src/validator.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const files = {
  event: "schemas/v1/event.schema.json",
  bundle: "schemas/v1/bundle.schema.json",
  patch: "schemas/v1/patch.schema.json",
  evaluation: "schemas/v1/eval.schema.json",
  diff: "schemas/v1/diff.schema.json",
  bundleFixture: "fixtures/v1/failed-tool-run.bundle.json",
  localAgentBundleFixture: "fixtures/v1/local-agent-tool-failure.bundle.json",
  patchFixture: "fixtures/v1/fix-tool-result.patch.json",
  evalFixture: "fixtures/v1/fix-tool-result.eval.json",
  diffFixture: "fixtures/v1/fix-tool-result.diff.json",
};

async function load(relativePath) {
  return JSON.parse(await readFile(path.join(root, relativePath), "utf8"));
}

export async function validateContracts() {
  const documents = Object.fromEntries(
    await Promise.all(Object.entries(files).map(async ([name, file]) => [name, await load(file)])),
  );

  const ajv = createContractsAjv();

  const checks = [
    [documents.bundle.$id, documents.bundleFixture, files.bundleFixture],
    [documents.bundle.$id, documents.localAgentBundleFixture, files.localAgentBundleFixture],
    [documents.patch.$id, documents.patchFixture, files.patchFixture],
    [documents.evaluation.$id, documents.evalFixture, files.evalFixture],
    [documents.diff.$id, documents.diffFixture, files.diffFixture],
  ];

  for (const [schemaId, fixture, fixturePath] of checks) {
    const validate = ajv.getSchema(schemaId);
    if (!validate(fixture)) {
      throw new Error(`${fixturePath}: ${ajv.errorsText(validate.errors, { separator: "\n" })}`);
    }
  }

  if (documents.patchFixture.base_bundle_id !== documents.bundleFixture.bundle_id) {
    throw new Error("patch fixture targets a different bundle");
  }
  if (documents.evalFixture.base_bundle_id !== documents.bundleFixture.bundle_id) {
    throw new Error("eval fixture targets a different bundle");
  }
  if (!documents.bundleFixture.events.some(({ event_id }) => event_id === documents.patchFixture.target_event_id)) {
    throw new Error("patch fixture targets an unknown event");
  }
  if (documents.diffFixture.base_bundle_id !== documents.bundleFixture.bundle_id) {
    throw new Error("diff fixture targets a different bundle");
  }
  if (
    documents.diffFixture.changed_events.length !== 1 ||
    documents.diffFixture.changed_events[0].event_id !== documents.patchFixture.target_event_id
  ) {
    throw new Error("diff fixture does not describe the patch target");
  }
  if (
    documents.diffFixture.changed_events[0].output.after.result !== documents.patchFixture.replacement.result ||
    documents.diffFixture.changed_events[0].output.after.summary !== documents.patchFixture.replacement.summary
  ) {
    throw new Error("diff fixture does not contain the patch replacement");
  }

  return {
    schemas: 5,
    fixtures: 5,
    events: documents.bundleFixture.events.length + documents.localAgentBundleFixture.events.length,
  };
}

if (fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const result = await validateContracts();
  console.log(`validated ${result.schemas} schemas, ${result.fixtures} fixtures, ${result.events} events`);
}
