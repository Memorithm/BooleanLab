//! BL-14.5 immutable observation envelope for matched model evaluation.
//!
//! `sparsity_matched_evaluation` freezes which exact masks must be evaluated.
//! This module binds one observation to every frozen arm without ranking the
//! arms, selecting from HOLDOUT evidence, or fabricating unavailable hardware
//! measurements.  Reference-model accounting and measured hardware evidence
//! remain distinct evidence kinds.

use std::collections::BTreeSet;
use std::fmt;

use crate::sparsity_matched_evaluation::MatchedEvaluationArmSet;
use crate::sparsity_rule_evidence::ResourceEvidenceKind;

/// Version of the BL-14 matched observation contract.
pub const MATCHED_OBSERVATION_CONTRACT_VERSION: &str = "bl14.matched-observation-set.v1";

/// Resource evidence retained for one evaluated arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedResourceObservation {
    /// Reference-model accounting or an actual measurement.
    pub kind: ResourceEvidenceKind,
    /// Frozen accounting/measurement protocol shared by every arm.
    pub protocol_id: String,
    /// Immutable provenance shared by every arm in the comparison.
    pub provenance_id: String,
    /// Numerical work actually counted by the declared reference/measurement surface.
    pub numerical_compute_units: u64,
    /// Boolean/mask tests counted separately from numerical work.
    pub mask_test_units: u64,
    /// Controller work counted separately from admitted numerical work.
    pub controller_cost_units: u64,
    /// Traffic only when the declared evidence source exposes it.
    pub memory_traffic_bytes: Option<u64>,
    /// End-to-end latency only for measured hardware evidence.
    pub latency_ns: Option<u64>,
}

/// One model-quality/resource observation bound to a frozen arm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedArmObservation {
    /// Exact arm identity from [`MatchedEvaluationArmSet`].
    pub arm_id: String,
    /// Stable identifier for the independently scored task-quality metric.
    pub quality_metric_id: String,
    /// Signed loss delta relative to the declared dense/reference convention.
    pub quality_loss_units: i64,
    /// Retained units observed for this exact mask.
    pub retained_units: usize,
    /// Resource evidence retained independently from quality.
    pub resources: MatchedResourceObservation,
}

/// Complete, immutable matched-arm observation set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedObservationSet {
    contract_version: &'static str,
    evaluation_id: String,
    workload_provenance: String,
    arm_protocol_id: String,
    observations: Vec<MatchedArmObservation>,
}

impl MatchedObservationSet {
    #[must_use]
    pub const fn contract_version(&self) -> &'static str {
        self.contract_version
    }

    #[must_use]
    pub fn evaluation_id(&self) -> &str {
        &self.evaluation_id
    }

    #[must_use]
    pub fn workload_provenance(&self) -> &str {
        &self.workload_provenance
    }

    #[must_use]
    pub fn arm_protocol_id(&self) -> &str {
        &self.arm_protocol_id
    }

    #[must_use]
    pub fn observations(&self) -> &[MatchedArmObservation] {
        &self.observations
    }
}

/// Fail-closed validation errors for matched model-level observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchedObservationError {
    EmptyEvaluationId,
    EmptyWorkloadProvenance,
    ObservationCountMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyArmId {
        index: usize,
    },
    DuplicateArmId {
        arm_id: String,
    },
    ArmOrderOrIdentityMismatch {
        index: usize,
        expected: String,
        actual: String,
    },
    EmptyQualityMetricId {
        arm_id: String,
    },
    QualityMetricMismatch {
        expected: String,
        actual: String,
        arm_id: String,
    },
    RetainedCardinalityMismatch {
        arm_id: String,
        expected: usize,
        actual: usize,
    },
    EmptyResourceProtocolId {
        arm_id: String,
    },
    EmptyResourceProvenanceId {
        arm_id: String,
    },
    ResourceKindMismatch {
        expected: ResourceEvidenceKind,
        actual: ResourceEvidenceKind,
        arm_id: String,
    },
    ResourceProtocolMismatch {
        expected: String,
        actual: String,
        arm_id: String,
    },
    ResourceProvenanceMismatch {
        expected: String,
        actual: String,
        arm_id: String,
    },
    ReferenceModelLatencyForbidden {
        arm_id: String,
    },
    MeasuredLatencyRequired {
        arm_id: String,
    },
}

impl fmt::Display for MatchedObservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MatchedObservationError {}

/// Bind one independently produced observation to every frozen model-evaluation arm.
///
/// The observation order must exactly equal the canonical arm order.  This makes
/// dropping a poor arm, appending a convenient comparator, or relabelling one
/// result fail closed.  All arms must share one quality metric and one resource
/// evidence contract.  Reference-model accounting cannot carry latency; measured
/// hardware evidence must carry it.  Optional traffic stays unavailable instead
/// of being converted to zero.
///
/// This function does not rank arms or authorize promotion.
///
/// # Errors
///
/// Returns [`MatchedObservationError`] for incomplete/reordered arm coverage,
/// cardinality drift, mixed quality/resource provenance, or contradictory
/// reference-versus-measured evidence.
#[allow(clippy::too_many_lines)]
pub fn bind_matched_observations(
    arms: &MatchedEvaluationArmSet,
    evaluation_id: impl Into<String>,
    workload_provenance: impl Into<String>,
    observations: Vec<MatchedArmObservation>,
) -> Result<MatchedObservationSet, MatchedObservationError> {
    let evaluation_id = evaluation_id.into();
    if evaluation_id.trim().is_empty() {
        return Err(MatchedObservationError::EmptyEvaluationId);
    }
    let workload_provenance = workload_provenance.into();
    if workload_provenance.trim().is_empty() {
        return Err(MatchedObservationError::EmptyWorkloadProvenance);
    }
    if observations.len() != arms.arms().len() {
        return Err(MatchedObservationError::ObservationCountMismatch {
            expected: arms.arms().len(),
            actual: observations.len(),
        });
    }

    let mut ids = BTreeSet::new();
    let mut expected_quality_metric: Option<&str> = None;
    let mut expected_resource_kind: Option<ResourceEvidenceKind> = None;
    let mut expected_resource_protocol: Option<&str> = None;
    let mut expected_resource_provenance: Option<&str> = None;

    for (index, (arm, observation)) in arms.arms().iter().zip(&observations).enumerate() {
        if observation.arm_id.is_empty() {
            return Err(MatchedObservationError::EmptyArmId { index });
        }
        if !ids.insert(observation.arm_id.as_str()) {
            return Err(MatchedObservationError::DuplicateArmId {
                arm_id: observation.arm_id.clone(),
            });
        }
        if observation.arm_id != arm.arm_id() {
            return Err(MatchedObservationError::ArmOrderOrIdentityMismatch {
                index,
                expected: arm.arm_id().to_owned(),
                actual: observation.arm_id.clone(),
            });
        }
        if observation.quality_metric_id.trim().is_empty() {
            return Err(MatchedObservationError::EmptyQualityMetricId {
                arm_id: observation.arm_id.clone(),
            });
        }
        if let Some(expected) = expected_quality_metric {
            if observation.quality_metric_id != expected {
                return Err(MatchedObservationError::QualityMetricMismatch {
                    expected: expected.to_owned(),
                    actual: observation.quality_metric_id.clone(),
                    arm_id: observation.arm_id.clone(),
                });
            }
        } else {
            expected_quality_metric = Some(observation.quality_metric_id.as_str());
        }

        let expected_retained = arm.mask().cardinality().retained();
        if observation.retained_units != expected_retained {
            return Err(MatchedObservationError::RetainedCardinalityMismatch {
                arm_id: observation.arm_id.clone(),
                expected: expected_retained,
                actual: observation.retained_units,
            });
        }

        let resource = &observation.resources;
        if resource.protocol_id.trim().is_empty() {
            return Err(MatchedObservationError::EmptyResourceProtocolId {
                arm_id: observation.arm_id.clone(),
            });
        }
        if resource.provenance_id.trim().is_empty() {
            return Err(MatchedObservationError::EmptyResourceProvenanceId {
                arm_id: observation.arm_id.clone(),
            });
        }
        if resource.kind == ResourceEvidenceKind::ReferenceModel && resource.latency_ns.is_some() {
            return Err(MatchedObservationError::ReferenceModelLatencyForbidden {
                arm_id: observation.arm_id.clone(),
            });
        }
        if resource.kind == ResourceEvidenceKind::Measured && resource.latency_ns.is_none() {
            return Err(MatchedObservationError::MeasuredLatencyRequired {
                arm_id: observation.arm_id.clone(),
            });
        }

        if let Some(expected) = expected_resource_kind {
            if resource.kind != expected {
                return Err(MatchedObservationError::ResourceKindMismatch {
                    expected,
                    actual: resource.kind,
                    arm_id: observation.arm_id.clone(),
                });
            }
        } else {
            expected_resource_kind = Some(resource.kind);
        }
        if let Some(expected) = expected_resource_protocol {
            if resource.protocol_id != expected {
                return Err(MatchedObservationError::ResourceProtocolMismatch {
                    expected: expected.to_owned(),
                    actual: resource.protocol_id.clone(),
                    arm_id: observation.arm_id.clone(),
                });
            }
        } else {
            expected_resource_protocol = Some(resource.protocol_id.as_str());
        }
        if let Some(expected) = expected_resource_provenance {
            if resource.provenance_id != expected {
                return Err(MatchedObservationError::ResourceProvenanceMismatch {
                    expected: expected.to_owned(),
                    actual: resource.provenance_id.clone(),
                    arm_id: observation.arm_id.clone(),
                });
            }
        } else {
            expected_resource_provenance = Some(resource.provenance_id.as_str());
        }
    }

    Ok(MatchedObservationSet {
        contract_version: MATCHED_OBSERVATION_CONTRACT_VERSION,
        evaluation_id,
        workload_provenance,
        arm_protocol_id: arms.protocol_id().to_owned(),
        observations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use booleanlab_core::ExactMask;

    use crate::sparsity_matched_baselines::{MatchedBaselineProtocol, build_matched_baseline_set};
    use crate::sparsity_matched_evaluation::build_matched_evaluation_arm_set;

    fn arm_set() -> MatchedEvaluationArmSet {
        let candidate = ExactMask::from_retained_indices(8, &[0, 2, 5, 7]).unwrap();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];
        let baselines = build_matched_baseline_set(
            &candidate,
            &scores,
            &scores,
            MatchedBaselineProtocol {
                protocol_id: "bl14.5-model-eval-v1".to_owned(),
                boolean_candidate_provenance: "boolean-mask:sha256:candidate".to_owned(),
                magnitude_score_provenance: "weights:sha256:a".to_owned(),
                structured_score_provenance: "weights:sha256:a".to_owned(),
                random_seed_provenance: "preregistered-seeds-v1".to_owned(),
                structured_group_size: 4,
                random_seeds: vec![7, 11],
            },
        )
        .unwrap();
        build_matched_evaluation_arm_set(&candidate, &baselines).unwrap()
    }

    fn observations(
        arms: &MatchedEvaluationArmSet,
        kind: ResourceEvidenceKind,
    ) -> Vec<MatchedArmObservation> {
        arms.arms()
            .iter()
            .enumerate()
            .map(|(index, arm)| MatchedArmObservation {
                arm_id: arm.arm_id().to_owned(),
                quality_metric_id: "task-mse-scaled-1e12".to_owned(),
                quality_loss_units: i64::try_from(index).expect("fixture arm index fits i64") - 2,
                retained_units: arm.mask().cardinality().retained(),
                resources: MatchedResourceObservation {
                    kind,
                    protocol_id: "reference-work-v1".to_owned(),
                    provenance_id: "workload:sha256:abc".to_owned(),
                    numerical_compute_units: 100 + index as u64,
                    mask_test_units: if arm.arm_id() == "dense" { 0 } else { 8 },
                    controller_cost_units: if arm.arm_id() == "boolean" { 4 } else { 0 },
                    memory_traffic_bytes: None,
                    latency_ns: (kind == ResourceEvidenceKind::Measured)
                        .then_some(1_000 + index as u64),
                },
            })
            .collect()
    }

    #[test]
    fn binds_complete_reference_observations_without_inventing_latency_or_traffic() {
        let arms = arm_set();
        let bound = bind_matched_observations(
            &arms,
            "dev-eval-001",
            "relu-workload:sha256:123",
            observations(&arms, ResourceEvidenceKind::ReferenceModel),
        )
        .unwrap();
        assert_eq!(
            bound.contract_version(),
            MATCHED_OBSERVATION_CONTRACT_VERSION
        );
        assert_eq!(bound.arm_protocol_id(), "bl14.5-model-eval-v1");
        assert_eq!(bound.observations().len(), arms.arms().len());
        assert!(
            bound
                .observations()
                .iter()
                .all(|item| item.resources.latency_ns.is_none())
        );
        assert!(
            bound
                .observations()
                .iter()
                .all(|item| item.resources.memory_traffic_bytes.is_none())
        );
    }

    #[test]
    fn measured_evidence_requires_latency_and_keeps_one_provenance() {
        let arms = arm_set();
        let bound = bind_matched_observations(
            &arms,
            "measured-eval-001",
            "relu-workload:sha256:123",
            observations(&arms, ResourceEvidenceKind::Measured),
        )
        .unwrap();
        assert!(
            bound
                .observations()
                .iter()
                .all(|item| item.resources.latency_ns.is_some())
        );
    }

    #[test]
    fn missing_reordered_or_duplicate_arms_fail_closed() {
        let arms = arm_set();
        let base = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        let mut missing = base.clone();
        missing.pop();
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", missing),
            Err(MatchedObservationError::ObservationCountMismatch { .. })
        ));

        let mut reordered = base.clone();
        reordered.swap(0, 1);
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", reordered),
            Err(MatchedObservationError::ArmOrderOrIdentityMismatch { .. })
        ));

        let mut duplicate = base;
        duplicate[1].arm_id = duplicate[0].arm_id.clone();
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", duplicate),
            Err(MatchedObservationError::DuplicateArmId { .. }
                | MatchedObservationError::ArmOrderOrIdentityMismatch { .. })
        ));
    }

    #[test]
    fn cardinality_quality_and_resource_drift_fail_closed() {
        let arms = arm_set();

        let mut rows = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        rows[1].retained_units += 1;
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", rows),
            Err(MatchedObservationError::RetainedCardinalityMismatch { .. })
        ));

        let mut rows = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        rows[1].quality_metric_id = "different".to_owned();
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", rows),
            Err(MatchedObservationError::QualityMetricMismatch { .. })
        ));

        let mut rows = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        rows[1].resources.provenance_id = "other".to_owned();
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", rows),
            Err(MatchedObservationError::ResourceProvenanceMismatch { .. })
        ));
    }

    #[test]
    fn reference_latency_and_missing_measured_latency_are_rejected() {
        let arms = arm_set();
        let mut reference = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        reference[0].resources.latency_ns = Some(1);
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", reference),
            Err(MatchedObservationError::ReferenceModelLatencyForbidden { .. })
        ));

        let mut measured = observations(&arms, ResourceEvidenceKind::Measured);
        measured[0].resources.latency_ns = None;
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", measured),
            Err(MatchedObservationError::MeasuredLatencyRequired { .. })
        ));
    }

    #[test]
    fn empty_identity_and_mixed_resource_kinds_fail_closed() {
        let arms = arm_set();
        assert!(matches!(
            bind_matched_observations(
                &arms,
                " ",
                "w",
                observations(&arms, ResourceEvidenceKind::ReferenceModel),
            ),
            Err(MatchedObservationError::EmptyEvaluationId)
        ));

        let mut rows = observations(&arms, ResourceEvidenceKind::ReferenceModel);
        rows[1].resources.kind = ResourceEvidenceKind::Measured;
        rows[1].resources.latency_ns = Some(1000);
        assert!(matches!(
            bind_matched_observations(&arms, "e", "w", rows),
            Err(MatchedObservationError::ResourceKindMismatch { .. })
        ));
    }
}
