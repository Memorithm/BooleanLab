//! Exact BL-14.5 calibration of exhaustive Boolean rules against a declared mask.
//!
//! This is a SEARCH-phase calibration primitive. It measures only exact Boolean
//! decision disagreement on explicit predicate rows. It does not substitute for
//! task quality, hardware timing, memory traffic, or HOLDOUT evaluation.

use std::fmt;

use booleanlab_core::DynamicMaskError;

use crate::sparsity_exhaustive_search::{
    ExhaustiveRuleSearchError, propose_exhaustive_rules,
};
use crate::sparsity_function_mask::materialize_boolean_function_mask;

/// Exact fit evidence for one exhaustively enumerated Boolean sparsity rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExhaustiveMaskFit {
    pub proposal_id: String,
    pub truth_table_code: u64,
    pub mismatches: u64,
    pub retained_units: u64,
}

/// Fail-closed errors for exhaustive mask-fit calibration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExhaustiveMaskFitError {
    EmptyTarget,
    TargetLengthMismatch { predicate_rows: usize, target: usize },
    Search(ExhaustiveRuleSearchError),
    Mask(DynamicMaskError),
    CounterOverflow,
}

/// Evaluate every bounded exhaustive rule against one explicit target mask.
///
/// Candidate order is the exact integer truth-table order supplied by
/// `propose_exhaustive_rules`. The function reports decision mismatches and
/// retained cardinality separately. No scalarized task-quality objective is
/// introduced.
///
/// # Errors
///
/// Rejects an empty target, target/predicate-row length drift, bounded search
/// failures, dynamic-mask validation failures, or exact counter overflow.
pub fn evaluate_exhaustive_mask_fit(
    input_bits: u32,
    predicate_rows: &[&[bool]],
    target_keep: &[bool],
) -> Result<Vec<ExhaustiveMaskFit>, ExhaustiveMaskFitError> {
    if target_keep.is_empty() {
        return Err(ExhaustiveMaskFitError::EmptyTarget);
    }
    if predicate_rows.len() != target_keep.len() {
        return Err(ExhaustiveMaskFitError::TargetLengthMismatch {
            predicate_rows: predicate_rows.len(),
            target: target_keep.len(),
        });
    }

    let proposals = propose_exhaustive_rules(input_bits).map_err(ExhaustiveMaskFitError::Search)?;
    let mut evidence = Vec::with_capacity(proposals.len());

    for proposal in proposals {
        let mask = materialize_boolean_function_mask(&proposal.function, predicate_rows)
            .map_err(ExhaustiveMaskFitError::Mask)?;
        let mut mismatches = 0u64;
        for (&actual, &expected) in mask.as_slice().iter().zip(target_keep.iter()) {
            if actual != expected {
                mismatches = mismatches
                    .checked_add(1)
                    .ok_or(ExhaustiveMaskFitError::CounterOverflow)?;
            }
        }
        let retained_units = u64::try_from(mask.cardinality().retained())
            .map_err(|_| ExhaustiveMaskFitError::CounterOverflow)?;
        evidence.push(ExhaustiveMaskFit {
            proposal_id: proposal.proposal_id,
            truth_table_code: proposal.truth_table_code,
            mismatches,
            retained_units,
        });
    }

    Ok(evidence)
}

/// Return every candidate index attaining the exact minimum mismatch count.
///
/// Ties are preserved deliberately. Incomplete predicate coverage can make
/// several distinct Boolean functions observationally indistinguishable.
///
/// # Errors
///
/// Returns [`ExhaustiveMaskFitError::EmptyTarget`] when no evidence is supplied.
pub fn best_mismatch_indices(
    evidence: &[ExhaustiveMaskFit],
) -> Result<Vec<usize>, ExhaustiveMaskFitError> {
    let Some(best) = evidence.iter().map(|row| row.mismatches).min() else {
        return Err(ExhaustiveMaskFitError::EmptyTarget);
    };
    Ok(evidence
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row.mismatches == best).then_some(index))
        .collect())
}

impl fmt::Display for ExhaustiveMaskFitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ExhaustiveMaskFitError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive_fit_recovers_unique_xor_on_complete_two_bit_domain() {
        let rows: [&[bool]; 4] = [
            &[false, false],
            &[true, false],
            &[false, true],
            &[true, true],
        ];
        let target = [false, true, true, false];

        let evidence = evaluate_exhaustive_mask_fit(2, &rows, &target).unwrap();
        let best = best_mismatch_indices(&evidence).unwrap();
        assert_eq!(best.len(), 1);
        let winner = &evidence[best[0]];
        assert_eq!(winner.truth_table_code, 0b0110);
        assert_eq!(winner.mismatches, 0);
        assert_eq!(winner.retained_units, 2);
    }

    #[test]
    fn incomplete_predicate_coverage_preserves_semantically_distinct_ties() {
        let rows: [&[bool]; 3] = [&[false, false], &[true, false], &[false, true]];
        let target = [false, true, true];

        let evidence = evaluate_exhaustive_mask_fit(2, &rows, &target).unwrap();
        let best = best_mismatch_indices(&evidence).unwrap();
        let codes = best
            .iter()
            .map(|&index| evidence[index].truth_table_code)
            .collect::<Vec<_>>();
        assert_eq!(codes, vec![0b0110, 0b1110]);
        assert!(best.iter().all(|&index| evidence[index].mismatches == 0));
    }

    #[test]
    fn rejects_target_shape_drift_before_search() {
        let rows: [&[bool]; 2] = [&[false, false], &[true, false]];
        assert_eq!(
            evaluate_exhaustive_mask_fit(2, &rows, &[false]),
            Err(ExhaustiveMaskFitError::TargetLengthMismatch {
                predicate_rows: 2,
                target: 1,
            })
        );
    }
}
