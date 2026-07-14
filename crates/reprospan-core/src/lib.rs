use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub schema_version: String,
    pub bundle_id: String,
    pub created_at: String,
    pub capture_policy: CapturePolicy,
    pub events: Vec<Event>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CapturePolicy {
    pub mode: String,
    pub redacted: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub schema_version: String,
    pub event_id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_event_id: Option<String>,
    pub sequence: u64,
    pub occurred_at: String,
    pub kind: EventKind,
    pub status: EventStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub attributes: serde_json::Map<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<RecordedOutput>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<ArtifactRef>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EventKind {
    #[serde(rename = "run.started")]
    RunStarted,
    #[serde(rename = "run.completed")]
    RunCompleted,
    #[serde(rename = "run.failed")]
    RunFailed,
    #[serde(rename = "model.request")]
    ModelRequest,
    #[serde(rename = "model.response")]
    ModelResponse,
    #[serde(rename = "tool.call")]
    ToolCall,
    #[serde(rename = "tool.result")]
    ToolResult,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventStatus {
    Pending,
    Ok,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RecordedOutput {
    pub result: OutputResult,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_ref: Option<ArtifactRef>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputResult {
    Ok,
    Error,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub sha256: String,
    pub media_type: String,
    pub byte_length: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub schema_version: String,
    pub patch_id: String,
    pub base_bundle_id: String,
    pub target_event_id: String,
    pub operation: PatchOperation,
    pub replacement: RecordedOutput,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum PatchOperation {
    #[serde(rename = "replace_recorded_output")]
    ReplaceRecordedOutput,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub schema_version: String,
    pub eval_id: String,
    pub base_bundle_id: String,
    pub assertions: Vec<Assertion>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Assertion {
    EventExists {
        assertion_id: String,
        event_id: String,
    },
    EventStatusEquals {
        assertion_id: String,
        event_id: String,
        expected: EventStatus,
    },
    OutputResultEquals {
        assertion_id: String,
        event_id: String,
        expected: OutputResult,
    },
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct EvaluationResult {
    pub eval_id: String,
    pub passed: bool,
    pub assertions: Vec<AssertionResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AssertionResult {
    pub assertion_id: String,
    pub passed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct SemanticDiff {
    pub schema_version: String,
    pub base_bundle_id: String,
    pub changed_events: Vec<EventDiff>,
    pub unchanged_event_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct EventDiff {
    pub event_id: String,
    pub kind: EventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ValueChange<EventStatus>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<ValueChange<Option<RecordedOutput>>>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ValueChange<T> {
    pub before: T,
    pub after: T,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("unsupported schema version: {0}")]
    UnsupportedVersion(String),
    #[error("bundle must contain at least one event")]
    EmptyBundle,
    #[error("event id is duplicated: {0}")]
    DuplicateEventId(String),
    #[error("event {event_id} has sequence {actual}; expected {expected}")]
    InvalidSequence {
        event_id: String,
        expected: u64,
        actual: u64,
    },
    #[error("event {event_id} belongs to run {actual}; expected {expected}")]
    MixedRuns {
        event_id: String,
        expected: String,
        actual: String,
    },
    #[error("event {event_id} references unknown or later parent {parent_id}")]
    InvalidParent { event_id: String, parent_id: String },
    #[error("document targets bundle {actual}; expected {expected}")]
    BundleMismatch { expected: String, actual: String },
    #[error("target event was not found: {0}")]
    TargetNotFound(String),
    #[error("event cannot receive a recorded output patch: {0}")]
    InvalidPatchTarget(String),
    #[error("diff requires the same number of events; before has {before}, after has {after}")]
    DiffEventCountMismatch { before: usize, after: usize },
    #[error("diff event identity differs at sequence {sequence}: before {before}, after {after}")]
    DiffEventIdentityMismatch {
        sequence: u64,
        before: String,
        after: String,
    },
    #[error("diff event kind differs for {event_id}")]
    DiffEventKindMismatch { event_id: String },
    #[error("diff bundle metadata differs")]
    DiffBundleMetadataMismatch,
    #[error("diff immutable event fields differ for {event_id}")]
    DiffEventMetadataMismatch { event_id: String },
}

impl Bundle {
    /// Validates the version, event identities, ordering, run membership, and parents.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] when the bundle or any event violates a v1 invariant.
    pub fn validate(&self) -> Result<(), CoreError> {
        if self.schema_version != "reprospan.bundle.v1" {
            return Err(CoreError::UnsupportedVersion(self.schema_version.clone()));
        }
        if self.events.is_empty() {
            return Err(CoreError::EmptyBundle);
        }

        let run_id = self.events[0].run_id.clone();
        let mut event_ids = HashSet::with_capacity(self.events.len());

        for (expected_sequence, event) in self.events.iter().enumerate() {
            if event.schema_version != "reprospan.event.v1" {
                return Err(CoreError::UnsupportedVersion(event.schema_version.clone()));
            }
            if event.sequence != expected_sequence as u64 {
                return Err(CoreError::InvalidSequence {
                    event_id: event.event_id.clone(),
                    expected: expected_sequence as u64,
                    actual: event.sequence,
                });
            }
            if event.run_id != run_id {
                return Err(CoreError::MixedRuns {
                    event_id: event.event_id.clone(),
                    expected: run_id.clone(),
                    actual: event.run_id.clone(),
                });
            }
            if event_ids.contains(&event.event_id) {
                return Err(CoreError::DuplicateEventId(event.event_id.clone()));
            }
            if let Some(parent_id) = &event.parent_event_id
                && !event_ids.contains(parent_id)
            {
                return Err(CoreError::InvalidParent {
                    event_id: event.event_id.clone(),
                    parent_id: parent_id.clone(),
                });
            }
            event_ids.insert(event.event_id.clone());
        }

        Ok(())
    }

    /// Produces a derived bundle with one recorded model or tool output replaced.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] when either document is invalid, the bundle identity differs,
    /// or the target event cannot receive a recorded output.
    pub fn apply_patch(&self, patch: &Patch) -> Result<Self, CoreError> {
        self.validate()?;
        if patch.schema_version != "reprospan.patch.v1" {
            return Err(CoreError::UnsupportedVersion(patch.schema_version.clone()));
        }
        if patch.base_bundle_id != self.bundle_id {
            return Err(CoreError::BundleMismatch {
                expected: self.bundle_id.clone(),
                actual: patch.base_bundle_id.clone(),
            });
        }

        let mut patched = self.clone();
        let event = patched
            .events
            .iter_mut()
            .find(|event| event.event_id == patch.target_event_id)
            .ok_or_else(|| CoreError::TargetNotFound(patch.target_event_id.clone()))?;

        if !matches!(event.kind, EventKind::ModelResponse | EventKind::ToolResult) {
            return Err(CoreError::InvalidPatchTarget(event.event_id.clone()));
        }

        event.output = Some(patch.replacement.clone());
        event.status = match patch.replacement.result {
            OutputResult::Ok => EventStatus::Ok,
            OutputResult::Error => EventStatus::Error,
        };
        Ok(patched)
    }

    /// Compares the mutable replay state of two versions of the same canonical timeline.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] when either bundle is invalid, identities differ, or the event
    /// collection is not structurally aligned.
    pub fn semantic_diff(&self, after: &Self) -> Result<SemanticDiff, CoreError> {
        self.validate()?;
        after.validate()?;
        if self.bundle_id != after.bundle_id {
            return Err(CoreError::BundleMismatch {
                expected: self.bundle_id.clone(),
                actual: after.bundle_id.clone(),
            });
        }
        if self.schema_version != after.schema_version
            || self.created_at != after.created_at
            || self.capture_policy != after.capture_policy
            || self.artifacts != after.artifacts
        {
            return Err(CoreError::DiffBundleMetadataMismatch);
        }
        if self.events.len() != after.events.len() {
            return Err(CoreError::DiffEventCountMismatch {
                before: self.events.len(),
                after: after.events.len(),
            });
        }

        let mut changed_events = Vec::new();
        for (before_event, after_event) in self.events.iter().zip(&after.events) {
            if before_event.event_id != after_event.event_id {
                return Err(CoreError::DiffEventIdentityMismatch {
                    sequence: before_event.sequence,
                    before: before_event.event_id.clone(),
                    after: after_event.event_id.clone(),
                });
            }
            if before_event.kind != after_event.kind {
                return Err(CoreError::DiffEventKindMismatch {
                    event_id: before_event.event_id.clone(),
                });
            }
            if before_event.schema_version != after_event.schema_version
                || before_event.run_id != after_event.run_id
                || before_event.parent_event_id != after_event.parent_event_id
                || before_event.sequence != after_event.sequence
                || before_event.occurred_at != after_event.occurred_at
                || before_event.name != after_event.name
                || before_event.attributes != after_event.attributes
                || before_event.artifact_refs != after_event.artifact_refs
            {
                return Err(CoreError::DiffEventMetadataMismatch {
                    event_id: before_event.event_id.clone(),
                });
            }

            let status = (before_event.status != after_event.status).then_some(ValueChange {
                before: before_event.status,
                after: after_event.status,
            });
            let output = (before_event.output != after_event.output).then(|| ValueChange {
                before: before_event.output.clone(),
                after: after_event.output.clone(),
            });
            if status.is_some() || output.is_some() {
                changed_events.push(EventDiff {
                    event_id: before_event.event_id.clone(),
                    kind: before_event.kind,
                    status,
                    output,
                });
            }
        }

        Ok(SemanticDiff {
            schema_version: "reprospan.diff.v1".to_owned(),
            base_bundle_id: self.bundle_id.clone(),
            unchanged_event_count: self.events.len() - changed_events.len(),
            changed_events,
        })
    }

    /// Runs deterministic assertions against this bundle.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError`] when the bundle or evaluation document is invalid or targets a
    /// different bundle.
    pub fn evaluate(&self, evaluation: &Evaluation) -> Result<EvaluationResult, CoreError> {
        self.validate()?;
        if evaluation.schema_version != "reprospan.eval.v1" {
            return Err(CoreError::UnsupportedVersion(
                evaluation.schema_version.clone(),
            ));
        }
        if evaluation.base_bundle_id != self.bundle_id {
            return Err(CoreError::BundleMismatch {
                expected: self.bundle_id.clone(),
                actual: evaluation.base_bundle_id.clone(),
            });
        }

        let assertions = evaluation
            .assertions
            .iter()
            .map(|assertion| {
                let (assertion_id, passed) = match assertion {
                    Assertion::EventExists {
                        assertion_id,
                        event_id,
                    } => (
                        assertion_id,
                        self.events.iter().any(|event| event.event_id == *event_id),
                    ),
                    Assertion::EventStatusEquals {
                        assertion_id,
                        event_id,
                        expected,
                    } => (
                        assertion_id,
                        self.events
                            .iter()
                            .find(|event| event.event_id == *event_id)
                            .is_some_and(|event| event.status == *expected),
                    ),
                    Assertion::OutputResultEquals {
                        assertion_id,
                        event_id,
                        expected,
                    } => (
                        assertion_id,
                        self.events
                            .iter()
                            .find(|event| event.event_id == *event_id)
                            .and_then(|event| event.output.as_ref())
                            .is_some_and(|output| output.result == *expected),
                    ),
                };
                AssertionResult {
                    assertion_id: assertion_id.clone(),
                    passed,
                }
            })
            .collect::<Vec<_>>();

        Ok(EvaluationResult {
            eval_id: evaluation.eval_id.clone(),
            passed: assertions.iter().all(|assertion| assertion.passed),
            assertions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BUNDLE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/failed-tool-run.bundle.json"
    ));
    const LOCAL_AGENT_BUNDLE: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/local-agent-tool-failure.bundle.json"
    ));
    const PATCH: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/fix-tool-result.patch.json"
    ));
    const EVALUATION: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/fix-tool-result.eval.json"
    ));
    const DIFF: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../packages/contracts/fixtures/v1/fix-tool-result.diff.json"
    ));

    fn fixture() -> Bundle {
        serde_json::from_str(BUNDLE).expect("bundle fixture should deserialize")
    }

    #[test]
    fn shared_fixture_patches_and_passes_evaluation() {
        let bundle = fixture();
        let patch: Patch = serde_json::from_str(PATCH).expect("patch fixture should deserialize");
        let evaluation: Evaluation =
            serde_json::from_str(EVALUATION).expect("eval fixture should deserialize");

        bundle.validate().expect("fixture should be valid");
        let original = bundle.clone();
        let patched = bundle.apply_patch(&patch).expect("patch should apply");
        let result = patched
            .evaluate(&evaluation)
            .expect("evaluation should run");

        assert_eq!(bundle, original);
        assert!(!bundle.evaluate(&evaluation).unwrap().passed);
        assert!(result.passed);
        assert!(result.assertions.iter().all(|assertion| assertion.passed));
    }

    #[test]
    fn shared_fixture_produces_expected_semantic_diff() {
        let bundle = fixture();
        let patch: Patch = serde_json::from_str(PATCH).expect("patch fixture should deserialize");
        let expected: SemanticDiff =
            serde_json::from_str(DIFF).expect("diff fixture should deserialize");
        let patched = bundle.apply_patch(&patch).expect("patch should apply");

        assert_eq!(bundle.semantic_diff(&patched).unwrap(), expected);
        assert_eq!(
            bundle.semantic_diff(&bundle).unwrap(),
            SemanticDiff {
                schema_version: "reprospan.diff.v1".to_owned(),
                base_bundle_id: bundle.bundle_id.clone(),
                changed_events: Vec::new(),
                unchanged_event_count: bundle.events.len(),
            }
        );
    }

    #[test]
    fn diff_rejects_other_bundle_or_event_identity() {
        let bundle = fixture();
        let mut other_bundle = bundle.clone();
        other_bundle.bundle_id = "another_bundle".to_owned();
        assert!(matches!(
            bundle.semantic_diff(&other_bundle),
            Err(CoreError::BundleMismatch { .. })
        ));

        let mut other_event = bundle.clone();
        other_event.events[3].event_id = "evt_replaced".to_owned();
        assert!(matches!(
            bundle.semantic_diff(&other_event),
            Err(CoreError::DiffEventIdentityMismatch { .. })
        ));

        let mut other_metadata = bundle.clone();
        other_metadata.events[1].name = Some("renamed_tool".to_owned());
        assert!(matches!(
            bundle.semantic_diff(&other_metadata),
            Err(CoreError::DiffEventMetadataMismatch { .. })
        ));
    }

    #[test]
    fn local_agent_fixture_is_valid() {
        let bundle: Bundle = serde_json::from_str(LOCAL_AGENT_BUNDLE)
            .expect("local agent bundle fixture should deserialize");
        bundle
            .validate()
            .expect("local agent fixture should be valid");
        assert_eq!(bundle.events.len(), 6);
    }

    #[test]
    fn self_and_future_parents_are_rejected() {
        let mut self_parent = fixture();
        self_parent.events[1].parent_event_id = Some(self_parent.events[1].event_id.clone());
        assert!(matches!(
            self_parent.validate(),
            Err(CoreError::InvalidParent { .. })
        ));

        let mut future_parent = fixture();
        future_parent.events[1].parent_event_id = Some(future_parent.events[2].event_id.clone());
        assert!(matches!(
            future_parent.validate(),
            Err(CoreError::InvalidParent { .. })
        ));
    }

    #[test]
    fn duplicate_event_id_is_rejected() {
        let mut bundle = fixture();
        bundle.events[1].event_id = bundle.events[0].event_id.clone();
        assert!(matches!(
            bundle.validate(),
            Err(CoreError::DuplicateEventId(_))
        ));
    }

    #[test]
    fn non_contiguous_sequence_is_rejected() {
        let mut bundle = fixture();
        bundle.events[1].sequence = 7;
        assert!(matches!(
            bundle.validate(),
            Err(CoreError::InvalidSequence { .. })
        ));
    }

    #[test]
    fn mixed_run_is_rejected() {
        let mut bundle = fixture();
        bundle.events[1].run_id = "another_run".to_owned();
        assert!(matches!(
            bundle.validate(),
            Err(CoreError::MixedRuns { .. })
        ));
    }

    #[test]
    fn patching_a_run_event_is_rejected() {
        let bundle = fixture();
        let mut patch: Patch =
            serde_json::from_str(PATCH).expect("patch fixture should deserialize");
        patch.target_event_id = "evt_001".to_owned();
        assert!(matches!(
            bundle.apply_patch(&patch),
            Err(CoreError::InvalidPatchTarget(_))
        ));
    }
}
