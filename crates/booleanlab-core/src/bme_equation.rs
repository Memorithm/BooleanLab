//! Typed dispatch for the canonical Boolean Matrix Equation baselines.
//!
//! This module names the already-qualified reference families without opening
//! an unconstrained equation search surface. It is intended as a stable bridge
//! for later Forge/FLAT/KVLab comparisons.

use crate::bme::{
    BmeError, or_and_product, thresholded_xnor_product, xnor_popcount_product, xor_and_product,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalBmeEquation {
    OrAnd,
    XorAnd,
    XnorPopcount,
    ThresholdedXnor { threshold: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalBmeOutput {
    Boolean(Vec<Vec<bool>>),
    Count(Vec<Vec<usize>>),
}

impl CanonicalBmeEquation {
    /// Evaluate one canonical BME reference family.
    ///
    /// # Errors
    ///
    /// Returns [`BmeError`] when either matrix is empty, ragged, dimensionally
    /// incompatible, or when a threshold exceeds the inner matrix dimension.
    pub fn evaluate(
        self,
        left: &[Vec<bool>],
        right: &[Vec<bool>],
    ) -> Result<CanonicalBmeOutput, BmeError> {
        match self {
            Self::OrAnd => or_and_product(left, right).map(CanonicalBmeOutput::Boolean),
            Self::XorAnd => xor_and_product(left, right).map(CanonicalBmeOutput::Boolean),
            Self::XnorPopcount => {
                xnor_popcount_product(left, right).map(CanonicalBmeOutput::Count)
            }
            Self::ThresholdedXnor { threshold } => thresholded_xnor_product(
                left,
                right,
                threshold,
            )
            .map(CanonicalBmeOutput::Boolean),
        }
    }

    #[must_use]
    pub const fn pair_rule(self) -> &'static str {
        match self {
            Self::OrAnd | Self::XorAnd => "AND",
            Self::XnorPopcount | Self::ThresholdedXnor { .. } => "XNOR",
        }
    }

    #[must_use]
    pub const fn reduction_rule(self) -> &'static str {
        match self {
            Self::OrAnd => "OR",
            Self::XorAnd => "XOR",
            Self::XnorPopcount => "POPCOUNT",
            Self::ThresholdedXnor { .. } => "THRESHOLDED_POPCOUNT",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Vec<Vec<bool>>, Vec<Vec<bool>>) {
        (
            vec![vec![true, false, true], vec![false, true, true]],
            vec![vec![true, false], vec![true, true], vec![false, true]],
        )
    }

    #[test]
    fn typed_dispatch_preserves_all_canonical_results() {
        let (left, right) = fixtures();
        assert_eq!(
            CanonicalBmeEquation::OrAnd.evaluate(&left, &right),
            Ok(CanonicalBmeOutput::Boolean(vec![
                vec![true, true],
                vec![true, true],
            ]))
        );
        assert_eq!(
            CanonicalBmeEquation::XorAnd.evaluate(&left, &right),
            Ok(CanonicalBmeOutput::Boolean(vec![
                vec![true, true],
                vec![true, false],
            ]))
        );
        assert_eq!(
            CanonicalBmeEquation::XnorPopcount.evaluate(&left, &right),
            Ok(CanonicalBmeOutput::Count(vec![vec![1, 1], vec![1, 3]]))
        );
        assert_eq!(
            CanonicalBmeEquation::ThresholdedXnor { threshold: 2 }.evaluate(&left, &right),
            Ok(CanonicalBmeOutput::Boolean(vec![
                vec![false, false],
                vec![false, true],
            ]))
        );
    }

    #[test]
    fn equation_metadata_names_phi_and_reduction_without_claiming_cost() {
        assert_eq!(CanonicalBmeEquation::OrAnd.pair_rule(), "AND");
        assert_eq!(CanonicalBmeEquation::OrAnd.reduction_rule(), "OR");
        assert_eq!(CanonicalBmeEquation::XnorPopcount.pair_rule(), "XNOR");
        assert_eq!(
            CanonicalBmeEquation::ThresholdedXnor { threshold: 2 }.reduction_rule(),
            "THRESHOLDED_POPCOUNT"
        );
    }

    #[test]
    fn dispatch_keeps_fail_closed_threshold_validation() {
        let (left, right) = fixtures();
        assert_eq!(
            CanonicalBmeEquation::ThresholdedXnor { threshold: 4 }.evaluate(&left, &right),
            Err(BmeError::ThresholdOutOfRange {
                threshold: 4,
                width: 3,
            })
        );
    }
}
