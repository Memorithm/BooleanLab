//! Deterministic score-to-ranking helpers for BL-14 baseline calibration.
//!
//! This module deliberately accepts only exact integer score keys. It does not
//! define how magnitude, structured, or Boolean-controller evidence is converted
//! into those keys. The random calibration baseline is the one exception: its
//! counter-based key generator is frozen here so a declared seed reproduces the
//! same exact ranking without depending on a platform RNG implementation.

use core::cmp::Reverse;

use crate::{ExactMask, SparsityError};

const SPLITMIX64_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;

fn mix_splitmix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

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

/// Generate the frozen counter-based random keys used by the BL-14 random
/// calibration baseline.
///
/// Each index receives one SplitMix64-style mixed counter derived solely from
/// `(seed, index)`. The function is deterministic and platform-independent; it
/// is not a cryptographic RNG and must not be used for security-sensitive work.
/// Exact ties, although possible for finite `u64` keys, are resolved by
/// [`rank_descending_u64`] using ascending index.
///
/// # Errors
///
/// Returns [`SparsityError::EmptyMask`] when `total` is zero.
pub fn deterministic_random_keys(total: usize, seed: u64) -> Result<Vec<u64>, SparsityError> {
    if total == 0 {
        return Err(SparsityError::EmptyMask);
    }

    Ok((0..total)
        .map(|index| {
            let counter = (index as u64).wrapping_add(1);
            mix_splitmix64(seed.wrapping_add(SPLITMIX64_GAMMA.wrapping_mul(counter)))
        })
        .collect())
}

/// Materialize the density-matched BL-14 random baseline for a frozen seed.
///
/// The returned mask contains exactly `retained` entries when the request is
/// valid. Randomness only determines the deterministic ranking; cardinality is
/// enforced by [`ExactMask::from_ranked_indices`].
///
/// # Errors
///
/// Returns [`SparsityError::EmptyMask`] when `total` is zero and the standard
/// exact-cardinality errors when `retained` exceeds `total`.
pub fn deterministic_random_mask(
    total: usize,
    retained: usize,
    seed: u64,
) -> Result<ExactMask, SparsityError> {
    let keys = deterministic_random_keys(total, seed)?;
    mask_from_descending_u64_scores(&keys, retained)
}

#[cfg(test)]
mod tests {
    use super::{
        deterministic_random_keys, deterministic_random_mask, mask_from_descending_u64_scores,
        rank_descending_u64,
    };
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

    #[test]
    fn random_key_stream_has_frozen_vectors() {
        assert_eq!(
            deterministic_random_keys(5, 0).unwrap(),
            vec![
                0xE220_A839_7B1D_CDAF,
                0x6E78_9E6A_A1B9_65F4,
                0x06C4_5D18_8009_454F,
                0xF88B_B8A8_724C_81EC,
                0x1B39_896A_51A8_749B,
            ]
        );
    }

    #[test]
    fn random_mask_is_seed_reproducible_and_exactly_density_matched() {
        let left = deterministic_random_mask(16, 5, 0x1234_5678).unwrap();
        let right = deterministic_random_mask(16, 5, 0x1234_5678).unwrap();

        assert_eq!(left, right);
        assert_eq!(left.cardinality(), MaskCardinality::new(5, 16).unwrap());
    }

    #[test]
    fn random_mask_handles_all_drop_all_keep_and_invalid_requests() {
        let dropped = deterministic_random_mask(4, 0, 7).unwrap();
        assert_eq!(dropped.as_slice(), &[false, false, false, false]);

        let kept = deterministic_random_mask(4, 4, 7).unwrap();
        assert_eq!(kept.as_slice(), &[true, true, true, true]);

        assert_eq!(
            deterministic_random_mask(0, 0, 7),
            Err(SparsityError::EmptyMask)
        );
        assert_eq!(
            deterministic_random_mask(3, 4, 7),
            Err(SparsityError::RetainedExceedsTotal {
                retained: 4,
                total: 3,
            })
        );
    }
}
