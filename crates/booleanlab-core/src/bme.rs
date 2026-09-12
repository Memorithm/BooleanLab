//! Canonical Boolean Matrix Equation baselines.
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
    EmptyMatrix,
    RaggedMatrix,
    MatrixDimensionMismatch { left_cols: usize, right_rows: usize },
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
            Self::EmptyMatrix => formatter.write_str("BME matrices must be non-empty"),
            Self::RaggedMatrix => formatter.write_str("BME matrices must be rectangular"),
            Self::MatrixDimensionMismatch {
                left_cols,
                right_rows,
            } => write!(
                formatter,
                "BME matrix dimension mismatch: left columns {left_cols} != right rows {right_rows}"
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

fn matrix_shape(matrix: &[Vec<bool>]) -> Result<(usize, usize), BmeError> {
    let first = matrix.first().ok_or(BmeError::EmptyMatrix)?;
    if first.is_empty() {
        return Err(BmeError::EmptyMatrix);
    }
    let cols = first.len();
    if matrix.iter().any(|row| row.len() != cols) {
        return Err(BmeError::RaggedMatrix);
    }
    Ok((matrix.len(), cols))
}

fn validate_matrix_pair(
    left: &[Vec<bool>],
    right: &[Vec<bool>],
) -> Result<(usize, usize, usize), BmeError> {
    let (left_rows, left_cols) = matrix_shape(left)?;
    let (right_rows, right_cols) = matrix_shape(right)?;
    if left_cols != right_rows {
        return Err(BmeError::MatrixDimensionMismatch {
            left_cols,
            right_rows,
        });
    }
    Ok((left_rows, left_cols, right_cols))
}

fn matrix_product_with<T, F>(
    left: &[Vec<bool>],
    right: &[Vec<bool>],
    mut cell: F,
) -> Result<Vec<Vec<T>>, BmeError>
where
    F: FnMut(&[bool], &[bool]) -> Result<T, BmeError>,
{
    let (rows, inner, cols) = validate_matrix_pair(left, right)?;
    let mut output = Vec::with_capacity(rows);
    let mut right_column = Vec::with_capacity(inner);

    for left_row in left {
        let mut output_row = Vec::with_capacity(cols);
        for col in 0..cols {
            right_column.clear();
            right_column.extend(right.iter().map(|row| row[col]));
            output_row.push(cell(left_row, &right_column)?);
        }
        output.push(output_row);
    }
    Ok(output)
}

/// Boolean semiring cell: OR reduction over pairwise AND.
///
/// # Errors
///
/// Returns [`BmeError::EmptyInput`] for empty cells and
/// [`BmeError::LengthMismatch`] when the two cell widths differ.
pub fn or_and_cell(left: &[bool], right: &[bool]) -> Result<bool, BmeError> {
    validate_pair(left, right)?;
    Ok(left.iter().zip(right).any(|(&a, &b)| a && b))
}

/// GF(2) cell: XOR/parity reduction over pairwise AND.
///
/// # Errors
///
/// Returns [`BmeError::EmptyInput`] for empty cells and
/// [`BmeError::LengthMismatch`] when the two cell widths differ.
pub fn xor_and_cell(left: &[bool], right: &[bool]) -> Result<bool, BmeError> {
    validate_pair(left, right)?;
    Ok(left
        .iter()
        .zip(right)
        .fold(false, |parity, (&a, &b)| parity ^ (a && b)))
}

/// Exact number of XNOR/equality matches in one Boolean matrix cell.
///
/// # Errors
///
/// Returns [`BmeError::EmptyInput`] for empty cells and
/// [`BmeError::LengthMismatch`] when the two cell widths differ.
pub fn xnor_popcount_cell(left: &[bool], right: &[bool]) -> Result<usize, BmeError> {
    validate_pair(left, right)?;
    Ok(left.iter().zip(right).filter(|(a, b)| a == b).count())
}

/// Thresholded XNOR-popcount cell.
///
/// # Errors
///
/// Returns [`BmeError::EmptyInput`] for empty cells,
/// [`BmeError::LengthMismatch`] when the two cell widths differ, and
/// [`BmeError::ThresholdOutOfRange`] when `threshold` exceeds the cell width.
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

/// Boolean semiring matrix product using the canonical OR-AND cell.
///
/// # Errors
///
/// Returns a matrix-shape error when either input is empty or ragged, or when
/// the left column count differs from the right row count.
pub fn or_and_product(left: &[Vec<bool>], right: &[Vec<bool>]) -> Result<Vec<Vec<bool>>, BmeError> {
    matrix_product_with(left, right, or_and_cell)
}

/// GF(2) matrix product using the canonical XOR-AND cell.
///
/// # Errors
///
/// Returns a matrix-shape error when either input is empty or ragged, or when
/// the left column count differs from the right row count.
pub fn xor_and_product(
    left: &[Vec<bool>],
    right: &[Vec<bool>],
) -> Result<Vec<Vec<bool>>, BmeError> {
    matrix_product_with(left, right, xor_and_cell)
}

/// Matrix of exact XNOR-popcount scores.
///
/// # Errors
///
/// Returns a matrix-shape error when either input is empty or ragged, or when
/// the left column count differs from the right row count.
pub fn xnor_popcount_product(
    left: &[Vec<bool>],
    right: &[Vec<bool>],
) -> Result<Vec<Vec<usize>>, BmeError> {
    matrix_product_with(left, right, xnor_popcount_cell)
}

/// Thresholded XNOR-popcount matrix product.
///
/// # Errors
///
/// Returns a matrix-shape error for malformed matrix inputs and
/// [`BmeError::ThresholdOutOfRange`] when `threshold` exceeds the inner matrix
/// dimension.
pub fn thresholded_xnor_product(
    left: &[Vec<bool>],
    right: &[Vec<bool>],
    threshold: usize,
) -> Result<Vec<Vec<bool>>, BmeError> {
    let (_, inner, _) = validate_matrix_pair(left, right)?;
    if threshold > inner {
        return Err(BmeError::ThresholdOutOfRange {
            threshold,
            width: inner,
        });
    }
    matrix_product_with(left, right, |a, b| thresholded_xnor_cell(a, b, threshold))
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
    fn canonical_matrix_products_match_hand_computed_results() {
        let left = vec![vec![true, false, true], vec![false, true, true]];
        let right = vec![vec![true, false], vec![true, true], vec![false, true]];

        assert_eq!(
            or_and_product(&left, &right),
            Ok(vec![vec![true, true], vec![true, true]])
        );
        assert_eq!(
            xor_and_product(&left, &right),
            Ok(vec![vec![true, true], vec![true, false]])
        );
        assert_eq!(
            xnor_popcount_product(&left, &right),
            Ok(vec![vec![1, 2], vec![2, 2]])
        );
        assert_eq!(
            thresholded_xnor_product(&left, &right, 2),
            Ok(vec![vec![false, true], vec![true, true]])
        );
    }

    #[test]
    fn gf2_parity_distinguishes_one_and_two_products() {
        assert_eq!(xor_and_cell(&[true, false], &[true, true]), Ok(true));
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

    #[test]
    fn malformed_matrices_fail_closed() {
        assert_eq!(or_and_product(&[], &[]), Err(BmeError::EmptyMatrix));
        assert_eq!(
            or_and_product(&[vec![true], vec![true, false]], &[vec![true]]),
            Err(BmeError::RaggedMatrix)
        );
        assert_eq!(
            xor_and_product(&[vec![true, false]], &[vec![true]]),
            Err(BmeError::MatrixDimensionMismatch {
                left_cols: 2,
                right_rows: 1,
            })
        );
        assert_eq!(
            thresholded_xnor_product(&[vec![true]], &[vec![true]], 2),
            Err(BmeError::ThresholdOutOfRange {
                threshold: 2,
                width: 1,
            })
        );
    }
}
