//! Exact, model-agnostic mask accounting for BL-14 sparsity experiments.
//!
//! This module deliberately stops before model execution or performance claims.
//! Its role is to make density matching fail closed so dense, random, magnitude,
//! structured, and Boolean-controlled baselines can be compared at identical
//! retained cardinality before quality or hardware evidence is interpreted.

use core::fmt;

/// Exact retained/total cardinality for one declared sparsity mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaskCardinality {
    retained: usize,
    total: usize,
}

impl MaskCardinality {
    /// Construct an exact cardinality record.
    ///
    /// # Errors
    ///
    /// Returns [`SparsityError::EmptyMask`] when `total` is zero and
    /// [`SparsityError::RetainedExceedsTotal`] when `retained > total`.
    pub fn new(retained: usize, total: usize) -> Result<Self, SparsityError> {
        if total == 0 {
            return Err(SparsityError::EmptyMask);
        }
        if retained > total {
            return Err(SparsityError::RetainedExceedsTotal { retained, total });
        }
        Ok(Self { retained, total })
    }

    /// Count a Boolean keep/drop mask exactly (`true` means retained).
    ///
    /// # Errors
    ///
    /// Returns [`SparsityError::EmptyMask`] when `mask` is empty.
    pub fn from_mask(mask: &[bool]) -> Result<Self, SparsityError> {
        if mask.is_empty() {
            return Err(SparsityError::EmptyMask);
        }
        let retained = mask.iter().filter(|&&keep| keep).count();
        Self::new(retained, mask.len())
    }

    #[must_use]
    pub const fn retained(self) -> usize {
        self.retained
    }

    #[must_use]
    pub const fn total(self) -> usize {
        self.total
    }

    #[must_use]
    pub const fn dropped(self) -> usize {
        self.total - self.retained
    }

    /// Exact cardinality equality: same retained count and same declared width.
    #[must_use]
    pub const fn same_cardinality(self, other: Self) -> bool {
        self.retained == other.retained && self.total == other.total
    }

    /// Exact rational-density equality without floating-point rounding.
    #[must_use]
    pub fn same_density(self, other: Self) -> bool {
        (self.retained as u128) * (other.total as u128)
            == (other.retained as u128) * (self.total as u128)
    }

    /// Require two baselines to use exactly the same declared mask width and
    /// retained cardinality.
    ///
    /// # Errors
    ///
    /// Returns [`SparsityError::MaskWidthMismatch`] when the declared mask
    /// widths differ, or [`SparsityError::RetainedCountMismatch`] when the
    /// widths match but retained cardinalities differ.
    pub fn require_matched_cardinality(self, other: Self) -> Result<(), SparsityError> {
        if self.total != other.total {
            return Err(SparsityError::MaskWidthMismatch {
                left_total: self.total,
                right_total: other.total,
            });
        }
        if self.retained != other.retained {
            return Err(SparsityError::RetainedCountMismatch {
                left_retained: self.retained,
                right_retained: other.retained,
                total: self.total,
            });
        }
        Ok(())
    }
}

/// Exact Boolean mask reconstructed from an explicit set of retained indices.
///
/// This is the common correctness boundary for BL-14 baseline generators:
/// random, magnitude, structured, and Boolean policies may choose indices by
/// different rules, but all must materialize an unambiguous mask before their
/// quality or systems measurements are compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactMask {
    values: Vec<bool>,
    cardinality: MaskCardinality,
}

impl ExactMask {
    /// Build a mask from retained indices.
    ///
    /// Index order has no semantic meaning. Every index must be unique and
    /// strictly smaller than `total`.
    ///
    /// # Errors
    ///
    /// Returns [`SparsityError::EmptyMask`] when `total` is zero,
    /// [`SparsityError::SelectionIndexOutOfRange`] when an index is outside the
    /// declared mask, and [`SparsityError::DuplicateSelectionIndex`] when the
    /// same retained index is supplied more than once.
    pub fn from_retained_indices(
        total: usize,
        retained_indices: &[usize],
    ) -> Result<Self, SparsityError> {
        if total == 0 {
            return Err(SparsityError::EmptyMask);
        }

        let mut values = vec![false; total];
        for &index in retained_indices {
            if index >= total {
                return Err(SparsityError::SelectionIndexOutOfRange { index, total });
            }
            if values[index] {
                return Err(SparsityError::DuplicateSelectionIndex { index });
            }
            values[index] = true;
        }

        let cardinality = MaskCardinality::new(retained_indices.len(), total)?;
        Ok(Self {
            values,
            cardinality,
        })
    }

    #[must_use]
    pub fn as_slice(&self) -> &[bool] {
        &self.values
    }

    #[must_use]
    pub const fn cardinality(&self) -> MaskCardinality {
        self.cardinality
    }
}

/// Fail-closed errors for BL-14 mask accounting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SparsityError {
    EmptyMask,
    RetainedExceedsTotal {
        retained: usize,
        total: usize,
    },
    MaskWidthMismatch {
        left_total: usize,
        right_total: usize,
    },
    RetainedCountMismatch {
        left_retained: usize,
        right_retained: usize,
        total: usize,
    },
    SelectionIndexOutOfRange {
        index: usize,
        total: usize,
    },
    DuplicateSelectionIndex {
        index: usize,
    },
}

impl fmt::Display for SparsityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsityError {}

#[cfg(test)]
mod tests {
    use super::{ExactMask, MaskCardinality, SparsityError};

    #[test]
    fn counts_mask_exactly() {
        let cardinality = MaskCardinality::from_mask(&[true, false, true, true, false]).unwrap();
        assert_eq!(cardinality.retained(), 3);
        assert_eq!(cardinality.dropped(), 2);
        assert_eq!(cardinality.total(), 5);
    }

    #[test]
    fn rejects_empty_and_impossible_masks() {
        assert_eq!(
            MaskCardinality::from_mask(&[]),
            Err(SparsityError::EmptyMask)
        );
        assert_eq!(
            MaskCardinality::new(5, 4),
            Err(SparsityError::RetainedExceedsTotal {
                retained: 5,
                total: 4,
            })
        );
    }

    #[test]
    fn exact_density_does_not_depend_on_float_rounding() {
        let half_small = MaskCardinality::new(1, 2).unwrap();
        let half_large = MaskCardinality::new(3, 6).unwrap();
        let different = MaskCardinality::new(4, 6).unwrap();
        assert!(half_small.same_density(half_large));
        assert!(!half_small.same_density(different));
        assert!(!half_small.same_cardinality(half_large));
    }

    #[test]
    fn matched_baseline_gate_requires_same_width_and_count() {
        let reference = MaskCardinality::new(4, 8).unwrap();
        assert_eq!(
            reference.require_matched_cardinality(MaskCardinality::new(4, 8).unwrap()),
            Ok(())
        );
        assert_eq!(
            reference.require_matched_cardinality(MaskCardinality::new(2, 4).unwrap()),
            Err(SparsityError::MaskWidthMismatch {
                left_total: 8,
                right_total: 4,
            })
        );
        assert_eq!(
            reference.require_matched_cardinality(MaskCardinality::new(3, 8).unwrap()),
            Err(SparsityError::RetainedCountMismatch {
                left_retained: 4,
                right_retained: 3,
                total: 8,
            })
        );
    }

    #[test]
    fn retained_indices_materialize_exact_mask_independent_of_input_order() {
        let left = ExactMask::from_retained_indices(8, &[7, 1, 3]).unwrap();
        let right = ExactMask::from_retained_indices(8, &[1, 3, 7]).unwrap();

        assert_eq!(left, right);
        assert_eq!(
            left.as_slice(),
            &[false, true, false, true, false, false, false, true]
        );
        assert_eq!(left.cardinality(), MaskCardinality::new(3, 8).unwrap());
    }

    #[test]
    fn retained_indices_fail_closed_on_duplicate_and_out_of_range_values() {
        assert_eq!(
            ExactMask::from_retained_indices(8, &[1, 1]),
            Err(SparsityError::DuplicateSelectionIndex { index: 1 })
        );
        assert_eq!(
            ExactMask::from_retained_indices(8, &[8]),
            Err(SparsityError::SelectionIndexOutOfRange { index: 8, total: 8 })
        );
        assert_eq!(
            ExactMask::from_retained_indices(0, &[]),
            Err(SparsityError::EmptyMask)
        );
    }

    #[test]
    fn zero_retained_indices_is_a_valid_all_drop_control() {
        let mask = ExactMask::from_retained_indices(4, &[]).unwrap();
        assert_eq!(mask.as_slice(), &[false, false, false, false]);
        assert_eq!(mask.cardinality(), MaskCardinality::new(0, 4).unwrap());
    }
}
