//! Packed-u64 CPU correctness baselines for Boolean Matrix Equation cells.
//!
//! These functions preserve the semantics of the canonical scalar baselines
//! while operating on explicitly bit-packed inputs. They are correctness and
//! representation baselines only: no SIMD, throughput, latency, bandwidth or
//! end-to-end speedup claim follows from using `u64` words.

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackedBmeError {
    ZeroWidth,
    WordCountMismatch {
        required: usize,
        left: usize,
        right: usize,
    },
    ThresholdOutOfRange {
        threshold: usize,
        width: usize,
    },
}

impl fmt::Display for PackedBmeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroWidth => formatter.write_str("packed BME width must be non-zero"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bme::{or_and_cell, thresholded_xnor_cell, xnor_popcount_cell, xor_and_cell};

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
                packed_thresholded_xnor_cell(
                    &packed_left,
                    &packed_right,
                    width,
                    threshold,
                ),
                Ok(thresholded_xnor_cell(&left, &right, threshold).unwrap())
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
        packed_right[1] = 1_u64;

        assert_eq!(
            packed_xnor_popcount_cell(&packed_left, &packed_right, width),
            Ok(expected)
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
}
