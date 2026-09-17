//! BL-14.5 immutable arm manifest for matched model-level evaluation.
//!
//! A model-level sparsity experiment needs the dense reference plus the exact
//! Boolean, magnitude, structured, and deterministic-random masks that will be
//! scored.  This module materializes that arm set once from the already-frozen
//! matched-baseline contract.  It does not execute a model, choose a winner,
//! inspect HOLDOUT outcomes, or turn logical mask density into hardware savings.

use std::fmt;

use booleanlab_core::{ExactMask, MaskCardinality, SparsityError};

use crate::sparsity_matched_baselines::MatchedBaselineSet;

/// Version of the BL-14 matched model-evaluation arm contract.
pub const MATCHED_EVALUATION_ARMS_CONTRACT_VERSION: &str = "bl14.matched-evaluation-arms.v1";

/// Stable role of one arm in the frozen evaluation set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchedEvaluationArmKind {
    /// Fully active numerical reference. It is intentionally not density matched.
    Dense,
    /// Frozen Boolean candidate mask.
    Boolean,
    /// Descending-score control at matched retained cardinality.
    Magnitude,
    /// N:M structured control at matched retained cardinality.
    Structured,
    /// Deterministic random control for one preregistered seed.
    Random { seed: u64 },
}

/// One immutable exact mask and its stable evaluation identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedEvaluationArm {
    arm_id: String,
    kind: MatchedEvaluationArmKind,
    mask: ExactMask,
}

impl MatchedEvaluationArm {
    #[must_use]
    pub fn arm_id(&self) -> &str {
        &self.arm_id
    }

    #[must_use]
    pub const fn kind(&self) -> MatchedEvaluationArmKind {
        self.kind
    }

    #[must_use]
    pub const fn mask(&self) -> &ExactMask {
        &self.mask
    }
}

/// Frozen dense + density-matched arm set for one Boolean candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedEvaluationArmSet {
    contract_version: &'static str,
    protocol_id: String,
    boolean_candidate_provenance: String,
    matched_cardinality: MaskCardinality,
    arms: Vec<MatchedEvaluationArm>,
}

impl MatchedEvaluationArmSet {
    #[must_use]
    pub const fn contract_version(&self) -> &'static str {
        self.contract_version
    }

    #[must_use]
    pub fn protocol_id(&self) -> &str {
        &self.protocol_id
    }

    #[must_use]
    pub fn boolean_candidate_provenance(&self) -> &str {
        &self.boolean_candidate_provenance
    }

    #[must_use]
    pub const fn matched_cardinality(&self) -> MaskCardinality {
        self.matched_cardinality
    }

    #[must_use]
    pub fn arms(&self) -> &[MatchedEvaluationArm] {
        &self.arms
    }
}

/// Fail-closed errors while freezing the matched evaluation arm set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchedEvaluationArmError {
    CandidateCardinalityDrift(SparsityError),
    BaselineCardinalityDrift {
        arm_id: String,
        source: SparsityError,
    },
    DenseReference(SparsityError),
}

impl fmt::Display for MatchedEvaluationArmError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MatchedEvaluationArmError {}

/// Freeze the exact model-evaluation arms associated with one matched baseline set.
///
/// Arm order is canonical and stable: dense, Boolean, magnitude, structured,
/// then deterministic-random controls in the preregistered seed order preserved
/// by [`MatchedBaselineSet`]. Dense remains explicitly separate because it is a
/// fully active reference; every other arm must match the Boolean candidate's
/// exact width and retained count.
///
/// # Errors
///
/// Fails closed if the candidate no longer matches the baseline-set cardinality,
/// if any matched control drifts, or if the dense reference cannot be built.
pub fn build_matched_evaluation_arm_set(
    boolean_candidate: &ExactMask,
    baselines: &MatchedBaselineSet,
) -> Result<MatchedEvaluationArmSet, MatchedEvaluationArmError> {
    let matched_cardinality = baselines.cardinality();
    matched_cardinality
        .require_matched_cardinality(boolean_candidate.cardinality())
        .map_err(MatchedEvaluationArmError::CandidateCardinalityDrift)?;

    let total = matched_cardinality.total();
    let dense_indices: Vec<usize> = (0..total).collect();
    let dense = ExactMask::from_retained_indices(total, &dense_indices)
        .map_err(MatchedEvaluationArmError::DenseReference)?;

    let mut arms = Vec::with_capacity(4 + baselines.random().len());
    arms.push(MatchedEvaluationArm {
        arm_id: "dense".to_owned(),
        kind: MatchedEvaluationArmKind::Dense,
        mask: dense,
    });
    arms.push(matched_arm(
        "boolean",
        MatchedEvaluationArmKind::Boolean,
        boolean_candidate,
        matched_cardinality,
    )?);
    arms.push(matched_arm(
        "magnitude",
        MatchedEvaluationArmKind::Magnitude,
        baselines.magnitude(),
        matched_cardinality,
    )?);
    arms.push(matched_arm(
        "structured-nm",
        MatchedEvaluationArmKind::Structured,
        baselines.structured(),
        matched_cardinality,
    )?);
    for (seed, mask) in baselines.random() {
        arms.push(matched_arm(
            &format!("random:{seed}"),
            MatchedEvaluationArmKind::Random { seed: *seed },
            mask,
            matched_cardinality,
        )?);
    }

    Ok(MatchedEvaluationArmSet {
        contract_version: MATCHED_EVALUATION_ARMS_CONTRACT_VERSION,
        protocol_id: baselines.protocol().protocol_id.clone(),
        boolean_candidate_provenance: baselines.protocol().boolean_candidate_provenance.clone(),
        matched_cardinality,
        arms,
    })
}

fn matched_arm(
    arm_id: &str,
    kind: MatchedEvaluationArmKind,
    mask: &ExactMask,
    expected: MaskCardinality,
) -> Result<MatchedEvaluationArm, MatchedEvaluationArmError> {
    expected
        .require_matched_cardinality(mask.cardinality())
        .map_err(
            |source| MatchedEvaluationArmError::BaselineCardinalityDrift {
                arm_id: arm_id.to_owned(),
                source,
            },
        )?;
    Ok(MatchedEvaluationArm {
        arm_id: arm_id.to_owned(),
        kind,
        mask: mask.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparsity_matched_baselines::{MatchedBaselineProtocol, build_matched_baseline_set};

    fn protocol() -> MatchedBaselineProtocol {
        MatchedBaselineProtocol {
            protocol_id: "bl14.5-model-eval-v1".to_owned(),
            boolean_candidate_provenance: "boolean-mask:sha256:candidate".to_owned(),
            magnitude_score_provenance: "weights:sha256:a".to_owned(),
            structured_score_provenance: "weights:sha256:a".to_owned(),
            random_seed_provenance: "preregistered-seeds-v1".to_owned(),
            structured_group_size: 4,
            random_seeds: vec![7, 11, 13],
        }
    }

    fn fixture() -> (ExactMask, MatchedBaselineSet) {
        let candidate = ExactMask::from_retained_indices(8, &[0, 2, 5, 7]).unwrap();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];
        let baselines =
            build_matched_baseline_set(&candidate, &scores, &scores, protocol()).unwrap();
        (candidate, baselines)
    }

    #[test]
    fn freezes_dense_and_all_matched_controls_in_canonical_order() {
        let (candidate, baselines) = fixture();
        let set = build_matched_evaluation_arm_set(&candidate, &baselines).unwrap();

        assert_eq!(set.contract_version(), "bl14.matched-evaluation-arms.v1");
        assert_eq!(set.protocol_id(), "bl14.5-model-eval-v1");
        assert_eq!(
            set.boolean_candidate_provenance(),
            "boolean-mask:sha256:candidate"
        );
        assert_eq!(
            set.arms()
                .iter()
                .map(MatchedEvaluationArm::arm_id)
                .collect::<Vec<_>>(),
            vec![
                "dense",
                "boolean",
                "magnitude",
                "structured-nm",
                "random:7",
                "random:11",
                "random:13",
            ]
        );
    }

    #[test]
    fn dense_is_explicitly_full_while_every_other_arm_is_exactly_matched() {
        let (candidate, baselines) = fixture();
        let set = build_matched_evaluation_arm_set(&candidate, &baselines).unwrap();
        let dense = &set.arms()[0];
        assert_eq!(dense.kind(), MatchedEvaluationArmKind::Dense);
        assert_eq!(dense.mask().cardinality().retained(), 8);
        assert_eq!(dense.mask().cardinality().total(), 8);

        for arm in &set.arms()[1..] {
            assert_eq!(arm.mask().cardinality(), candidate.cardinality());
        }
    }

    #[test]
    fn random_seed_identity_and_mask_order_are_preserved_exactly() {
        let (candidate, baselines) = fixture();
        let set = build_matched_evaluation_arm_set(&candidate, &baselines).unwrap();
        for ((seed, baseline_mask), arm) in baselines.random().iter().zip(&set.arms()[4..]) {
            assert_eq!(arm.kind(), MatchedEvaluationArmKind::Random { seed: *seed });
            assert_eq!(arm.mask(), baseline_mask);
        }
    }

    #[test]
    fn candidate_substitution_with_same_density_but_different_width_fails_closed() {
        let (_candidate, baselines) = fixture();
        let substituted = ExactMask::from_retained_indices(4, &[0, 2]).unwrap();
        assert!(matches!(
            build_matched_evaluation_arm_set(&substituted, &baselines),
            Err(MatchedEvaluationArmError::CandidateCardinalityDrift(
                SparsityError::MaskWidthMismatch { .. }
            ))
        ));
    }

    #[test]
    fn candidate_substitution_with_same_width_but_different_retained_count_fails_closed() {
        let (_candidate, baselines) = fixture();
        let substituted = ExactMask::from_retained_indices(8, &[0, 2, 5]).unwrap();
        assert!(matches!(
            build_matched_evaluation_arm_set(&substituted, &baselines),
            Err(MatchedEvaluationArmError::CandidateCardinalityDrift(
                SparsityError::RetainedCountMismatch { .. }
            ))
        ));
    }
}
