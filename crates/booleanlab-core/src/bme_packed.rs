//! Packed-u64 CPU correctness baselines for Boolean Matrix Equation cells.
//!
//! These functions preserve the semantics of the canonical scalar baselines
//! while operating on explicitly bit-packed inputs. They are correctness and
//! representation baselines only: no SIMD, throughput, latency, bandwidth or
//! end-to-end speedup claim follows from using `u64` words.

use core::fmt;

/// Maximum number of result cells materialized by one packed matrix product.
pub const MAX_PACKED_MATRIX_OUTPUT_CELLS: usize = 1_000_000;
/// Maximum upper-bound count of packed-word evaluations in one matrix product.
pub const MAX_PACKED_MATRIX_WORD_EVALUATIONS: usize = 64_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackedBmeError {
    ZeroWidth,
    EmptyMatrix,
    WordCountMismatch {
        required: usize,
        left: usize,
        right: usize,
    },
    ThresholdOutOfRange {
        threshold: usize,
        width: usize,
    },
    ResourceSizeOverflow,
    OutputCellLimitExceeded {
        requested: usize,
        max: usize,
    },
    WorkLimitExceeded {
        requested_word_evaluations: usize,
        max: usize,
    },
}

impl fmt::Display for PackedBmeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("packed BME width must be non-zero"),
            Self::EmptyMatrix => {
                formatter.write_str("packed BME row and column collections must be non-empty")
            }
            Self::WordCountMismatch {
                required,
                left,
                right,
            } => write!(
                formatter,
                "packed BME requires {required} words for the declared width; left={left}, right={right}"
            ),
            Self::ThresholdOutOfRange { threshold, width } => write!(
                formatter,
                "packed BME threshold {threshold} exceeds declared width {width}"
            ),
            Self::ResourceSizeOverflow => {
                formatter.write_str("packed BME resource-size arithmetic overflowed")
            }
            Self::OutputCellLimitExceeded { requested, max } => write!(
                formatter,
                "packed BME requested {requested} output cells, exceeding the limit {max}"
            ),
            Self::WorkLimitExceeded {
                requested_word_evaluations,
                max,
            } => write!(
                formatter,
                "packed BME requested an upper bound of {requested_word_evaluations} word evaluations, exceeding the limit {max}"
            ),
        }
    }
}

impl std::error::Error for PackedBmeError {}

fn required_words(width: usize) -> Result<usize, PackedBmeError> {
    if width == 0 {
        return Err(PackedBmeError::ZeroWidth);
    }
    Ok(width.div_ceil(u64::BITS as usize))
}

fn validate(left: &[u64], right: &[u64], width: usize) -> Result<usize, PackedBmeError> {
    let required = required_words(width)?;
    if left.len() != required || right.len() != required {
        return Err(PackedBmeError::WordCountMismatch {
            required,
            left: left.len(),
            right: right.len(),
        });
    }
    Ok(required)
}

fn tail_mask(width: usize) -> u64 {
    let tail = width % u64::BITS as usize;
    if tail == 0 {
        u64::MAX
    } else {
        (1_u64 << tail) - 1
    }
}

fn valid_word_mask(index: usize, words: usize, width: usize) -> u64 {
    if index + 1 == words {
        tail_mask(width)
    } else {
        u64::MAX
    }
}

fn packed_product_with<T, F>(
    left_rows: &[Vec<u64>],
    right_columns: &[Vec<u64>],
    inner_width: usize,
    mut cell: F,
) -> Result<Vec<Vec<T>>, PackedBmeError>
where
    F: FnMut(&[u64], &[u64], usize) -> Result<T, PackedBmeError>,
{
    if left_rows.is_empty() || right_columns.is_empty() {
        return Err(PackedBmeError::EmptyMatrix);
    }
    let words = required_words(inner_width)?;
    let output_cells = left_rows
        .len()
        .checked_mul(right_columns.len())
        .ok_or(PackedBmeError::ResourceSizeOverflow)?;
    if output_cells > MAX_PACKED_MATRIX_OUTPUT_CELLS {
        return Err(PackedBmeError::OutputCellLimitExceeded {
            requested: output_cells,
            max: MAX_PACKED_MATRIX_OUTPUT_CELLS,
        });
    }
    let word_evaluations = output_cells
        .checked_mul(words)
        .ok_or(PackedBmeError::ResourceSizeOverflow)?;
    if word_evaluations > MAX_PACKED_MATRIX_WORD_EVALUATIONS {
        return Err(PackedBmeError::WorkLimitExceeded {
            requested_word_evaluations: word_evaluations,
            max: MAX_PACKED_MATRIX_WORD_EVALUATIONS,
        });
    }

    let mut output = Vec::with_capacity(left_rows.len());
    for left_row in left_rows {
        let mut output_row = Vec::with_capacity(right_columns.len());
        for right_column in right_columns {
            output_row.push(cell(left_row, right_column, inner_width)?);
        }
        output.push(output_row);
    }
    Ok(output)
}

/// Packed equivalent of OR over pairwise AND.
///
/// Bits above `width` in the last word are ignored rather than becoming
/// accidental logical input.
///
/// # Errors
///
/// Fails for zero width or if either packed slice does not contain exactly the
/// number of words implied by `width`.
pub fn packed_or_and_cell(
    left: &[u64],
    right: &[u64],
    width: usize,
) -> Result<bool, PackedBmeError> {
    let words = validate(left, right, width)?;
    Ok(left
        .iter()
        .zip(right)
        .enumerate()
        .any(|(index, (&a, &b))| (a & b & valid_word_mask(index, words, width)) != 0))
}

/// Packed equivalent of XOR/parity over pairwise AND.
///
/// # Errors
///
/// Fails for zero width or an inconsistent packed representation.
pub fn packed_xor_and_cell(
    left: &[u64],
    right: &[u64],
    width: usize,
) -> Result<bool, PackedBmeError> {
    let words = validate(left, right, width)?;
    let parity = left
        .iter()
        .zip(right)
        .enumerate()
        .fold(0_u32, |acc, (index, (&a, &b))| {
            acc ^ ((a & b & valid_word_mask(index, words, width)).count_ones() & 1)
        });
    Ok(parity != 0)
}

/// Packed exact XNOR/equality match count over the declared logical width.
///
/// # Errors
///
/// Fails for zero width or an inconsistent packed representation.
pub fn packed_xnor_popcount_cell(
    left: &[u64],
    right: &[u64],
    width: usize,
) -> Result<usize, PackedBmeError> {
    let words = validate(left, right, width)?;
    let matches = left
        .iter()
        .zip(right)
        .enumerate()
        .map(|(index, (&a, &b))| {
            (!(a ^ b) & valid_word_mask(index, words, width)).count_ones() as usize
        })
        .sum();
    Ok(matches)
}

/// Packed thresholded XNOR-popcount reference cell.
///
/// # Errors
///
/// Fails for zero width, inconsistent packed storage, or a threshold larger
/// than the declared logical width.
pub fn packed_thresholded_xnor_cell(
    left: &[u64],
    right: &[u64],
    width: usize,
    threshold: usize,
) -> Result<bool, PackedBmeError> {
    validate(left, right, width)?;
    if threshold > width {
        return Err(PackedBmeError::ThresholdOutOfRange { threshold, width });
    }
    Ok(packed_xnor_popcount_cell(left, right, width)? >= threshold)
}

/// Packed OR-AND matrix product over prepacked left rows and right columns.
///
/// `left_rows` must contain one packed bit-vector for each logical left matrix
/// row. `right_columns` must contain one packed bit-vector for each logical
/// right matrix column. Every vector uses the same declared `inner_width`.
/// This explicit row/column contract avoids repacking or transposing inside the
/// correctness baseline.
///
/// # Errors
///
/// Fails when either collection is empty, `inner_width` is zero, any packed
/// row/column has the wrong word count, or explicit output/work limits would be
/// exceeded before allocation or evaluation begins.
pub fn packed_or_and_product_rows_columns(
    left_rows: &[Vec<u64>],
    right_columns: &[Vec<u64>],
    inner_width: usize,
) -> Result<Vec<Vec<bool>>, PackedBmeError> {
    packed_product_with(left_rows, right_columns, inner_width, packed_or_and_cell)
}

/// Packed GF(2) XOR-AND matrix product over prepacked left rows and right columns.
///
/// # Errors
///
/// Fails when either collection is empty, `inner_width` is zero, any packed
/// row/column has the wrong word count, or explicit output/work limits would be
/// exceeded.
pub fn packed_xor_and_product_rows_columns(
    left_rows: &[Vec<u64>],
    right_columns: &[Vec<u64>],
    inner_width: usize,
) -> Result<Vec<Vec<bool>>, PackedBmeError> {
    packed_product_with(left_rows, right_columns, inner_width, packed_xor_and_cell)
}

/// Packed XNOR-popcount score matrix over prepacked left rows and right columns.
///
/// # Errors
///
/// Fails when either collection is empty, `inner_width` is zero, any packed
/// row/column has the wrong word count, or explicit output/work limits would be
/// exceeded.
pub fn packed_xnor_popcount_product_rows_columns(
    left_rows: &[Vec<u64>],
    right_columns: &[Vec<u64>],
    inner_width: usize,
) -> Result<Vec<Vec<usize>>, PackedBmeError> {
    packed_product_with(
        left_rows,
        right_columns,
        inner_width,
        packed_xnor_popcount_cell,
    )
}

/// Packed thresholded XNOR-popcount matrix product over prepacked rows/columns.
///
/// # Errors
///
/// Fails when either collection is empty, `inner_width` is zero, any packed
/// row/column has the wrong word count, `threshold` exceeds `inner_width`, or
/// explicit output/work limits would be exceeded.
pub fn packed_thresholded_xnor_product_rows_columns(
    left_rows: &[Vec<u64>],
    right_columns: &[Vec<u64>],
    inner_width: usize,
    threshold: usize,
) -> Result<Vec<Vec<bool>>, PackedBmeError> {
    if threshold > inner_width {
        return Err(PackedBmeError::ThresholdOutOfRange {
            threshold,
            width: inner_width,
        });
    }
    packed_product_with(
        left_rows,
        right_columns,
        inner_width,
        |left, right, width| packed_thresholded_xnor_cell(left, right, width, threshold),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bme::{
        or_and_cell, or_and_product, thresholded_xnor_cell, thresholded_xnor_product,
        xnor_popcount_cell, xnor_popcount_product, xor_and_cell, xor_and_product,
    };

    fn pack(bits: &[bool]) -> Vec<u64> {
        let mut words = vec![0_u64; bits.len().div_ceil(u64::BITS as usize)];
        for (index, bit) in bits.iter().copied().enumerate() {
            if bit {
                words[index / u64::BITS as usize] |= 1_u64 << (index % u64::BITS as usize);
            }
        }
        words
    }

    fn fixture(width: usize) -> (Vec<bool>, Vec<bool>) {
        let left = (0..width)
            .map(|index| index % 3 != 1 && index % 7 != 4)
            .collect();
        let right = (0..width)
            .map(|index| index % 5 <= 1 || index % 11 == 7)
            .collect();
        (left, right)
    }

    fn matrix_fixture(inner: usize) -> (Vec<Vec<bool>>, Vec<Vec<bool>>) {
        let left = (0..3)
            .map(|row| {
                (0..inner)
                    .map(|index| (index + row * 3) % 5 <= 1 || (index + row) % 11 == 7)
                    .collect()
            })
            .collect();
        let right = (0..inner)
            .map(|row| {
                (0..4)
                    .map(|col| (row + col * 5) % 7 <= 2 && (row + col) % 3 != 1)
                    .collect()
            })
            .collect();
        (left, right)
    }

    fn pack_right_columns(right: &[Vec<bool>]) -> Vec<Vec<u64>> {
        let cols = right[0].len();
        (0..cols)
            .map(|col| {
                let bits: Vec<_> = right.iter().map(|row| row[col]).collect();
                pack(&bits)
            })
            .collect()
    }

    #[test]
    fn packed_cells_match_scalar_oracles_across_word_boundaries() {
        for width in [1, 7, 63, 64, 65, 127, 128, 129] {
            let (left, right) = fixture(width);
            let packed_left = pack(&left);
            let packed_right = pack(&right);

            assert_eq!(
                packed_or_and_cell(&packed_left, &packed_right, width),
                Ok(or_and_cell(&left, &right).unwrap())
            );
            assert_eq!(
                packed_xor_and_cell(&packed_left, &packed_right, width),
                Ok(xor_and_cell(&left, &right).unwrap())
            );
            assert_eq!(
                packed_xnor_popcount_cell(&packed_left, &packed_right, width),
                Ok(xnor_popcount_cell(&left, &right).unwrap())
            );
            let threshold = width / 2;
            assert_eq!(
                packed_thresholded_xnor_cell(&packed_left, &packed_right, width, threshold,),
                Ok(thresholded_xnor_cell(&left, &right, threshold).unwrap())
            );
        }
    }

    #[test]
    fn packed_matrix_products_match_scalar_oracles_across_word_boundaries() {
        for inner in [1, 63, 64, 65, 127, 128, 129] {
            let (left, right) = matrix_fixture(inner);
            let packed_left: Vec<_> = left.iter().map(|row| pack(row)).collect();
            let packed_right_columns = pack_right_columns(&right);
            let threshold = inner / 2;

            assert_eq!(
                packed_or_and_product_rows_columns(&packed_left, &packed_right_columns, inner)
                    .unwrap(),
                or_and_product(&left, &right).unwrap()
            );
            assert_eq!(
                packed_xor_and_product_rows_columns(&packed_left, &packed_right_columns, inner)
                    .unwrap(),
                xor_and_product(&left, &right).unwrap()
            );
            assert_eq!(
                packed_xnor_popcount_product_rows_columns(
                    &packed_left,
                    &packed_right_columns,
                    inner,
                )
                .unwrap(),
                xnor_popcount_product(&left, &right).unwrap()
            );
            assert_eq!(
                packed_thresholded_xnor_product_rows_columns(
                    &packed_left,
                    &packed_right_columns,
                    inner,
                    threshold,
                )
                .unwrap(),
                thresholded_xnor_product(&left, &right, threshold).unwrap()
            );
        }
    }

    #[test]
    fn unused_tail_bits_never_change_declared_semantics() {
        let width = 65;
        let (left, right) = fixture(width);
        let mut packed_left = pack(&left);
        let mut packed_right = pack(&right);
        let expected = packed_xnor_popcount_cell(&packed_left, &packed_right, width).unwrap();

        packed_left[1] |= !1_u64;
        packed_right[1] |= !1_u64;

        assert_eq!(
            packed_xnor_popcount_cell(&packed_left, &packed_right, width),
            Ok(expected)
        );
    }

    #[test]
    fn matrix_product_limits_fail_before_cartesian_allocation() {
        let left = vec![vec![0_u64]; 1_001];
        let right = vec![vec![0_u64]; 1_000];
        assert_eq!(
            packed_or_and_product_rows_columns(&left, &right, 1),
            Err(PackedBmeError::OutputCellLimitExceeded {
                requested: 1_001_000,
                max: MAX_PACKED_MATRIX_OUTPUT_CELLS,
            })
        );

        let excessive_words = MAX_PACKED_MATRIX_WORD_EVALUATIONS + 1;
        let excessive_width = excessive_words
            .checked_mul(u64::BITS as usize)
            .expect("test width arithmetic must fit usize");
        assert_eq!(
            packed_or_and_product_rows_columns(&[vec![0]], &[vec![0]], excessive_width),
            Err(PackedBmeError::WorkLimitExceeded {
                requested_word_evaluations: excessive_words,
                max: MAX_PACKED_MATRIX_WORD_EVALUATIONS,
            })
        );
    }

    #[test]
    fn malformed_packed_inputs_fail_closed() {
        assert_eq!(
            packed_or_and_cell(&[], &[], 0),
            Err(PackedBmeError::ZeroWidth)
        );
        assert_eq!(
            packed_or_and_cell(&[0], &[0, 0], 65),
            Err(PackedBmeError::WordCountMismatch {
                required: 2,
                left: 1,
                right: 2,
            })
        );
        assert_eq!(
            packed_thresholded_xnor_cell(&[0], &[0], 64, 65),
            Err(PackedBmeError::ThresholdOutOfRange {
                threshold: 65,
                width: 64,
            })
        );
    }

    #[test]
    fn malformed_packed_matrix_inputs_fail_closed() {
        assert_eq!(
            packed_or_and_product_rows_columns(&[], &[vec![0]], 1),
            Err(PackedBmeError::EmptyMatrix)
        );
        assert_eq!(
            packed_xor_and_product_rows_columns(&[vec![0]], &[vec![0]], 65),
            Err(PackedBmeError::WordCountMismatch {
                required: 2,
                left: 1,
                right: 1,
            })
        );
        assert_eq!(
            packed_thresholded_xnor_product_rows_columns(&[vec![0]], &[vec![0]], 64, 65),
            Err(PackedBmeError::ThresholdOutOfRange {
                threshold: 65,
                width: 64,
            })
        );
    }
}
