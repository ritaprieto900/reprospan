# Privacy model

Reprospan treats replay bundles as sensitive debugging artifacts even when they contain only synthetic or metadata-only events.

## Defaults

- Capture mode is `metadata_only`.
- Exported bundles declare whether redaction was applied.
- Credentials, authorization headers, environment variables, prompt bodies, and tool payload bodies are never part of the canonical event shape.
- Optional content is represented by a content-addressed artifact reference containing a SHA-256 digest, media type, and byte length.
- Artifact storage is outside the current tracer bullet; a reference does not imply that bytes are available.
- Export serializes only the validated canonical bundle. It neither resolves artifact references nor adds captured content.
- The TypeScript capture API accepts explicit metadata and bounded summaries, not prompt text, model bodies, tool arguments/results, transport headers, environment variables, or credentials.
- A summary is caller-supplied safe text. The SDK never derives it by truncating a sensitive body.
- Server URLs and HTTP configuration belong to the transport client, not to canonical events.

## Boundary

Redaction must happen before persistence or export. Reprospan does not make a captured secret safe merely by hashing or packaging it.
