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

    /// Floating representation for reporting only.
    ///
    /// Scientific density matching should use [`Self::same_density`] or
    /// [`Self::same_cardinality`] rather than comparing floating-point values.
    #[must_use]
    pub fn retained_density(self) -> f64 {
        self.retained as f64 / self.total as f64
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
}

impl fmt::Display for SparsityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SparsityError {}

#[cfg(test)]
mod tests {
    use super::{MaskCardinality, SparsityError};

    #[test]
    fn counts_mask_exactly() {
        let cardinality = MaskCardinality::from_mask(&[true, false, true, true, false]).unwrap();
        assert_eq!(cardinality.retained(), 3);
        assert_eq!(cardinality.dropped(), 2);
        assert_eq!(cardinality.total(), 5);
        assert!((cardinality.retained_density() - 0.6).abs() < f64::EPSILON);
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
}
