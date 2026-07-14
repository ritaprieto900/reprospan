# Contributing

Reprospan is pre-alpha. The first milestone is a single offline workflow: failed run → replay bundle → one-step patch → semantic diff → deterministic evaluation.

Before opening a change:

1. keep it inside the active milestone;
2. add or update a fixture that demonstrates the behavior;
3. run the TypeScript and Rust checks;
4. document any privacy or replay-semantics impact.

Capture changes must keep the JSON Schema, shared fixtures, TypeScript validator, Rust `Bundle::validate`, and the loopback end-to-end flow aligned. Never accept raw secrets or content with the intention of hashing or redacting them later; excluded content must not enter the canonical capture path. Keep patch, diff, and evaluation on the offline CLI surface rather than adding HTTP routes.

Do not add live side-effecting replay paths without an explicit security design review.
