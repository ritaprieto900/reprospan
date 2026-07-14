# Bundle format v1

A replay bundle is a portable JSON document whose canonical events are the source of truth.

## Invariants

- `schema_version` is `reprospan.bundle.v1`.
- `bundle_id` is stable and is referenced by patches and evaluations.
- Every event uses `reprospan.event.v1`, belongs to one run, has a unique ID, and has a contiguous zero-based sequence.
- A parent ID must refer to an event at an earlier sequence; self and forward references are invalid.
- The capture SDK derives `status` from recorded output results so `ok` and `error` cannot disagree.
- The capture policy is explicit. The v1 fixture and tracer bullet accept metadata-only, redacted bundles.
- Large or sensitive content is never embedded in canonical events. Optional artifact references carry digest and shape metadata only.

JSON Schema validates document shape. `reprospan-core` additionally enforces identity, ordering, parent, patch-target, and evaluation invariants.

The local Agent capture example records `run.started → model.request → model.response → tool.call → tool.result → run.failed`. Its `model.*` metadata describes a deterministic local decision driver and does not imply a live provider call.

## Import and export

New imports persist the complete validated canonical bundle alongside the event projection. Export is a semantic round trip of that canonical object: deserializing an exported document produces the same `Bundle`, including capture policy and artifact references. It does not promise byte-for-byte preservation of whitespace or JSON key order.

Legacy records created before complete bundle storage remain readable as timelines but are not exportable, because reconstructing them would silently lose top-level metadata. Artifact references are exported; artifact bytes are not stored or embedded by this milestone.
