import { LoopbackClient } from "@reprospan/sdk";
import { runLocalAgent } from "./agent.js";

declare const process: { argv: string[] };

const baseUrl = process.argv[2];
const client = new LoopbackClient(baseUrl === undefined ? {} : { baseUrl });
const bundle = runLocalAgent();
const ingested = await client.ingest(bundle);

console.log(JSON.stringify({
  bundle_id: ingested.bundle_id,
  event_count: ingested.events.length,
  ingested: true,
}));
