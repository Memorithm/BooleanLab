//! Deterministic score-to-ranking helpers for BL-14 baseline calibration.
//!
//! This module deliberately accepts only exact integer score keys. It does not
//! define how random, magnitude, structured, or Boolean-controller evidence is
//! converted into those keys. That conversion remains part of each experiment's
//! frozen observation contract.

use core::cmp::Reverse;

use crate::{ExactMask, SparsityError};

/// Deterministically rank exact score keys from highest to lowest.
///
/// Equal scores are resolved by ascending original index. This makes the tie
/// policy explicit and reproducible instead of depending on sort stability or
/// input traversal details.
///
/// # Errors
///
/// Returns [`SparsityError::EmptyMask`] when `scores` is empty.
pub fn rank_descending_u64(scores: &[u64]) -> Result<Vec<usize>, SparsityError> {
    if scores.is_empty() {
        return Err(SparsityError::EmptyMask);
    }

    let mut ranking: Vec<usize> = (0..scores.len()).collect();
    ranking.sort_unstable_by_key(|&index| (Reverse(scores[index]), index));
    Ok(ranking)
}

/// Materialize an exact retained mask from deterministic integer score keys.
///
/// Higher score keys rank first; ties retain the lower original index first.
/// The caller owns the scientific meaning and provenance of the score keys.
///
/// # Errors
///
/// Returns the exact cardinality errors from [`ExactMask::from_ranked_indices`]
/// and [`SparsityError::EmptyMask`] when `scores` is empty.
pub fn mask_from_descending_u64_scores(
    scores: &[u64],
    retained: usize,
) -> Result<ExactMask, SparsityError> {
    let ranking = rank_descending_u64(scores)?;
    ExactMask::from_ranked_indices(scores.len(), retained, &ranking)
}

#[cfg(test)]
mod tests {
    use super::{mask_from_descending_u64_scores, rank_descending_u64};
    use crate::{MaskCardinality, SparsityError};

    #[test]
    fn ranking_is_descending_with_explicit_index_ties() {
        let ranking = rank_descending_u64(&[7, 11, 11, 3, 7]).unwrap();
        assert_eq!(ranking, vec![1, 2, 0, 4, 3]);
    }

    #[test]
    fn ranking_is_deterministic_for_all_equal_scores() {
        let ranking = rank_descending_u64(&[5, 5, 5, 5]).unwrap();
        assert_eq!(ranking, vec![0, 1, 2, 3]);
    }

    #[test]
    fn materialization_uses_exact_retained_prefix() {
        let mask = mask_from_descending_u64_scores(&[4, 9, 1, 9, 3], 3).unwrap();
        assert_eq!(mask.as_slice(), &[true, true, false, true, false]);
        assert_eq!(mask.cardinality(), MaskCardinality::new(3, 5).unwrap());
    }

    #[test]
    fn all_drop_and_all_keep_controls_remain_exact() {
        let dropped = mask_from_descending_u64_scores(&[4, 2, 8], 0).unwrap();
        assert_eq!(dropped.as_slice(), &[false, false, false]);

        let kept = mask_from_descending_u64_scores(&[4, 2, 8], 3).unwrap();
        assert_eq!(kept.as_slice(), &[true, true, true]);
    }

    #[test]
    fn empty_or_impossible_requests_fail_closed() {
        assert_eq!(rank_descending_u64(&[]), Err(SparsityError::EmptyMask));
        assert_eq!(
            mask_from_descending_u64_scores(&[], 0),
            Err(SparsityError::EmptyMask)
        );
        assert_eq!(
            mask_from_descending_u64_scores(&[1, 2, 3], 4),
            Err(SparsityError::RetainedExceedsTotal {
                retained: 4,
                total: 3,
            })
        );
    }
}
