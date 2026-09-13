//! Exact bit-packed admission primitives for BL-4 Boolean attention experiments.
//!
//! This module deliberately stops at the routing boundary. It does not execute
//! numerical attention and it makes no performance claim. Its purpose is to
//! provide deterministic, auditable candidate selection and exact accounting
//! before FLAT-ATTENTION or another runtime measures end-to-end behaviour.

use core::fmt;

/// Packed Boolean signature used by the BL-4 pre-attention control plane.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BitSignature {
    words: Vec<u64>,
    bit_len: usize,
}

impl BitSignature {
    /// Pack a non-empty Boolean slice into little-endian-in-word `u64` storage.
    ///
    /// Bit `i` is stored in word `i / 64` at position `i % 64`.
    ///
    /// # Errors
    ///
    /// Returns [`AttentionRouterError::EmptySignature`] when `bits` is empty.
    pub fn from_bits(bits: &[bool]) -> Result<Self, AttentionRouterError> {
        if bits.is_empty() {
            return Err(AttentionRouterError::EmptySignature);
        }

        let mut words = vec![0_u64; bits.len().div_ceil(64)];
        for (index, &bit) in bits.iter().enumerate() {
            if bit {
                words[index / 64] |= 1_u64 << (index % 64);
            }
        }

        Ok(Self {
            words,
            bit_len: bits.len(),
        })
    }

    /// Construct a signature from already-packed words.
    ///
    /// Unused high bits in the final word must be zero so equality and Hamming
    /// distance remain canonical.
    ///
    /// # Errors
    ///
    /// Returns an error for zero width, wrong packed word count, or non-zero
    /// padding bits beyond `bit_len`.
    pub fn from_words(bit_len: usize, words: Vec<u64>) -> Result<Self, AttentionRouterError> {
        if bit_len == 0 {
            return Err(AttentionRouterError::EmptySignature);
        }

        let expected_words = bit_len.div_ceil(64);
        if words.len() != expected_words {
            return Err(AttentionRouterError::PackedWordCountMismatch {
                expected: expected_words,
                actual: words.len(),
            });
        }

        let used_tail_bits = bit_len % 64;
        if used_tail_bits != 0 {
            let valid_tail_mask = (1_u64 << used_tail_bits) - 1;
            let last = words.last().copied().unwrap_or_default();
            if last & !valid_tail_mask != 0 {
                return Err(AttentionRouterError::NonZeroPaddingBits);
            }
        }

        Ok(Self { words, bit_len })
    }

    #[must_use]
    pub const fn bit_len(&self) -> usize {
        self.bit_len
    }

    #[must_use]
    pub fn words(&self) -> &[u64] {
        &self.words
    }
}

/// Exact classification counts for one candidate-admission mask against an
/// externally supplied reference-positive mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AdmissionScore {
    total: usize,
    admitted: usize,
    reference_positive: usize,
    true_positive: usize,
    false_positive: usize,
    false_negative: usize,
}

impl AdmissionScore {
    #[must_use]
    pub const fn total(self) -> usize {
        self.total
    }

    #[must_use]
    pub const fn admitted(self) -> usize {
        self.admitted
    }

    #[must_use]
    pub const fn rejected(self) -> usize {
        self.total - self.admitted
    }

    #[must_use]
    pub const fn reference_positive(self) -> usize {
        self.reference_positive
    }

    #[must_use]
    pub const fn true_positive(self) -> usize {
        self.true_positive
    }

    #[must_use]
    pub const fn false_positive(self) -> usize {
        self.false_positive
    }

    #[must_use]
    pub const fn false_negative(self) -> usize {
        self.false_negative
    }
}

/// Fail-closed errors for BL-4 Boolean attention routing primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttentionRouterError {
    EmptySignature,
    PackedWordCountMismatch { expected: usize, actual: usize },
    NonZeroPaddingBits,
    SignatureWidthMismatch { left: usize, right: usize },
    ThresholdExceedsSignatureWidth { threshold: u64, bit_len: usize },
    EmptyAdmissionMask,
    AdmissionMaskLengthMismatch { candidate: usize, reference: usize },
}

impl fmt::Display for AttentionRouterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AttentionRouterError {}

/// Compute exact Hamming distance with XOR + population count.
///
/// # Errors
///
/// Returns [`AttentionRouterError::SignatureWidthMismatch`] when the two
/// signatures do not declare the same number of meaningful bits.
pub fn hamming_distance(
    left: &BitSignature,
    right: &BitSignature,
) -> Result<u64, AttentionRouterError> {
    if left.bit_len != right.bit_len {
        return Err(AttentionRouterError::SignatureWidthMismatch {
            left: left.bit_len,
            right: right.bit_len,
        });
    }

    Ok(left
        .words
        .iter()
        .zip(&right.words)
        .map(|(&lhs, &rhs)| u64::from((lhs ^ rhs).count_ones()))
        .sum())
}

/// Decide whether one Q/K signature pair survives the Boolean front-end.
///
/// A pair is admitted exactly when its Hamming distance is less than or equal
/// to `max_distance`.
///
/// # Errors
///
/// Returns a width mismatch error, or rejects thresholds larger than the
/// declared signature width so experiment configurations fail closed.
pub fn admit_by_hamming(
    query: &BitSignature,
    key: &BitSignature,
    max_distance: u64,
) -> Result<bool, AttentionRouterError> {
    if max_distance > u64::try_from(query.bit_len).unwrap_or(u64::MAX) {
        return Err(AttentionRouterError::ThresholdExceedsSignatureWidth {
            threshold: max_distance,
            bit_len: query.bit_len,
        });
    }

    Ok(hamming_distance(query, key)? <= max_distance)
}

/// Build a deterministic candidate mask for one query against many keys.
///
/// # Errors
///
/// Propagates any signature-width or threshold error encountered while
/// evaluating the row.
pub fn hamming_admission_row(
    query: &BitSignature,
    keys: &[BitSignature],
    max_distance: u64,
) -> Result<Vec<bool>, AttentionRouterError> {
    keys.iter()
        .map(|key| admit_by_hamming(query, key, max_distance))
        .collect()
}

/// Compare a Boolean candidate mask against an external reference mask.
///
/// `true` in `reference` means the exact/dense oracle declares that position
/// important under the experiment's frozen criterion. The returned counts are
/// exact integers; callers may derive density, recall, or false-negative rates
/// without hiding denominator choices in this core layer.
///
/// # Errors
///
/// Returns an error for empty masks or unequal lengths.
pub fn score_admission(
    candidate: &[bool],
    reference: &[bool],
) -> Result<AdmissionScore, AttentionRouterError> {
    if candidate.is_empty() || reference.is_empty() {
        return Err(AttentionRouterError::EmptyAdmissionMask);
    }
    if candidate.len() != reference.len() {
        return Err(AttentionRouterError::AdmissionMaskLengthMismatch {
            candidate: candidate.len(),
            reference: reference.len(),
        });
    }

    let mut admitted = 0_usize;
    let mut reference_positive = 0_usize;
    let mut true_positive = 0_usize;
    let mut false_positive = 0_usize;
    let mut false_negative = 0_usize;

    for (&keep, &oracle_positive) in candidate.iter().zip(reference) {
        admitted += usize::from(keep);
        reference_positive += usize::from(oracle_positive);

        match (keep, oracle_positive) {
            (true, true) => true_positive += 1,
            (true, false) => false_positive += 1,
            (false, true) => false_negative += 1,
            (false, false) => {}
        }
    }

    Ok(AdmissionScore {
        total: candidate.len(),
        admitted,
        reference_positive,
        true_positive,
        false_positive,
        false_negative,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AdmissionScore, AttentionRouterError, BitSignature, admit_by_hamming,
        hamming_admission_row, hamming_distance, score_admission,
    };

    #[test]
    fn packs_bits_canonically_across_word_boundary() {
        let mut bits = vec![false; 65];
        bits[0] = true;
        bits[63] = true;
        bits[64] = true;

        let signature = BitSignature::from_bits(&bits).unwrap();
        assert_eq!(signature.bit_len(), 65);
        assert_eq!(signature.words(), &[0x8000_0000_0000_0001, 1]);
    }

    #[test]
    fn packed_constructor_rejects_non_canonical_tail_bits() {
        assert_eq!(
            BitSignature::from_words(65, vec![0, 2]),
            Err(AttentionRouterError::NonZeroPaddingBits)
        );
        assert_eq!(
            BitSignature::from_words(65, vec![0]),
            Err(AttentionRouterError::PackedWordCountMismatch {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn hamming_distance_is_exact_xor_popcount() {
        let left = BitSignature::from_bits(&[true, false, true, false]).unwrap();
        let right = BitSignature::from_bits(&[true, true, false, false]).unwrap();
        assert_eq!(hamming_distance(&left, &right), Ok(2));
    }

    #[test]
    fn hamming_distance_rejects_width_mismatch() {
        let left = BitSignature::from_bits(&[true, false]).unwrap();
        let right = BitSignature::from_bits(&[true, false, true]).unwrap();
        assert_eq!(
            hamming_distance(&left, &right),
            Err(AttentionRouterError::SignatureWidthMismatch { left: 2, right: 3 })
        );
    }

    #[test]
    fn admission_threshold_is_inclusive_and_fail_closed() {
        let query = BitSignature::from_bits(&[true, false, true, false]).unwrap();
        let key = BitSignature::from_bits(&[true, true, false, false]).unwrap();

        assert_eq!(admit_by_hamming(&query, &key, 1), Ok(false));
        assert_eq!(admit_by_hamming(&query, &key, 2), Ok(true));
        assert_eq!(
            admit_by_hamming(&query, &key, 5),
            Err(AttentionRouterError::ThresholdExceedsSignatureWidth {
                threshold: 5,
                bit_len: 4,
            })
        );
    }

    #[test]
    fn row_admission_preserves_key_order() {
        let query = BitSignature::from_bits(&[true, false, true, false]).unwrap();
        let keys = [
            BitSignature::from_bits(&[true, false, true, false]).unwrap(),
            BitSignature::from_bits(&[true, true, true, false]).unwrap(),
            BitSignature::from_bits(&[false, true, false, true]).unwrap(),
        ];

        assert_eq!(
            hamming_admission_row(&query, &keys, 1).unwrap(),
            vec![true, true, false]
        );
    }

    #[test]
    fn scoring_reports_exact_candidate_oracle_confusion_counts() {
        let score = score_admission(
            &[true, true, false, false, true],
            &[true, false, true, false, true],
        )
        .unwrap();

        assert_eq!(
            score,
            AdmissionScore {
                total: 5,
                admitted: 3,
                reference_positive: 3,
                true_positive: 2,
                false_positive: 1,
                false_negative: 1,
            }
        );
        assert_eq!(score.rejected(), 2);
    }

    #[test]
    fn scoring_rejects_empty_or_misaligned_masks() {
        assert_eq!(
            score_admission(&[], &[]),
            Err(AttentionRouterError::EmptyAdmissionMask)
        );
        assert_eq!(
            score_admission(&[true], &[true, false]),
            Err(AttentionRouterError::AdmissionMaskLengthMismatch {
                candidate: 1,
                reference: 2,
            })
        );
    }
}