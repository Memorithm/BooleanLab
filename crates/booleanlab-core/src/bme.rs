//! Canonical Boolean Matrix Equation cell baselines.
//!
//! These functions implement the preregistered low-complexity reference
//! families for `C_ij = R_k(Phi(A_ik, B_kj))`. They are correctness baselines,
//! not performance claims or novelty candidates.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BmeError {
    EmptyInput,
    LengthMismatch { left: usize, right: usize },
    ThresholdOutOfRange { threshold: usize, width: usize },
}

impl fmt::Display for BmeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => formatter.write_str("BME cell inputs must be non-empty"),
            Self::LengthMismatch { left, right } => {
                write!(formatter, "BME cell width mismatch: {left} != {right}")
            }
            Self::ThresholdOutOfRange { threshold, width } => write!(
                formatter,
                "BME threshold {threshold} exceeds cell width {width}"
            ),
        }
    }
}

impl std::error::Error for BmeError {}

fn validate_pair(left: &[bool], right: &[bool]) -> Result<(), BmeError> {
    if left.is_empty() || right.is_empty() {
        return Err(BmeError::EmptyInput);
    }
    if left.len() != right.len() {
        return Err(BmeError::LengthMismatch {
            left: left.len(),
            right: right.len(),
        });
    }
    Ok(())
}

/// Boolean semiring cell: OR reduction over pairwise AND.
pub fn or_and_cell(left: &[bool], right: &[bool]) -> Result<bool, BmeError> {
    validate_pair(left, right)?;
    Ok(left.iter().zip(right).any(|(&a, &b)| a && b))
}

/// GF(2) cell: XOR/parity reduction over pairwise AND.
pub fn xor_and_cell(left: &[bool], right: &[bool]) -> Result<bool, BmeError> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .fold(false, |parity, (&a, &b)| parity ^ (a && b)))
}

/// Exact number of XNOR/equality matches in one Boolean matrix cell.
pub fn xnor_popcount_cell(left: &[bool], right: &[bool]) -> Result<usize, BmeError> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .filter(|(a, b)| a == b)
        .count())
}

/// Thresholded XNOR-popcount cell.
pub fn thresholded_xnor_cell(
    left: &[bool],
    right: &[bool],
    threshold: usize,
) -> Result<bool, BmeError> {
    validate_pair(left, right)?;
    if threshold > left.len() {
        return Err(BmeError::ThresholdOutOfRange {
            threshold,
            width: left.len(),
        });
    }
    Ok(xnor_popcount_cell(left, right)? >= threshold)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_baselines_match_hand_computed_cells() {
        let left = [true, false, true, true];
        let right = [false, false, true, true];

        assert_eq!(or_and_cell(&left, &right), Ok(true));
        assert_eq!(xor_and_cell(&left, &right), Ok(false));
        assert_eq!(xnor_popcount_cell(&left, &right), Ok(3));
        assert_eq!(thresholded_xnor_cell(&left, &right, 3), Ok(true));
        assert_eq!(thresholded_xnor_cell(&left, &right, 4), Ok(false));
    }

    #[test]
    fn gf2_parity_distinguishes_one_and_two_products() {
        assert_eq!(
            xor_and_cell(&[true, false], &[true, true]),
            Ok(true)
        );
        assert_eq!(xor_and_cell(&[true, true], &[true, true]), Ok(false));
    }

    #[test]
    fn malformed_cells_fail_closed() {
        assert_eq!(or_and_cell(&[], &[]), Err(BmeError::EmptyInput));
        assert_eq!(
            xnor_popcount_cell(&[true], &[true, false]),
            Err(BmeError::LengthMismatch { left: 1, right: 2 })
        );
        assert_eq!(
            thresholded_xnor_cell(&[true], &[true], 2),
            Err(BmeError::ThresholdOutOfRange {
                threshold: 2,
                width: 1,
            })
        );
    }
}
