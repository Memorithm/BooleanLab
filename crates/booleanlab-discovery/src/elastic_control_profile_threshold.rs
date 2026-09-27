//! Exact monotone occupancy threshold for the BL-BE5 control-profile partition.
//!
//! For a fixed physical slot count, dense-W128 and hybrid-W64+Boolean payload
//! costs are constant with occupancy, while sparse-W512 grows linearly with the
//! number of present slots. Sparse membership in the exact minimum set is
//! therefore monotone: once sparse stops being minimal, it never becomes
//! minimal again as occupancy increases.
//!
//! This module proves that reduced threshold against the already-qualified exact
//! oracle over the full 1..=4096 slot domain. It remains offline evidence, not a
//! production selector.

use crate::elastic_control_profile_partition::{
    ControlProfilePartitionError, DEFAULT_EXHAUSTIVE_MAX_SLOTS, PROFILE_DENSE_W128,
    PROFILE_HYBRID_W64_BOOLEAN, PROFILE_SPARSE_W512, payloads,
};

/// Versioned identity of the monotone sparse-threshold contract.
pub const ELASTIC_CONTROL_PROFILE_SPARSE_THRESHOLD_V1: &str =
    "booleanlab.elastic-control-profile-sparse-threshold@1.0.0";

/// Exact largest present-slot count for which sparse-W512 belongs to the
/// minimum/tie set.
///
/// The two exact sparse-minimum predicates are:
///
/// 4 * present <= slots
///
/// and
///
/// 8 * present <= slots + 3 * ceil(slots / 64).
///
/// # Errors
///
/// Returns the same bounded-domain and arithmetic errors as the exact oracle.
pub fn sparse_minimum_threshold(slot_count: usize) -> Result<usize, ControlProfilePartitionError> {
    if slot_count == 0 {
        return Err(ControlProfilePartitionError::ZeroSlots);
    }

    let bitmap_words = slot_count.div_ceil(64);
    let sparse_vs_dense = slot_count / 4;
    let sparse_vs_hybrid = slot_count
        .checked_add(
            bitmap_words
                .checked_mul(3)
                .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?,
        )
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?
        / 8;

    Ok(sparse_vs_dense.min(sparse_vs_hybrid))
}

/// Exact dense/hybrid minimum mask after sparse is no longer minimal.
///
/// This comparison is occupancy-independent because both retained dense
/// representations allocate by physical slot domain, not present-slot count.
///
/// # Errors
///
/// Returns a zero-slot or arithmetic-overflow error.
pub fn non_sparse_minimum_mask(slot_count: usize) -> Result<u8, ControlProfilePartitionError> {
    if slot_count == 0 {
        return Err(ControlProfilePartitionError::ZeroSlots);
    }
    let bitmap_words = slot_count.div_ceil(64);
    let hybrid_boundary = bitmap_words
        .checked_mul(3)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;

    Ok(match hybrid_boundary.cmp(&slot_count) {
        core::cmp::Ordering::Less => PROFILE_HYBRID_W64_BOOLEAN,
        core::cmp::Ordering::Equal => PROFILE_DENSE_W128 | PROFILE_HYBRID_W64_BOOLEAN,
        core::cmp::Ordering::Greater => PROFILE_DENSE_W128,
    })
}

/// Verify the reduced threshold against every exact occupancy row.
///
/// # Errors
///
/// Returns the first exact mismatch or underlying accounting error.
pub fn verify_sparse_threshold(max_slots: usize) -> Result<(), SparseThresholdError> {
    if max_slots == 0 {
        return Err(SparseThresholdError::Oracle(
            ControlProfilePartitionError::ZeroSlots,
        ));
    }
    if max_slots > DEFAULT_EXHAUSTIVE_MAX_SLOTS {
        return Err(SparseThresholdError::OutsideQualifiedDomain {
            observed: max_slots,
            maximum: DEFAULT_EXHAUSTIVE_MAX_SLOTS,
        });
    }

    for slot_count in 1..=max_slots {
        let threshold = sparse_minimum_threshold(slot_count)?;
        let non_sparse = non_sparse_minimum_mask(slot_count)?;

        for present_slots in 0..=slot_count {
            let exact = payloads(slot_count, present_slots)?.exact_minimum_mask();

            if present_slots < threshold && exact != PROFILE_SPARSE_W512 {
                return Err(SparseThresholdError::Mismatch {
                    slot_count,
                    present_slots,
                    threshold,
                    exact_mask: exact,
                    expected_relation: "below-threshold-sparse-only",
                });
            }

            if present_slots == threshold && exact & PROFILE_SPARSE_W512 == 0 {
                return Err(SparseThresholdError::Mismatch {
                    slot_count,
                    present_slots,
                    threshold,
                    exact_mask: exact,
                    expected_relation: "threshold-must-include-sparse",
                });
            }

            if present_slots > threshold && exact != non_sparse {
                return Err(SparseThresholdError::Mismatch {
                    slot_count,
                    present_slots,
                    threshold,
                    exact_mask: exact,
                    expected_relation: "above-threshold-fixed-nonsparse",
                });
            }
        }
    }

    Ok(())
}

/// Fail-closed threshold proof errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SparseThresholdError {
    OutsideQualifiedDomain {
        observed: usize,
        maximum: usize,
    },
    Mismatch {
        slot_count: usize,
        present_slots: usize,
        threshold: usize,
        exact_mask: u8,
        expected_relation: &'static str,
    },
    Oracle(ControlProfilePartitionError),
}

impl From<ControlProfilePartitionError> for SparseThresholdError {
    fn from(value: ControlProfilePartitionError) -> Self {
        Self::Oracle(value)
    }
}

impl std::fmt::Display for SparseThresholdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutsideQualifiedDomain { observed, maximum } => write!(
                formatter,
                "sparse-threshold verification requested {observed} slots; qualified maximum is {maximum}"
            ),
            Self::Mismatch {
                slot_count,
                present_slots,
                threshold,
                exact_mask,
                expected_relation,
            } => write!(
                formatter,
                "sparse-threshold mismatch slots={slot_count} present={present_slots} threshold={threshold} mask={exact_mask:#05b} expected={expected_relation}"
            ),
            Self::Oracle(error) => write!(formatter, "profile oracle failed: {error}"),
        }
    }
}

impl std::error::Error for SparseThresholdError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_formula_matches_known_domains() {
        assert_eq!(sparse_minimum_threshold(1).unwrap(), 0);
        assert_eq!(sparse_minimum_threshold(3).unwrap(), 0);
        assert_eq!(sparse_minimum_threshold(4).unwrap(), 0);
        assert_eq!(sparse_minimum_threshold(16).unwrap(), 2);
        assert_eq!(sparse_minimum_threshold(64).unwrap(), 8);
        assert_eq!(sparse_minimum_threshold(256).unwrap(), 33);
    }

    #[test]
    fn nonsparse_winner_is_occupancy_independent() {
        for slot_count in 1..=DEFAULT_EXHAUSTIVE_MAX_SLOTS {
            let threshold = sparse_minimum_threshold(slot_count).unwrap();
            let expected = non_sparse_minimum_mask(slot_count).unwrap();

            for present_slots in threshold.saturating_add(1)..=slot_count {
                let exact = payloads(slot_count, present_slots)
                    .unwrap()
                    .exact_minimum_mask();
                assert_eq!(
                    exact, expected,
                    "slot_count={slot_count}, present_slots={present_slots}"
                );
            }
        }
    }

    #[test]
    fn exhaustive_qualified_domain_matches_closed_threshold() {
        verify_sparse_threshold(DEFAULT_EXHAUSTIVE_MAX_SLOTS).unwrap();
    }

    #[test]
    fn threshold_domain_bounds_fail_closed() {
        assert_eq!(
            sparse_minimum_threshold(0),
            Err(ControlProfilePartitionError::ZeroSlots)
        );
        assert!(matches!(
            verify_sparse_threshold(DEFAULT_EXHAUSTIVE_MAX_SLOTS + 1),
            Err(SparseThresholdError::OutsideQualifiedDomain { .. })
        ));
    }
}
