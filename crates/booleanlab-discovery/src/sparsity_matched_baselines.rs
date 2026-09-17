//! BL-14.5 exact matched-density baseline construction.
//!
//! Boolean sparsity candidates may only be compared with magnitude, random, and
//! structured controls after the controls have been materialised at exactly the
//! same retained cardinality and width. This module freezes that comparison
//! boundary. It does not execute a numerical model, measure hardware, or inspect
//! HOLDOUT outcomes.

use std::collections::BTreeSet;
use std::fmt;

use booleanlab_core::{
    ExactMask, MaskCardinality, SparsityError, StructuredSparsityError, deterministic_random_mask,
    mask_from_descending_u64_scores, structured_nm_mask_from_u64_scores,
};

/// Version of the BL-14 matched-baseline construction contract.
pub const MATCHED_BASELINE_CONTRACT_VERSION: &str = "bl14.matched-baseline-set.v1";

/// Provenance and structure frozen before baseline construction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedBaselineProtocol {
    /// Experiment-owned protocol identity.
    pub protocol_id: String,
    /// Identity of the already-materialised Boolean candidate mask.
    pub boolean_candidate_provenance: String,
    /// Identity of the exact score vector used by the magnitude-like baseline.
    pub magnitude_score_provenance: String,
    /// Identity of the exact score vector used by the structured baseline.
    pub structured_score_provenance: String,
    /// Identity of the rule that selected the declared random seeds.
    pub random_seed_provenance: String,
    /// Number of consecutive elements per structured N:M group.
    pub structured_group_size: usize,
    /// Frozen random-control seeds. Duplicate seeds are rejected.
    pub random_seeds: Vec<u64>,
}

/// Exact controls matched to one already-materialised Boolean candidate mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedBaselineSet {
    contract_version: &'static str,
    protocol: MatchedBaselineProtocol,
    cardinality: MaskCardinality,
    magnitude: ExactMask,
    random: Vec<(u64, ExactMask)>,
    structured: ExactMask,
    structured_retained_per_group: usize,
}

impl MatchedBaselineSet {
    #[must_use]
    pub const fn contract_version(&self) -> &'static str {
        self.contract_version
    }
    #[must_use]
    pub const fn protocol(&self) -> &MatchedBaselineProtocol {
        &self.protocol
    }
    #[must_use]
    pub const fn cardinality(&self) -> MaskCardinality {
        self.cardinality
    }
    #[must_use]
    pub const fn magnitude(&self) -> &ExactMask {
        &self.magnitude
    }
    #[must_use]
    pub fn random(&self) -> &[(u64, ExactMask)] {
        &self.random
    }
    #[must_use]
    pub const fn structured(&self) -> &ExactMask {
        &self.structured
    }
    #[must_use]
    pub const fn structured_retained_per_group(&self) -> usize {
        self.structured_retained_per_group
    }
}

/// Fail-closed construction errors for the matched-baseline set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatchedBaselineError {
    EmptyProtocolField {
        field: &'static str,
    },
    EmptyRandomSeeds,
    DuplicateRandomSeed {
        seed: u64,
    },
    ScoreWidthMismatch {
        baseline: &'static str,
        expected: usize,
        actual: usize,
    },
    ZeroStructuredGroupSize,
    StructuredGroupSizeDoesNotDivideTotal {
        total: usize,
        group_size: usize,
    },
    StructuredRetainedNotDivisibleAcrossGroups {
        retained: usize,
        groups: usize,
    },
    Sparsity(SparsityError),
    Structured(StructuredSparsityError),
}

impl From<SparsityError> for MatchedBaselineError {
    fn from(error: SparsityError) -> Self {
        Self::Sparsity(error)
    }
}

impl From<StructuredSparsityError> for MatchedBaselineError {
    fn from(error: StructuredSparsityError) -> Self {
        Self::Structured(error)
    }
}

/// Construct magnitude, random, and structured controls at exactly the same
/// width and retained cardinality as `boolean_candidate`.
///
/// The score arrays are opaque exact integer keys. Their scientific meaning is
/// intentionally caller-owned and bound only by the supplied provenance ids.
/// The structured N:M shape is accepted only when the candidate's retained
/// count can be distributed uniformly across complete groups; the function does
/// not round density up or down to manufacture a match.
///
/// # Errors
///
/// Fails on empty provenance, absent/duplicate random seeds, score-width drift,
/// an impossible structured grouping, or any underlying exact-mask error.
pub fn build_matched_baseline_set(
    boolean_candidate: &ExactMask,
    magnitude_scores: &[u64],
    structured_scores: &[u64],
    protocol: MatchedBaselineProtocol,
) -> Result<MatchedBaselineSet, MatchedBaselineError> {
    validate_protocol(&protocol)?;

    let cardinality = boolean_candidate.cardinality();
    let total = cardinality.total();
    let retained = cardinality.retained();

    require_score_width("magnitude", magnitude_scores.len(), total)?;
    require_score_width("structured", structured_scores.len(), total)?;

    if protocol.structured_group_size == 0 {
        return Err(MatchedBaselineError::ZeroStructuredGroupSize);
    }
    if !total.is_multiple_of(protocol.structured_group_size) {
        return Err(
            MatchedBaselineError::StructuredGroupSizeDoesNotDivideTotal {
                total,
                group_size: protocol.structured_group_size,
            },
        );
    }
    let groups = total / protocol.structured_group_size;
    if !retained.is_multiple_of(groups) {
        return Err(
            MatchedBaselineError::StructuredRetainedNotDivisibleAcrossGroups { retained, groups },
        );
    }
    let structured_retained_per_group = retained / groups;

    let magnitude = mask_from_descending_u64_scores(magnitude_scores, retained)?;
    cardinality.require_matched_cardinality(magnitude.cardinality())?;

    let random = protocol
        .random_seeds
        .iter()
        .copied()
        .map(|seed| {
            let mask = deterministic_random_mask(total, retained, seed)?;
            cardinality.require_matched_cardinality(mask.cardinality())?;
            Ok((seed, mask))
        })
        .collect::<Result<Vec<_>, SparsityError>>()?;

    let structured = structured_nm_mask_from_u64_scores(
        structured_scores,
        structured_retained_per_group,
        protocol.structured_group_size,
    )?;
    cardinality.require_matched_cardinality(structured.cardinality())?;

    Ok(MatchedBaselineSet {
        contract_version: MATCHED_BASELINE_CONTRACT_VERSION,
        protocol,
        cardinality,
        magnitude,
        random,
        structured,
        structured_retained_per_group,
    })
}

fn validate_protocol(protocol: &MatchedBaselineProtocol) -> Result<(), MatchedBaselineError> {
    for (field, value) in [
        ("protocol_id", protocol.protocol_id.as_str()),
        (
            "boolean_candidate_provenance",
            protocol.boolean_candidate_provenance.as_str(),
        ),
        (
            "magnitude_score_provenance",
            protocol.magnitude_score_provenance.as_str(),
        ),
        (
            "structured_score_provenance",
            protocol.structured_score_provenance.as_str(),
        ),
        (
            "random_seed_provenance",
            protocol.random_seed_provenance.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(MatchedBaselineError::EmptyProtocolField { field });
        }
    }
    if protocol.random_seeds.is_empty() {
        return Err(MatchedBaselineError::EmptyRandomSeeds);
    }
    let mut seeds = BTreeSet::new();
    for &seed in &protocol.random_seeds {
        if !seeds.insert(seed) {
            return Err(MatchedBaselineError::DuplicateRandomSeed { seed });
        }
    }
    Ok(())
}

fn require_score_width(
    baseline: &'static str,
    actual: usize,
    expected: usize,
) -> Result<(), MatchedBaselineError> {
    if actual != expected {
        return Err(MatchedBaselineError::ScoreWidthMismatch {
            baseline,
            expected,
            actual,
        });
    }
    Ok(())
}

impl fmt::Display for MatchedBaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for MatchedBaselineError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn protocol() -> MatchedBaselineProtocol {
        MatchedBaselineProtocol {
            protocol_id: "bl14.5-search-model-v1".to_owned(),
            boolean_candidate_provenance: "boolean-mask:sha256:candidate".to_owned(),
            magnitude_score_provenance: "weights:sha256:a".to_owned(),
            structured_score_provenance: "weights:sha256:a".to_owned(),
            random_seed_provenance: "preregistered-seeds-v1".to_owned(),
            structured_group_size: 4,
            random_seeds: vec![7, 11, 13],
        }
    }

    fn candidate() -> ExactMask {
        ExactMask::from_retained_indices(8, &[0, 2, 5, 7]).unwrap()
    }

    #[test]
    fn builds_all_controls_at_exact_candidate_cardinality() {
        let candidate = candidate();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];
        let set = build_matched_baseline_set(&candidate, &scores, &scores, protocol()).unwrap();

        assert_eq!(set.contract_version(), "bl14.matched-baseline-set.v1");
        assert_eq!(set.cardinality(), candidate.cardinality());
        assert_eq!(set.structured_retained_per_group(), 2);
        assert_eq!(set.magnitude().cardinality(), candidate.cardinality());
        assert_eq!(set.structured().cardinality(), candidate.cardinality());
        assert_eq!(set.random().len(), 3);
        for (_, mask) in set.random() {
            assert_eq!(mask.cardinality(), candidate.cardinality());
        }
    }

    #[test]
    fn random_controls_are_reproducible_in_declared_seed_order() {
        let candidate = candidate();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];
        let left = build_matched_baseline_set(&candidate, &scores, &scores, protocol()).unwrap();
        let right = build_matched_baseline_set(&candidate, &scores, &scores, protocol()).unwrap();

        assert_eq!(left.random(), right.random());
        assert_eq!(
            left.random()
                .iter()
                .map(|(seed, _)| *seed)
                .collect::<Vec<_>>(),
            vec![7, 11, 13]
        );
    }

    #[test]
    fn rejects_score_width_and_structured_density_drift() {
        let candidate = candidate();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores[..7], &scores, protocol()),
            Err(MatchedBaselineError::ScoreWidthMismatch {
                baseline: "magnitude",
                expected: 8,
                actual: 7,
            })
        );

        let three_of_eight = ExactMask::from_retained_indices(8, &[0, 2, 5]).unwrap();
        assert_eq!(
            build_matched_baseline_set(&three_of_eight, &scores, &scores, protocol()),
            Err(
                MatchedBaselineError::StructuredRetainedNotDivisibleAcrossGroups {
                    retained: 3,
                    groups: 2,
                }
            )
        );
    }

    #[test]
    fn rejects_invalid_group_geometry_without_density_rounding() {
        let candidate = candidate();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];

        let mut zero = protocol();
        zero.structured_group_size = 0;
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores, &scores, zero),
            Err(MatchedBaselineError::ZeroStructuredGroupSize)
        );

        let mut truncated = protocol();
        truncated.structured_group_size = 3;
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores, &scores, truncated),
            Err(
                MatchedBaselineError::StructuredGroupSizeDoesNotDivideTotal {
                    total: 8,
                    group_size: 3,
                }
            )
        );
    }

    #[test]
    fn rejects_empty_provenance_and_duplicate_or_missing_random_controls() {
        let candidate = candidate();
        let scores = [9, 1, 8, 2, 7, 3, 6, 4];

        let mut missing = protocol();
        missing.magnitude_score_provenance = " ".to_owned();
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores, &scores, missing),
            Err(MatchedBaselineError::EmptyProtocolField {
                field: "magnitude_score_provenance",
            })
        );

        let mut empty_random = protocol();
        empty_random.random_seeds.clear();
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores, &scores, empty_random),
            Err(MatchedBaselineError::EmptyRandomSeeds)
        );

        let mut duplicate = protocol();
        duplicate.random_seeds = vec![7, 7];
        assert_eq!(
            build_matched_baseline_set(&candidate, &scores, &scores, duplicate),
            Err(MatchedBaselineError::DuplicateRandomSeed { seed: 7 })
        );
    }

    #[test]
    fn all_drop_and_all_keep_candidates_remain_exactly_matchable() {
        let scores = [4, 3, 2, 1];
        let mut p = protocol();
        p.structured_group_size = 2;

        let drop = ExactMask::from_retained_indices(4, &[]).unwrap();
        let dropped = build_matched_baseline_set(&drop, &scores, &scores, p.clone()).unwrap();
        assert_eq!(dropped.cardinality().retained(), 0);
        assert_eq!(dropped.structured_retained_per_group(), 0);

        let keep = ExactMask::from_retained_indices(4, &[0, 1, 2, 3]).unwrap();
        let kept = build_matched_baseline_set(&keep, &scores, &scores, p).unwrap();
        assert_eq!(kept.cardinality().retained(), 4);
        assert_eq!(kept.structured_retained_per_group(), 2);
    }
}
