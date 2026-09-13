//! Exact logical-operation accounting for canonical Boolean Matrix Equations.
//!
//! Counts here are semantic operation counts only, not performance claims.

use core::fmt;
use crate::bme_equation::CanonicalBmeEquation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BmeShape { pub rows: usize, pub inner: usize, pub cols: usize }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BmeLogicalCost {
    pub and_ops: u128,
    pub xnor_ops: u128,
    pub or_reductions: u128,
    pub xor_reductions: u128,
    pub popcount_accumulations: u128,
    pub threshold_comparisons: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmeCostError { ZeroDimension, ArithmeticOverflow, ThresholdOutOfRange { threshold: usize, width: usize } }

impl fmt::Display for BmeCostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => f.write_str("BME cost accounting requires non-zero dimensions"),
            Self::ArithmeticOverflow => f.write_str("BME logical-operation count overflowed u128"),
            Self::ThresholdOutOfRange { threshold, width } => write!(f, "BME threshold {threshold} exceeds inner dimension {width}"),
        }
    }
}
impl std::error::Error for BmeCostError {}

impl BmeShape {
    #[must_use]
    pub const fn new(rows: usize, inner: usize, cols: usize) -> Self { Self { rows, inner, cols } }
    fn cells(self) -> Result<u128, BmeCostError> {
        if self.rows == 0 || self.inner == 0 || self.cols == 0 { return Err(BmeCostError::ZeroDimension); }
        (self.rows as u128).checked_mul(self.cols as u128).ok_or(BmeCostError::ArithmeticOverflow)
    }
    fn pairs(self) -> Result<u128, BmeCostError> { self.cells()?.checked_mul(self.inner as u128).ok_or(BmeCostError::ArithmeticOverflow) }
    fn reductions(self) -> Result<u128, BmeCostError> { self.cells()?.checked_mul(self.inner.saturating_sub(1) as u128).ok_or(BmeCostError::ArithmeticOverflow) }
}

/// Returns exact abstract operation counts for one canonical BME baseline.
pub fn logical_cost(equation: CanonicalBmeEquation, shape: BmeShape) -> Result<BmeLogicalCost, BmeCostError> {
    let pairs = shape.pairs()?;
    let reductions = shape.reductions()?;
    let cells = shape.cells()?;
    Ok(match equation {
        CanonicalBmeEquation::OrAnd => BmeLogicalCost { and_ops: pairs, or_reductions: reductions, ..Default::default() },
        CanonicalBmeEquation::XorAnd => BmeLogicalCost { and_ops: pairs, xor_reductions: reductions, ..Default::default() },
        CanonicalBmeEquation::XnorPopcount => BmeLogicalCost { xnor_ops: pairs, popcount_accumulations: reductions, ..Default::default() },
        CanonicalBmeEquation::ThresholdedXnor { threshold } => {
            if threshold > shape.inner { return Err(BmeCostError::ThresholdOutOfRange { threshold, width: shape.inner }); }
            BmeLogicalCost { xnor_ops: pairs, popcount_accumulations: reductions, threshold_comparisons: cells, ..Default::default() }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_reference_families() {
        let shape = BmeShape::new(2, 3, 4);
        assert_eq!((logical_cost(CanonicalBmeEquation::OrAnd, shape).unwrap().and_ops, logical_cost(CanonicalBmeEquation::OrAnd, shape).unwrap().or_reductions), (24, 16));
        assert_eq!((logical_cost(CanonicalBmeEquation::XorAnd, shape).unwrap().and_ops, logical_cost(CanonicalBmeEquation::XorAnd, shape).unwrap().xor_reductions), (24, 16));
        let xnor = logical_cost(CanonicalBmeEquation::XnorPopcount, shape).unwrap();
        assert_eq!((xnor.xnor_ops, xnor.popcount_accumulations), (24, 16));
        let thresholded = logical_cost(CanonicalBmeEquation::ThresholdedXnor { threshold: 2 }, shape).unwrap();
        assert_eq!((thresholded.xnor_ops, thresholded.popcount_accumulations, thresholded.threshold_comparisons), (24, 16, 8));
    }
    #[test]
    fn invalid_inputs_fail_closed() {
        assert_eq!(logical_cost(CanonicalBmeEquation::OrAnd, BmeShape::new(0, 3, 4)), Err(BmeCostError::ZeroDimension));
        assert_eq!(logical_cost(CanonicalBmeEquation::ThresholdedXnor { threshold: 4 }, BmeShape::new(2, 3, 4)), Err(BmeCostError::ThresholdOutOfRange { threshold: 4, width: 3 }));
    }
}
