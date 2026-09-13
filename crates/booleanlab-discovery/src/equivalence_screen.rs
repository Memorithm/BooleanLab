//! BL-13.2.1 exact first-stage equivalence screen.
//!
//! This module answers only whether a candidate is already present in a declared
//! reference corpus under exact truth-table equality or BooleanLab's currently
//! declared output-complement equivalence. It intentionally does not claim EA,
//! CCZ, affine, permutation, circuit, or algebraic equivalence.

use crate::BooleanFunction;

/// The strongest equivalence relation actually demonstrated by this screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenedEquivalence {
    /// Same complete truth table.
    Exact,
    /// Equal only after BooleanLab's declared output-complement canonicalisation.
    OutputComplement,
}

/// A reference-corpus match with the exact index that established it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquivalenceMatch {
    pub reference_index: usize,
    pub relation: ScreenedEquivalence,
}

/// Screens one scalar Boolean function against a bounded reference corpus.
///
/// Exact equality is checked first. If no exact match exists, the function is
/// canonicalised only under `f ~ f XOR 1`, matching the equivalence relation
/// already declared by BooleanLab. Functions with different input widths are
/// incomparable here and are skipped rather than coerced.
///
/// A `None` result means only "not found under these two declared relations in
/// this supplied corpus". It is not evidence of novelty.
#[must_use]
pub fn screen_exact_or_complement(
    candidate: &BooleanFunction,
    reference: &[BooleanFunction],
) -> Option<EquivalenceMatch> {
    if let Some(reference_index) = reference.iter().position(|known| known == candidate) {
        return Some(EquivalenceMatch {
            reference_index,
            relation: ScreenedEquivalence::Exact,
        });
    }

    let canonical = candidate.canonical_under_complement();
    reference
        .iter()
        .enumerate()
        .find_map(|(reference_index, known)| {
            if known.input_bits() != candidate.input_bits() {
                return None;
            }
            (known.canonical_under_complement() == canonical).then_some(EquivalenceMatch {
                reference_index,
                relation: ScreenedEquivalence::OutputComplement,
            })
        })
}

/// Screens a candidate set without collapsing individual provenance.
///
/// The output order exactly matches `candidates`; each entry is independently
/// `Some(match)` or `None`.
#[must_use]
pub fn screen_candidate_set(
    candidates: &[BooleanFunction],
    reference: &[BooleanFunction],
) -> Vec<Option<EquivalenceMatch>> {
    candidates
        .iter()
        .map(|candidate| screen_exact_or_complement(candidate, reference))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(bits: &[u8]) -> BooleanFunction {
        BooleanFunction::new(2, bits.to_vec()).unwrap()
    }

    #[test]
    fn exact_match_has_precedence() {
        let candidate = function(&[0, 1, 1, 0]);
        let reference = vec![function(&[1, 0, 0, 1]), candidate.clone()];
        assert_eq!(
            screen_exact_or_complement(&candidate, &reference),
            Some(EquivalenceMatch {
                reference_index: 1,
                relation: ScreenedEquivalence::Exact,
            })
        );
    }

    #[test]
    fn complement_match_is_reported_but_not_promoted_to_exact() {
        let candidate = function(&[0, 0, 1, 1]);
        let reference = vec![function(&[1, 1, 0, 0])];
        assert_eq!(
            screen_exact_or_complement(&candidate, &reference),
            Some(EquivalenceMatch {
                reference_index: 0,
                relation: ScreenedEquivalence::OutputComplement,
            })
        );
    }

    #[test]
    fn absence_is_not_fabricated_into_an_equivalence() {
        let candidate = function(&[0, 0, 0, 1]);
        let reference = vec![function(&[0, 1, 1, 0])];
        assert_eq!(screen_exact_or_complement(&candidate, &reference), None);
    }

    #[test]
    fn differing_widths_are_not_compared() {
        let candidate = function(&[0, 0, 0, 1]);
        let wider = BooleanFunction::new(3, vec![0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
        assert_eq!(screen_exact_or_complement(&candidate, &[wider]), None);
    }

    #[test]
    fn candidate_set_preserves_input_order() {
        let exact = function(&[0, 1, 1, 0]);
        let absent = function(&[0, 0, 0, 1]);
        let reference = vec![exact.clone()];
        let screened = screen_candidate_set(&[absent, exact], &reference);
        assert_eq!(screened[0], None);
        assert_eq!(
            screened[1],
            Some(EquivalenceMatch {
                reference_index: 0,
                relation: ScreenedEquivalence::Exact,
            })
        );
    }
}
