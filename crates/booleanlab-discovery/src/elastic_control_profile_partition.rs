//! Exact bounded partition oracle for SLHAv2 Elastic control-plane profiles.
//!
//! Source semantics are pinned to SLHAv2 merge
//! `0bf49558eef14519ae1ce4b246b67347941c8237`, which retains three read-only
//! control-plane representations:
//!
//! - sparse W512: one 512-bit word per present slot;
//! - dense W128: one 128-bit word per physical slot position;
//! - hybrid W64 + Boolean planes: one 64-bit generation lane per slot plus
//!   three packed bitplanes over the slot domain.
//!
//! BooleanLab owns this exact bounded differential oracle only. It does not
//! promote a runtime policy or actuate SLHAv2/ElasticXxx state.

/// Pinned SLHAv2 source revision for the compared representation formulas.
pub const SLHA_CONTROL_PROFILE_SOURCE_REVISION: &str =
    "0bf49558eef14519ae1ce4b246b67347941c8237";

/// Largest slot count exhaustively qualified by the default test.
pub const DEFAULT_EXHAUSTIVE_MAX_SLOTS: usize = 4096;

/// Candidate bit in an exact argmin/tie mask.
pub const PROFILE_SPARSE_W512: u8 = 1 << 0;
/// Candidate bit in an exact argmin/tie mask.
pub const PROFILE_DENSE_W128: u8 = 1 << 1;
/// Candidate bit in an exact argmin/tie mask.
pub const PROFILE_HYBRID_W64_BOOLEAN: u8 = 1 << 2;

/// Exact structural payload bits for one bounded state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlProfilePayloads {
    pub slot_count: usize,
    pub present_slots: usize,
    pub sparse_w512_bits: usize,
    pub dense_w128_bits: usize,
    pub hybrid_w64_boolean_bits: usize,
}

impl ControlProfilePayloads {
    /// Exact argmin set, with one bit per candidate and ties preserved.
    #[must_use]
    pub fn exact_minimum_mask(self) -> u8 {
        let minimum = self
            .sparse_w512_bits
            .min(self.dense_w128_bits)
            .min(self.hybrid_w64_boolean_bits);
        let mut mask = 0_u8;
        if self.sparse_w512_bits == minimum {
            mask |= PROFILE_SPARSE_W512;
        }
        if self.dense_w128_bits == minimum {
            mask |= PROFILE_DENSE_W128;
        }
        if self.hybrid_w64_boolean_bits == minimum {
            mask |= PROFILE_HYBRID_W64_BOOLEAN;
        }
        mask
    }

    /// Boolean comparison predicate for sparse membership in the exact argmin.
    #[must_use]
    pub fn sparse_is_minimum_predicate(self) -> bool {
        self.sparse_w512_bits <= self.dense_w128_bits
            && self.sparse_w512_bits <= self.hybrid_w64_boolean_bits
    }

    /// Boolean comparison predicate for dense membership in the exact argmin.
    #[must_use]
    pub fn dense_is_minimum_predicate(self) -> bool {
        self.dense_w128_bits <= self.sparse_w512_bits
            && self.dense_w128_bits <= self.hybrid_w64_boolean_bits
    }

    /// Boolean comparison predicate for hybrid membership in the exact argmin.
    #[must_use]
    pub fn hybrid_is_minimum_predicate(self) -> bool {
        self.hybrid_w64_boolean_bits <= self.sparse_w512_bits
            && self.hybrid_w64_boolean_bits <= self.dense_w128_bits
    }

    /// Reconstruct the argmin/tie mask from the three Boolean predicates.
    #[must_use]
    pub fn predicate_minimum_mask(self) -> u8 {
        u8::from(self.sparse_is_minimum_predicate()) * PROFILE_SPARSE_W512
            | u8::from(self.dense_is_minimum_predicate()) * PROFILE_DENSE_W128
            | u8::from(self.hybrid_is_minimum_predicate()) * PROFILE_HYBRID_W64_BOOLEAN
    }
}

/// Exact payload accounting copied as formulas, not runtime implementation.
///
/// # Errors
///
/// Rejects zero slot domains, impossible present counts and arithmetic overflow.
pub fn payloads(
    slot_count: usize,
    present_slots: usize,
) -> Result<ControlProfilePayloads, ControlProfilePartitionError> {
    if slot_count == 0 {
        return Err(ControlProfilePartitionError::ZeroSlots);
    }
    if present_slots > slot_count {
        return Err(ControlProfilePartitionError::PresentExceedsSlots {
            present_slots,
            slot_count,
        });
    }

    let sparse_w512_bits = present_slots
        .checked_mul(512)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let dense_w128_bits = slot_count
        .checked_mul(128)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let bitmap_words = slot_count.div_ceil(64);
    let hybrid_generation_bits = slot_count
        .checked_mul(64)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let hybrid_boolean_bits = bitmap_words
        .checked_mul(3)
        .and_then(|words| words.checked_mul(64))
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let hybrid_w64_boolean_bits = hybrid_generation_bits
        .checked_add(hybrid_boolean_bits)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;

    Ok(ControlProfilePayloads {
        slot_count,
        present_slots,
        sparse_w512_bits,
        dense_w128_bits,
        hybrid_w64_boolean_bits,
    })
}

/// Algebraically reduced predicates equivalent to the direct payload
/// comparisons. These are the compact candidate boundary rules BooleanLab
/// qualifies; they are not a production selector.
///
/// Sparse <= dense:
/// `present * 512 <= slots * 128` => `4 * present <= slots`.
///
/// Sparse <= hybrid:
/// `present * 512 <= slots * 64 + ceil(slots/64) * 192` =>
/// `8 * present <= slots + 3 * ceil(slots/64)`.
///
/// Hybrid <= dense:
/// `slots * 64 + ceil(slots/64) * 192 <= slots * 128` =>
/// `3 * ceil(slots/64) <= slots`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReducedBoundaryPredicates {
    pub sparse_le_dense: bool,
    pub sparse_le_hybrid: bool,
    pub hybrid_le_dense: bool,
}

pub fn reduced_predicates(
    slot_count: usize,
    present_slots: usize,
) -> Result<ReducedBoundaryPredicates, ControlProfilePartitionError> {
    if slot_count == 0 {
        return Err(ControlProfilePartitionError::ZeroSlots);
    }
    if present_slots > slot_count {
        return Err(ControlProfilePartitionError::PresentExceedsSlots {
            present_slots,
            slot_count,
        });
    }

    let four_present = present_slots
        .checked_mul(4)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let eight_present = present_slots
        .checked_mul(8)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let bitmap_words = slot_count.div_ceil(64);
    let hybrid_boundary = bitmap_words
        .checked_mul(3)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
    let sparse_hybrid_rhs = slot_count
        .checked_add(hybrid_boundary)
        .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;

    Ok(ReducedBoundaryPredicates {
        sparse_le_dense: four_present <= slot_count,
        sparse_le_hybrid: eight_present <= sparse_hybrid_rhs,
        hybrid_le_dense: hybrid_boundary <= slot_count,
    })
}

/// Exact result summary for a bounded exhaustive scan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExhaustivePartitionSummary {
    pub cases: u64,
    pub sparse_minimum_cases: u64,
    pub dense_minimum_cases: u64,
    pub hybrid_minimum_cases: u64,
    pub tied_minimum_cases: u64,
}

/// Exhaustively compare direct bit accounting, direct Boolean comparisons and
/// reduced algebraic predicates over every `1..=max_slots` slot count and
/// every valid `0..=slot_count` present count.
///
/// # Errors
///
/// Returns the first exact mismatch or an input/accounting error.
pub fn exhaustive_partition(
    max_slots: usize,
) -> Result<ExhaustivePartitionSummary, ControlProfilePartitionError> {
    if max_slots == 0 {
        return Err(ControlProfilePartitionError::ZeroSlots);
    }

    let mut summary = ExhaustivePartitionSummary::default();
    for slot_count in 1..=max_slots {
        for present_slots in 0..=slot_count {
            let costs = payloads(slot_count, present_slots)?;
            let exact = costs.exact_minimum_mask();
            let predicates = costs.predicate_minimum_mask();
            if exact != predicates {
                return Err(ControlProfilePartitionError::PredicateMismatch {
                    slot_count,
                    present_slots,
                    exact_mask: exact,
                    predicate_mask: predicates,
                });
            }

            let reduced = reduced_predicates(slot_count, present_slots)?;
            if reduced.sparse_le_dense
                != (costs.sparse_w512_bits <= costs.dense_w128_bits)
                || reduced.sparse_le_hybrid
                    != (costs.sparse_w512_bits <= costs.hybrid_w64_boolean_bits)
                || reduced.hybrid_le_dense
                    != (costs.hybrid_w64_boolean_bits <= costs.dense_w128_bits)
            {
                return Err(ControlProfilePartitionError::ReducedBoundaryMismatch {
                    slot_count,
                    present_slots,
                });
            }

            summary.cases = summary
                .cases
                .checked_add(1)
                .ok_or(ControlProfilePartitionError::ArithmeticOverflow)?;
            summary.sparse_minimum_cases += u64::from(exact & PROFILE_SPARSE_W512 != 0);
            summary.dense_minimum_cases += u64::from(exact & PROFILE_DENSE_W128 != 0);
            summary.hybrid_minimum_cases +=
                u64::from(exact & PROFILE_HYBRID_W64_BOOLEAN != 0);
            summary.tied_minimum_cases += u64::from(exact.count_ones() > 1);
        }
    }
    Ok(summary)
}

/// Fail-closed bounded-oracle errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlProfilePartitionError {
    ZeroSlots,
    PresentExceedsSlots {
        present_slots: usize,
        slot_count: usize,
    },
    ArithmeticOverflow,
    PredicateMismatch {
        slot_count: usize,
        present_slots: usize,
        exact_mask: u8,
        predicate_mask: u8,
    },
    ReducedBoundaryMismatch {
        slot_count: usize,
        present_slots: usize,
    },
}

impl std::fmt::Display for ControlProfilePartitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroSlots => f.write_str("control-profile partition requires slot_count > 0"),
            Self::PresentExceedsSlots {
                present_slots,
                slot_count,
            } => write!(
                f,
                "present slot count {present_slots} exceeds physical slot count {slot_count}"
            ),
            Self::ArithmeticOverflow => {
                f.write_str("control-profile payload accounting overflowed usize")
            }
            Self::PredicateMismatch {
                slot_count,
                present_slots,
                exact_mask,
                predicate_mask,
            } => write!(
                f,
                "Boolean argmin predicate mismatch at slots={slot_count} present={present_slots}: exact={exact_mask:#05b}, predicate={predicate_mask:#05b}"
            ),
            Self::ReducedBoundaryMismatch {
                slot_count,
                present_slots,
            } => write!(
                f,
                "reduced profile boundary mismatch at slots={slot_count} present={present_slots}"
            ),
        }
    }
}

impl std::error::Error for ControlProfilePartitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_structural_regimes_preserve_ties_and_non_universality() {
        let tiny = payloads(1, 1).unwrap();
        assert_eq!(tiny.sparse_w512_bits, 512);
        assert_eq!(tiny.dense_w128_bits, 128);
        assert_eq!(tiny.hybrid_w64_boolean_bits, 256);
        assert_eq!(tiny.exact_minimum_mask(), PROFILE_DENSE_W128);

        let dense64 = payloads(64, 64).unwrap();
        assert_eq!(dense64.hybrid_w64_boolean_bits, 4288);
        assert_eq!(dense64.dense_w128_bits, 8192);
        assert_eq!(dense64.sparse_w512_bits, 32768);
        assert_eq!(
            dense64.exact_minimum_mask(),
            PROFILE_HYBRID_W64_BOOLEAN
        );

        let sparse64 = payloads(64, 1).unwrap();
        assert_eq!(sparse64.sparse_w512_bits, 512);
        assert_eq!(sparse64.hybrid_w64_boolean_bits, 4288);
        assert_eq!(sparse64.dense_w128_bits, 8192);
        assert_eq!(sparse64.exact_minimum_mask(), PROFILE_SPARSE_W512);

        // At 3 dense slots, dense W128 and the hybrid profile tie exactly.
        let tie = payloads(3, 3).unwrap();
        assert_eq!(tie.dense_w128_bits, 384);
        assert_eq!(tie.hybrid_w64_boolean_bits, 384);
        assert_eq!(
            tie.exact_minimum_mask(),
            PROFILE_DENSE_W128 | PROFILE_HYBRID_W64_BOOLEAN
        );
    }

    #[test]
    fn exhaustive_4096_slot_partition_matches_direct_accounting() {
        let summary = exhaustive_partition(DEFAULT_EXHAUSTIVE_MAX_SLOTS).unwrap();
        let expected_cases = (1..=DEFAULT_EXHAUSTIVE_MAX_SLOTS)
            .map(|slots| u64::try_from(slots + 1).unwrap())
            .sum::<u64>();
        assert_eq!(summary.cases, expected_cases);
        assert!(summary.sparse_minimum_cases > 0);
        assert!(summary.dense_minimum_cases > 0);
        assert!(summary.hybrid_minimum_cases > 0);
        assert!(summary.tied_minimum_cases > 0);
    }

    #[test]
    fn invalid_domain_fails_closed() {
        assert_eq!(
            payloads(0, 0),
            Err(ControlProfilePartitionError::ZeroSlots)
        );
        assert_eq!(
            payloads(4, 5),
            Err(ControlProfilePartitionError::PresentExceedsSlots {
                present_slots: 5,
                slot_count: 4,
            })
        );
    }
}
