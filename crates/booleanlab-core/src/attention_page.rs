//! Conservative page-level metadata for BL-4.5.1 Boolean KV routing.
//!
//! A [`PageEnvelope`] records which signature bits are unanimously one and
//! unanimously zero across every key in a declared KV page. For a query, those
//! unanimous bits yield an exact lower bound on the Hamming distance to every
//! key in the page. If that lower bound already exceeds the pair-level routing
//! threshold, the whole page can be rejected without evaluating its individual
//! key signatures.
//!
//! This is a correctness primitive, not a hardware-performance claim.

use core::fmt;

use crate::{AttentionRouterError, BitSignature};

/// Boolean metadata summarising one non-empty page of equal-width key signatures.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageEnvelope {
    always_one: BitSignature,
    always_zero: BitSignature,
    key_count: usize,
}

impl PageEnvelope {
    /// Build a conservative envelope from every key signature in one page.
    ///
    /// `always_one` contains bits set in every key. `always_zero` contains bits
    /// clear in every key. Bits that vary across the page are clear in both.
    ///
    /// # Errors
    ///
    /// Returns [`PageEnvelopeError::EmptyPage`] for an empty page and propagates
    /// width mismatches or malformed packed representation errors.
    pub fn from_keys(keys: &[BitSignature]) -> Result<Self, PageEnvelopeError> {
        let first = keys.first().ok_or(PageEnvelopeError::EmptyPage)?;
        let bit_len = first.bit_len();
        let word_len = first.words().len();

        let mut always_one = vec![u64::MAX; word_len];
        let mut always_zero = vec![u64::MAX; word_len];

        for key in keys {
            if key.bit_len() != bit_len {
                return Err(PageEnvelopeError::Router(
                    AttentionRouterError::SignatureWidthMismatch {
                        left: bit_len,
                        right: key.bit_len(),
                    },
                ));
            }

            for ((one, zero), &word) in always_one
                .iter_mut()
                .zip(always_zero.iter_mut())
                .zip(key.words())
            {
                *one &= word;
                *zero &= !word;
            }
        }

        let tail_bits = bit_len % 64;
        if tail_bits != 0 {
            let tail_mask = (1_u64 << tail_bits) - 1;
            if let Some(last) = always_one.last_mut() {
                *last &= tail_mask;
            }
            if let Some(last) = always_zero.last_mut() {
                *last &= tail_mask;
            }
        }

        Ok(Self {
            always_one: BitSignature::from_words(bit_len, always_one)
                .map_err(PageEnvelopeError::Router)?,
            always_zero: BitSignature::from_words(bit_len, always_zero)
                .map_err(PageEnvelopeError::Router)?,
            key_count: keys.len(),
        })
    }

    #[must_use]
    pub const fn bit_len(&self) -> usize {
        self.always_one.bit_len()
    }

    #[must_use]
    pub const fn key_count(&self) -> usize {
        self.key_count
    }

    #[must_use]
    pub const fn always_one(&self) -> &BitSignature {
        &self.always_one
    }

    #[must_use]
    pub const fn always_zero(&self) -> &BitSignature {
        &self.always_zero
    }
}

/// Fail-closed errors for Boolean KV page envelopes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageEnvelopeError {
    EmptyPage,
    Router(AttentionRouterError),
}

impl fmt::Display for PageEnvelopeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for PageEnvelopeError {}

impl From<AttentionRouterError> for PageEnvelopeError {
    fn from(value: AttentionRouterError) -> Self {
        Self::Router(value)
    }
}

/// Compute a guaranteed lower bound on query-to-key Hamming distance for a page.
///
/// A mismatch is guaranteed when the query has `1` at a page-unanimous-zero bit,
/// or `0` at a page-unanimous-one bit. Variable page bits contribute nothing to
/// the lower bound.
///
/// # Errors
///
/// Returns a width mismatch when the query and page metadata disagree.
pub fn page_hamming_lower_bound(
    query: &BitSignature,
    page: &PageEnvelope,
) -> Result<u64, PageEnvelopeError> {
    if query.bit_len() != page.bit_len() {
        return Err(PageEnvelopeError::Router(
            AttentionRouterError::SignatureWidthMismatch {
                left: query.bit_len(),
                right: page.bit_len(),
            },
        ));
    }

    Ok(query
        .words()
        .iter()
        .zip(page.always_zero.words())
        .zip(page.always_one.words())
        .map(|((&query_word, &always_zero), &always_one)| {
            let guaranteed_mismatch = (query_word & always_zero) | ((!query_word) & always_one);
            u64::from(guaranteed_mismatch.count_ones())
        })
        .sum())
}

/// Conservative page admission for a pair-level Hamming threshold.
///
/// If the returned value is `false`, every key in the page is guaranteed to
/// exceed `max_distance`; rejecting the page cannot remove a key that the
/// pair-level Hamming rule would have admitted. `true` means only that the page
/// cannot yet be safely rejected and may need finer evaluation.
///
/// # Errors
///
/// Rejects thresholds wider than the declared signature and propagates width
/// mismatches.
pub fn admit_page_by_hamming_lower_bound(
    query: &BitSignature,
    page: &PageEnvelope,
    max_distance: u64,
) -> Result<bool, PageEnvelopeError> {
    if max_distance > u64::try_from(query.bit_len()).unwrap_or(u64::MAX) {
        return Err(PageEnvelopeError::Router(
            AttentionRouterError::ThresholdExceedsSignatureWidth {
                threshold: max_distance,
                bit_len: query.bit_len(),
            },
        ));
    }

    Ok(page_hamming_lower_bound(query, page)? <= max_distance)
}

/// Apply conservative page admission while preserving page order.
///
/// # Errors
///
/// Propagates fail-closed validation from each page.
pub fn hamming_page_admission_row(
    query: &BitSignature,
    pages: &[PageEnvelope],
    max_distance: u64,
) -> Result<Vec<bool>, PageEnvelopeError> {
    pages
        .iter()
        .map(|page| admit_page_by_hamming_lower_bound(query, page, max_distance))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{BitSignature, hamming_distance};

    use super::{
        PageEnvelope, PageEnvelopeError, admit_page_by_hamming_lower_bound,
        hamming_page_admission_row, page_hamming_lower_bound,
    };

    fn signature(bits: &[bool]) -> BitSignature {
        BitSignature::from_bits(bits).unwrap()
    }

    #[test]
    fn envelope_records_only_unanimous_bits() {
        let keys = [
            signature(&[true, false, true, false]),
            signature(&[true, true, false, false]),
            signature(&[true, false, false, false]),
        ];
        let page = PageEnvelope::from_keys(&keys).unwrap();

        assert_eq!(page.key_count(), 3);
        assert_eq!(page.always_one().words(), &[0b0001]);
        assert_eq!(page.always_zero().words(), &[0b1000]);
    }

    #[test]
    fn lower_bound_is_no_greater_than_every_exact_key_distance() {
        let query = signature(&[true, false, true, false]);
        let keys = [
            signature(&[false, true, true, false]),
            signature(&[false, true, false, false]),
            signature(&[false, true, true, true]),
        ];
        let page = PageEnvelope::from_keys(&keys).unwrap();
        let lower_bound = page_hamming_lower_bound(&query, &page).unwrap();

        assert_eq!(lower_bound, 2);
        for key in &keys {
            assert!(lower_bound <= hamming_distance(&query, key).unwrap());
        }
    }

    #[test]
    fn page_rejection_is_conservative_relative_to_pair_rule() {
        let query = signature(&[true, false, true, false]);
        let keys = [
            signature(&[false, true, true, false]),
            signature(&[false, true, false, false]),
        ];
        let page = PageEnvelope::from_keys(&keys).unwrap();

        assert_eq!(
            admit_page_by_hamming_lower_bound(&query, &page, 1),
            Ok(false)
        );
        assert_eq!(
            admit_page_by_hamming_lower_bound(&query, &page, 2),
            Ok(true)
        );
    }

    #[test]
    fn page_row_preserves_declared_page_order() {
        let query = signature(&[true, false, true, false]);
        let near = PageEnvelope::from_keys(&[signature(&[true, false, true, false])]).unwrap();
        let far = PageEnvelope::from_keys(&[signature(&[false, true, false, true])]).unwrap();

        assert_eq!(
            hamming_page_admission_row(&query, &[near, far], 1).unwrap(),
            vec![true, false]
        );
    }

    #[test]
    fn construction_and_query_validation_fail_closed() {
        assert_eq!(
            PageEnvelope::from_keys(&[]),
            Err(PageEnvelopeError::EmptyPage)
        );

        let mismatched = [signature(&[true, false]), signature(&[true, false, true])];
        assert!(matches!(
            PageEnvelope::from_keys(&mismatched),
            Err(PageEnvelopeError::Router(_))
        ));

        let page = PageEnvelope::from_keys(&[signature(&[true, false])]).unwrap();
        let query = signature(&[true, false, true]);
        assert!(matches!(
            page_hamming_lower_bound(&query, &page),
            Err(PageEnvelopeError::Router(_))
        ));
    }
}
