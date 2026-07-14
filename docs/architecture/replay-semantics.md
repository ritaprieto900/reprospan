# Replay semantics

Reprospan uses replay terminology narrowly:

- **Playback** renders recorded events in their canonical sequence.
- **State reconstruction** rebuilds the recorded agent context without executing it.
- **Simulated replay** substitutes recorded or explicitly mocked model and tool outputs.
- **Active re-execution** invokes a live model or tool. It is outside the MVP and disabled by default.

A v1 patch performs one controlled `replace_recorded_output` operation against a recorded `model.response` or `tool.result`. It does not run the target again, delete history, or mutate the source bundle. The derived bundle keeps the same bundle identity because it is another simulated version of the same recorded timeline.

A v1 semantic diff compares only event status and complete recorded output for structurally aligned versions of one bundle. It rejects cross-bundle comparisons and event insertion, deletion, reordering, identity, or kind changes. A patch does not infer downstream execution: replacing a failed tool result does not automatically turn a recorded `run.failed` event into `run.completed`.

Evaluation consumes the derived timeline and uses deterministic assertions only.
