//! Reproducible threshold sweeps for BL-4 Boolean attention admission.
//!
//! The core crate owns exact routing primitives. This module adds the
//! experiment-facing sweep needed to compare candidate density against exact
//! oracle misses without selecting one threshold post hoc.

use core::fmt;

use booleanlab_core::{
    AdmissionScore, AttentionRouterError, BitSignature, hamming_admission_row, score_admission,
};

/// One preregistered Hamming threshold and its exact admission counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ThresholdPoint {
    threshold: u64,
    score: AdmissionScore,
}

impl ThresholdPoint {
    #[must_use]
    pub const fn threshold(self) -> u64 {
        self.threshold
    }

    #[must_use]
    pub const fn score(self) -> AdmissionScore {
        self.score
    }
}

/// Fail-closed errors for BL-4 threshold-frontier experiments.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionFrontierError {
    EmptyThresholdSweep,
    ThresholdsNotStrictlyIncreasing {
        index: usize,
        previous: u64,
        current: u64,
    },
    ReferenceLengthMismatch {
        keys: usize,
        reference: usize,
    },
    Router(AttentionRouterError),
}

impl fmt::Display for AttentionFrontierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AttentionFrontierError {}

impl From<AttentionRouterError> for AttentionFrontierError {
    fn from(value: AttentionRouterError) -> Self {
        Self::Router(value)
    }
}

/// Evaluate one query/key set across a strictly increasing threshold schedule.
///
/// Threshold order is part of the experiment contract. Duplicate or decreasing
/// thresholds are rejected instead of being silently sorted because that would
/// mutate the preregistered sweep.
///
/// # Errors
///
/// Returns an error for an empty/non-monotonic threshold schedule, key/reference
/// length mismatch, or any fail-closed routing error from `booleanlab-core`.
pub fn sweep_hamming_thresholds(
    query: &BitSignature,
    keys: &[BitSignature],
    reference: &[bool],
    thresholds: &[u64],
) -> Result<Vec<ThresholdPoint>, AttentionFrontierError> {
    if thresholds.is_empty() {
        return Err(AttentionFrontierError::EmptyThresholdSweep);
    }
    if keys.len() != reference.len() {
        return Err(AttentionFrontierError::ReferenceLengthMismatch {
            keys: keys.len(),
            reference: reference.len(),
        });
    }

    for (offset, window) in thresholds.windows(2).enumerate() {
        let previous = window[0];
        let current = window[1];
        if previous >= current {
            return Err(AttentionFrontierError::ThresholdsNotStrictlyIncreasing {
                index: offset + 1,
                previous,
                current,
            });
        }
    }

    thresholds
        .iter()
        .copied()
        .map(|threshold| {
            let candidate = hamming_admission_row(query, keys, threshold)?;
            let score = score_admission(&candidate, reference)?;
            Ok(ThresholdPoint { threshold, score })
        })
        .collect()
}

/// Return the exact non-dominated density/miss frontier from a threshold sweep.
///
/// Point `A` dominates `B` when `A` admits no more candidates and produces no
/// more false negatives, with at least one of those two quantities strictly
/// smaller. Output order matches the input sweep order.
#[must_use]
pub fn density_false_negative_frontier(points: &[ThresholdPoint]) -> Vec<ThresholdPoint> {
    points
        .iter()
        .copied()
        .filter(|candidate| {
            !points.iter().any(|other| {
                let candidate_score = candidate.score();
                let other_score = other.score();
                let no_more_candidates = other_score.admitted() <= candidate_score.admitted();
                let no_more_misses =
                    other_score.false_negative() <= candidate_score.false_negative();
                let strictly_better = other_score.admitted() < candidate_score.admitted()
                    || other_score.false_negative() < candidate_score.false_negative();
                no_more_candidates && no_more_misses && strictly_better
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use booleanlab_core::{AttentionRouterError, BitSignature};

    use super::{
        AttentionFrontierError, density_false_negative_frontier, sweep_hamming_thresholds,
    };

    fn signature(bits: &[bool]) -> BitSignature {
        BitSignature::from_bits(bits).unwrap()
    }

    #[test]
    fn threshold_sweep_records_exact_density_miss_tradeoff() {
        let query = signature(&[true, false, true, false]);
        let keys = [
            signature(&[true, false, true, false]),
            signature(&[true, true, true, false]),
            signature(&[false, true, true, false]),
            signature(&[false, true, false, true]),
        ];
        let reference = [true, true, true, false];

        let points = sweep_hamming_thresholds(&query, &keys, &reference, &[0, 1, 2, 4]).unwrap();

        assert_eq!(points.len(), 4);
        assert_eq!(points[0].threshold(), 0);
        assert_eq!(points[0].score().admitted(), 1);
        assert_eq!(points[0].score().false_negative(), 2);

        assert_eq!(points[1].threshold(), 1);
        assert_eq!(points[1].score().admitted(), 2);
        assert_eq!(points[1].score().false_negative(), 1);

        assert_eq!(points[2].threshold(), 2);
        assert_eq!(points[2].score().admitted(), 3);
        assert_eq!(points[2].score().false_negative(), 0);

        assert_eq!(points[3].threshold(), 4);
        assert_eq!(points[3].score().admitted(), 4);
        assert_eq!(points[3].score().false_negative(), 0);
        assert_eq!(points[3].score().false_positive(), 1);
    }

    #[test]
    fn frontier_removes_point_with_more_candidates_and_same_miss_count() {
        let query = signature(&[true, false, true, false]);
        let keys = [
            signature(&[true, false, true, false]),
            signature(&[true, true, true, false]),
            signature(&[false, true, true, false]),
            signature(&[false, true, false, true]),
        ];
        let reference = [true, true, true, false];

        let points = sweep_hamming_thresholds(&query, &keys, &reference, &[0, 1, 2, 4]).unwrap();
        let frontier = density_false_negative_frontier(&points);

        assert_eq!(
            frontier.iter().map(|point| point.threshold()).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn sweep_rejects_non_preregistered_order_mutation() {
        let query = signature(&[true, false]);
        let keys = [signature(&[true, false])];
        let reference = [true];

        assert_eq!(
            sweep_hamming_thresholds(&query, &keys, &reference, &[0, 1, 1]),
            Err(AttentionFrontierError::ThresholdsNotStrictlyIncreasing {
                index: 2,
                previous: 1,
                current: 1,
            })
        );
    }

    #[test]
    fn sweep_rejects_reference_shape_mismatch() {
        let query = signature(&[true, false]);
        let keys = [signature(&[true, false]), signature(&[false, false])];

        assert_eq!(
            sweep_hamming_thresholds(&query, &keys, &[true], &[0]),
            Err(AttentionFrontierError::ReferenceLengthMismatch {
                keys: 2,
                reference: 1,
            })
        );
    }

    #[test]
    fn sweep_propagates_router_threshold_validation() {
        let query = signature(&[true, false]);
        let keys = [signature(&[true, false])];
        let reference = [true];

        assert_eq!(
            sweep_hamming_thresholds(&query, &keys, &reference, &[3]),
            Err(AttentionFrontierError::Router(
                AttentionRouterError::ThresholdExceedsSignatureWidth {
                    threshold: 3,
                    bit_len: 2,
                }
            ))
        );
    }
}