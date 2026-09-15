//! BL-14 adapter from a frozen model-unit mask to bounded exact rule synthesis.
//!
//! The numerical model, predicate extraction, and SEARCH-selected mask remain
//! experiment-owned. This module only asks whether that already-frozen mask is
//! exactly representable by the bounded conjunction language in
//! `booleanlab-core`. HOLDOUT outcomes, task quality, and resource objectives are
//! intentionally absent from the API so they cannot influence synthesis.

use std::fmt;

use booleanlab_core::sparsity_synthesis::{
    ConjunctiveSparsityRule, MAX_SYNTHESIS_PREDICATES, MAX_SYNTHESIS_ROWS, RuleSynthesisError,
    SynthesisRow, synthesize_exact_conjunction_with_work_budget,
};

/// Fail-closed input or synthesis errors for frozen model-mask reconstruction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelMaskSynthesisError {
    /// No model units were supplied.
    EmptyMask,
    /// Predicate rows and the frozen model mask must address the same units.
    RowCountMismatch { predicates: usize, mask: usize },
    /// The bounded exact synthesis kernel rejected or exhausted the request.
    Synthesis(RuleSynthesisError),
}

/// Try to reconstruct one frozen model mask as an exact conjunctive rule.
///
/// Each entry in `predicate_rows` describes one model unit using caller-frozen
/// Boolean predicates. `selected_mask` is the already-selected model mask for the
/// same ordered units. This function converts that immutable model-level object
/// into labelled synthesis rows and delegates to the exact bounded core search.
///
/// `Ok(None)` is an informative negative result: the bounded search completed and
/// no conjunction within `max_literals` exactly represents the frozen mask.
/// Resource exhaustion remains an error and must not be interpreted as absence of
/// an exact rule.
///
/// # Errors
///
/// Returns an error for empty or mismatched model-unit inputs, malformed predicate
/// tables, invalid literal budgets, or synthesis work-budget exhaustion.
pub fn synthesize_frozen_model_mask(
    predicate_rows: &[Vec<bool>],
    selected_mask: &[bool],
    max_literals: usize,
    work_budget: usize,
) -> Result<Option<ConjunctiveSparsityRule>, ModelMaskSynthesisError> {
    if selected_mask.is_empty() {
        return Err(ModelMaskSynthesisError::EmptyMask);
    }
    if predicate_rows.len() != selected_mask.len() {
        return Err(ModelMaskSynthesisError::RowCountMismatch {
            predicates: predicate_rows.len(),
            mask: selected_mask.len(),
        });
    }

    if predicate_rows.len() > MAX_SYNTHESIS_ROWS {
        return Err(ModelMaskSynthesisError::Synthesis(
            RuleSynthesisError::TooManyRows {
                actual: predicate_rows.len(),
                maximum: MAX_SYNTHESIS_ROWS,
            },
        ));
    }

    let predicate_count = predicate_rows[0].len();
    if predicate_count > MAX_SYNTHESIS_PREDICATES {
        return Err(ModelMaskSynthesisError::Synthesis(
            RuleSynthesisError::TooManyPredicates {
                actual: predicate_count,
                maximum: MAX_SYNTHESIS_PREDICATES,
            },
        ));
    }
    for (row, predicates) in predicate_rows.iter().enumerate().skip(1) {
        if predicates.len() != predicate_count {
            return Err(ModelMaskSynthesisError::Synthesis(
                RuleSynthesisError::RowWidthMismatch {
                    row,
                    expected: predicate_count,
                    actual: predicates.len(),
                },
            ));
        }
    }

    let rows: Vec<_> = predicate_rows
        .iter()
        .zip(selected_mask.iter().copied())
        .map(|(predicates, active)| SynthesisRow::new(predicates.clone(), active))
        .collect();

    synthesize_exact_conjunction_with_work_budget(&rows, max_literals, work_budget)
        .map_err(ModelMaskSynthesisError::Synthesis)
}

impl fmt::Display for ModelMaskSynthesisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ModelMaskSynthesisError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn two_predicate_rows() -> Vec<Vec<bool>> {
        vec![
            vec![false, false],
            vec![false, true],
            vec![true, false],
            vec![true, true],
        ]
    }

    #[test]
    fn reconstructs_frozen_conjunctive_model_mask_exactly() {
        let predicates = two_predicate_rows();
        let mask = [false, false, false, true];
        let rule = synthesize_frozen_model_mask(&predicates, &mask, 2, 128)
            .unwrap()
            .unwrap();

        for (row, expected) in predicates.iter().zip(mask) {
            assert_eq!(rule.evaluate(row).unwrap(), expected);
        }
    }

    #[test]
    fn preserves_nonrepresentable_mask_as_negative_result() {
        let predicates = two_predicate_rows();
        let xor_mask = [false, true, true, false];

        assert_eq!(
            synthesize_frozen_model_mask(&predicates, &xor_mask, 2, 128).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_empty_and_misaligned_model_units() {
        assert_eq!(
            synthesize_frozen_model_mask(&[], &[], 0, 1),
            Err(ModelMaskSynthesisError::EmptyMask)
        );
        assert_eq!(
            synthesize_frozen_model_mask(&[vec![true]], &[true, false], 1, 8),
            Err(ModelMaskSynthesisError::RowCountMismatch {
                predicates: 1,
                mask: 2,
            })
        );
    }

    #[test]
    fn rejects_core_bounds_before_cloning_model_rows() {
        let too_many_rows = vec![vec![true]; MAX_SYNTHESIS_ROWS + 1];
        let too_many_mask = vec![true; MAX_SYNTHESIS_ROWS + 1];
        assert_eq!(
            synthesize_frozen_model_mask(&too_many_rows, &too_many_mask, 1, 8),
            Err(ModelMaskSynthesisError::Synthesis(
                RuleSynthesisError::TooManyRows {
                    actual: MAX_SYNTHESIS_ROWS + 1,
                    maximum: MAX_SYNTHESIS_ROWS,
                }
            ))
        );

        let too_wide = vec![vec![false; MAX_SYNTHESIS_PREDICATES + 1]];
        assert_eq!(
            synthesize_frozen_model_mask(&too_wide, &[true], 1, 8),
            Err(ModelMaskSynthesisError::Synthesis(
                RuleSynthesisError::TooManyPredicates {
                    actual: MAX_SYNTHESIS_PREDICATES + 1,
                    maximum: MAX_SYNTHESIS_PREDICATES,
                }
            ))
        );
    }

    #[test]
    fn propagates_inconsistent_predicate_width_and_work_exhaustion() {
        let malformed = vec![vec![false, false], vec![true]];
        assert!(matches!(
            synthesize_frozen_model_mask(&malformed, &[false, true], 1, 16),
            Err(ModelMaskSynthesisError::Synthesis(
                RuleSynthesisError::RowWidthMismatch { .. }
            ))
        ));

        let predicates = two_predicate_rows();
        let mask = [false, false, false, true];
        assert_eq!(
            synthesize_frozen_model_mask(&predicates, &mask, 2, 0),
            Err(ModelMaskSynthesisError::Synthesis(
                RuleSynthesisError::WorkBudgetExceeded { budget: 0 }
            ))
        );
    }
}
