import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import Ajv2020 from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const schemaFiles = [
  "schemas/v1/event.schema.json",
  "schemas/v1/bundle.schema.json",
  "schemas/v1/patch.schema.json",
  "schemas/v1/eval.schema.json",
  "schemas/v1/diff.schema.json",
];

function load(relativePath) {
  return JSON.parse(readFileSync(path.join(root, relativePath), "utf8"));
}

export function createContractsAjv() {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  addFormats(ajv);
  for (const schemaFile of schemaFiles) {
    ajv.addSchema(load(schemaFile));
  }
  return ajv;
}

const ajv = createContractsAjv();
const validateBundle = ajv.getSchema("https://reprospan.dev/schemas/v1/bundle.schema.json");

export class ContractValidationError extends Error {
  constructor(message, errors = []) {
    super(message);
    this.name = "ContractValidationError";
    this.errors = errors;
  }
}

export function assertValidBundle(document) {
  if (!validateBundle(document)) {
    const errors = validateBundle.errors ?? [];
    throw new ContractValidationError(
      ajv.errorsText(errors, { separator: "\n" }),
      errors,
    );
  }
}
