//! Exact representation-work accounting for packed Boolean Matrix Equation baselines.
//!
//! These counters describe declared shapes and the current `u64` packed representation. They are
//! not elapsed time, hardware traffic, bandwidth, energy, SIMD utilization, or speedup evidence.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackedBmeWorkEstimate {
    pub output_cells: u128,
    pub words_per_vector: u128,
    pub logical_bit_pairs: u128,
    pub packed_word_evaluations: u128,
    pub left_input_words: u128,
    pub right_input_words: u128,
    pub input_payload_bytes: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackedBmeWorkError {
    ZeroDimension,
    ArithmeticOverflow,
}

impl fmt::Display for PackedBmeWorkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => {
                formatter.write_str("packed BME work accounting requires non-zero dimensions")
            }
            Self::ArithmeticOverflow => {
                formatter.write_str("packed BME work accounting overflowed u128")
            }
        }
    }
}

impl std::error::Error for PackedBmeWorkError {}

fn as_u128(value: usize) -> Result<u128, PackedBmeWorkError> {
    u128::try_from(value).map_err(|_| PackedBmeWorkError::ArithmeticOverflow)
}

/// Returns exact shape-derived work and storage counters for the row/column packed baseline.
///
/// `rows` is the number of packed left rows, `inner_width` is the logical bit width shared by
/// every packed row/column, and `cols` is the number of packed right columns. The result assumes
/// one `u64` word per 64 logical bits, with the final word retained even when partially occupied.
///
/// `input_payload_bytes` counts only the declared packed words for the left-row and right-column
/// collections. It excludes allocator metadata, output storage, cache effects, transfers, and any
/// physical memory traffic.
///
/// # Errors
///
/// Returns [`PackedBmeWorkError::ZeroDimension`] when any dimension is zero and
/// [`PackedBmeWorkError::ArithmeticOverflow`] if an exact counter cannot be represented in `u128`.
pub fn packed_bme_work_estimate(
    rows: usize,
    inner_width: usize,
    cols: usize,
) -> Result<PackedBmeWorkEstimate, PackedBmeWorkError> {
    if rows == 0 || inner_width == 0 || cols == 0 {
        return Err(PackedBmeWorkError::ZeroDimension);
    }

    let rows = as_u128(rows)?;
    let inner_width = as_u128(inner_width)?;
    let cols = as_u128(cols)?;
    let words_per_vector = inner_width
        .checked_add(u128::from(u64::BITS - 1))
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?
        / u128::from(u64::BITS);
    let output_cells = rows
        .checked_mul(cols)
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;
    let logical_bit_pairs = output_cells
        .checked_mul(inner_width)
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;
    let packed_word_evaluations = output_cells
        .checked_mul(words_per_vector)
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;
    let left_input_words = rows
        .checked_mul(words_per_vector)
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;
    let right_input_words = cols
        .checked_mul(words_per_vector)
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;
    let input_payload_bytes = left_input_words
        .checked_add(right_input_words)
        .and_then(|words| words.checked_mul(u128::from(u64::BITS / 8)))
        .ok_or(PackedBmeWorkError::ArithmeticOverflow)?;

    Ok(PackedBmeWorkEstimate {
        output_cells,
        words_per_vector,
        logical_bit_pairs,
        packed_word_evaluations,
        left_input_words,
        right_input_words,
        input_payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_word_boundaries_without_turning_them_into_performance_claims() {
        let at_64 = packed_bme_work_estimate(2, 64, 3).unwrap();
        assert_eq!(at_64.output_cells, 6);
        assert_eq!(at_64.words_per_vector, 1);
        assert_eq!(at_64.logical_bit_pairs, 384);
        assert_eq!(at_64.packed_word_evaluations, 6);
        assert_eq!(at_64.left_input_words, 2);
        assert_eq!(at_64.right_input_words, 3);
        assert_eq!(at_64.input_payload_bytes, 40);

        let at_65 = packed_bme_work_estimate(2, 65, 3).unwrap();
        assert_eq!(at_65.output_cells, 6);
        assert_eq!(at_65.words_per_vector, 2);
        assert_eq!(at_65.logical_bit_pairs, 390);
        assert_eq!(at_65.packed_word_evaluations, 12);
        assert_eq!(at_65.left_input_words, 4);
        assert_eq!(at_65.right_input_words, 6);
        assert_eq!(at_65.input_payload_bytes, 80);
    }

    #[test]
    fn zero_dimensions_fail_closed() {
        for shape in [(0, 64, 3), (2, 0, 3), (2, 64, 0)] {
            assert_eq!(
                packed_bme_work_estimate(shape.0, shape.1, shape.2),
                Err(PackedBmeWorkError::ZeroDimension)
            );
        }
    }
}
