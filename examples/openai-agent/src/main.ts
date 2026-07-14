import OpenAI from "openai";
import { LoopbackClient } from "@reprospan/sdk";
import { runOpenAIAgent } from "./agent.js";
import { lookupPolicy } from "./local-tool.js";

const baseUrl = process.argv[2];
const apiKey = process.argv[3] || process.env.OPENAI_API_KEY;
if (!apiKey) {
  console.error("OPENAI_API_KEY env var or second argument required");
  process.exit(1);
}
const client = new LoopbackClient(baseUrl === undefined ? {} : { baseUrl });
const openai = new OpenAI({ apiKey });

const bundle = await runOpenAIAgent(
  baseUrl ?? "http://127.0.0.1:8787",
  openai as unknown as Parameters<typeof runOpenAIAgent>[1],
  async (name, input) => JSON.stringify(lookupPolicy(String(input.policy_key ?? ""))),
);

const ingested = await client.ingest(bundle);

console.log(JSON.stringify({
  bundle_id: ingested.bundle_id,
  event_count: ingested.events.length,
  ingested: true,
}));
