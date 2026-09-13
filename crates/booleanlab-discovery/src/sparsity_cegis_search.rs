//! BL-14.5 bounded counterexample-guided Boolean rule search.
//!
//! This is a deterministic finite-domain CEGIS baseline for SEARCH only. It
//! operates on an explicit candidate population and an explicit labelled
//! predicate table. HOLDOUT evidence, task quality, hardware timing and model
//! state are outside this module.

use core::fmt;

use crate::BooleanFunction;
use crate::sparsity_exhaustive_search::ExhaustiveRuleProposal;

/// Exact accounting for one deterministic finite-domain CEGIS run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CegisSearchResult {
    pub candidate_index: usize,
    pub candidate_id: String,
    pub counterexample_rows: Vec<usize>,
    pub synthesis_candidate_checks: u64,
    pub synthesis_row_checks: u64,
    pub verification_row_checks: u64,
}

/// Fail-closed errors for bounded CEGIS search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CegisSearchError {
    EmptyCandidateSet,
    EmptyExamples,
    ExampleLengthMismatch { predicate_rows: usize, target: usize },
    PredicateArityMismatch {
        row: usize,
        expected: usize,
        actual: usize,
    },
    MixedCandidateArity {
        candidate_index: usize,
        expected: u32,
        actual: u32,
    },
    NoConsistentCandidate,
    CounterOverflow,
}

/// Run deterministic counterexample-guided synthesis over a finite population.
///
/// The synthesizer always picks the first candidate consistent with every
/// accumulated counterexample. The verifier then scans the complete SEARCH
/// table in row order and adds the first mismatch. Search terminates when the
/// selected candidate matches every SEARCH row exactly, or when no candidate is
/// consistent with the accumulated constraints.
///
/// This method intentionally preserves the distinction between synthesis work
/// and full-table verification work so BL-14.5 can compare search methods under
/// explicit candidate-evaluation budgets.
///
/// # Errors
///
/// Rejects empty candidate/example sets, target length drift, candidate or row
/// arity drift, exhausted candidate populations, and exact accounting overflow.
pub fn search_cegis(
    candidates: &[ExhaustiveRuleProposal],
    predicate_rows: &[&[bool]],
    target_keep: &[bool],
) -> Result<CegisSearchResult, CegisSearchError> {
    if candidates.is_empty() {
        return Err(CegisSearchError::EmptyCandidateSet);
    }
    if predicate_rows.is_empty() || target_keep.is_empty() {
        return Err(CegisSearchError::EmptyExamples);
    }
    if predicate_rows.len() != target_keep.len() {
        return Err(CegisSearchError::ExampleLengthMismatch {
            predicate_rows: predicate_rows.len(),
            target: target_keep.len(),
        });
    }

    let input_bits = candidates[0].function.input_bits();
    let expected_arity = usize::try_from(input_bits).map_err(|_| CegisSearchError::CounterOverflow)?;
    for (candidate_index, candidate) in candidates.iter().enumerate() {
        if candidate.function.input_bits() != input_bits {
            return Err(CegisSearchError::MixedCandidateArity {
                candidate_index,
                expected: input_bits,
                actual: candidate.function.input_bits(),
            });
        }
    }
    for (row, predicates) in predicate_rows.iter().enumerate() {
        if predicates.len() != expected_arity {
            return Err(CegisSearchError::PredicateArityMismatch {
                row,
                expected: expected_arity,
                actual: predicates.len(),
            });
        }
    }

    let mut counterexamples = Vec::new();
    let mut synthesis_candidate_checks = 0u64;
    let mut synthesis_row_checks = 0u64;
    let mut verification_row_checks = 0u64;

    loop {
        let mut selected = None;
        'candidate: for (candidate_index, candidate) in candidates.iter().enumerate() {
            synthesis_candidate_checks = synthesis_candidate_checks
                .checked_add(1)
                .ok_or(CegisSearchError::CounterOverflow)?;
            for &row in &counterexamples {
                synthesis_row_checks = synthesis_row_checks
                    .checked_add(1)
                    .ok_or(CegisSearchError::CounterOverflow)?;
                if evaluate(&candidate.function, predicate_rows[row]) != target_keep[row] {
                    continue 'candidate;
                }
            }
            selected = Some(candidate_index);
            break;
        }

        let Some(candidate_index) = selected else {
            return Err(CegisSearchError::NoConsistentCandidate);
        };
        let candidate = &candidates[candidate_index];
        let mut mismatch = None;
        for (row, predicates) in predicate_rows.iter().enumerate() {
            verification_row_checks = verification_row_checks
                .checked_add(1)
                .ok_or(CegisSearchError::CounterOverflow)?;
            if evaluate(&candidate.function, predicates) != target_keep[row] {
                mismatch = Some(row);
                break;
            }
        }

        if let Some(row) = mismatch {
            if !counterexamples.contains(&row) {
                counterexamples.push(row);
            }
        } else {
            return Ok(CegisSearchResult {
                candidate_index,
                candidate_id: candidate.proposal_id.clone(),
                counterexample_rows: counterexamples,
                synthesis_candidate_checks,
                synthesis_row_checks,
                verification_row_checks,
            });
        }
    }
}

fn evaluate(function: &BooleanFunction, predicates: &[bool]) -> bool {
    let row = predicates
        .iter()
        .enumerate()
        .fold(0usize, |row, (bit, predicate)| {
            row | (usize::from(*predicate) << bit)
        });
    function.truth_table()[row] == 1
}

impl fmt::Display for CegisSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CegisSearchError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sparsity_exhaustive_search::propose_exhaustive_rules;

    #[test]
    fn cegis_recovers_xor_from_complete_two_bit_domain() {
        let candidates = propose_exhaustive_rules(2).unwrap();
        let rows: [&[bool]; 4] = [
            &[false, false],
            &[true, false],
            &[false, true],
            &[true, true],
        ];
        let target = [false, true, true, false];

        let result = search_cegis(&candidates, &rows, &target).unwrap();
        assert_eq!(candidates[result.candidate_index].truth_table_code, 0b0110);
        assert_eq!(result.candidate_id, "bl14-exhaustive-n2-0000000000000006");
        assert!(!result.counterexample_rows.is_empty());
        assert!(result.synthesis_candidate_checks > 0);
        assert!(result.verification_row_checks > 0);
    }

    #[test]
    fn cegis_preserves_first_consistent_solution_under_partial_coverage() {
        let candidates = propose_exhaustive_rules(2).unwrap();
        let rows: [&[bool]; 3] = [&[false, false], &[true, false], &[false, true]];
        let target = [false, true, true];

        let result = search_cegis(&candidates, &rows, &target).unwrap();
        assert_eq!(candidates[result.candidate_index].truth_table_code, 0b0110);
    }

    #[test]
    fn rejects_predicate_arity_drift() {
        let candidates = propose_exhaustive_rules(2).unwrap();
        let rows: [&[bool]; 1] = [&[true]];
        assert_eq!(
            search_cegis(&candidates, &rows, &[true]),
            Err(CegisSearchError::PredicateArityMismatch {
                row: 0,
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn reports_no_consistent_candidate_when_population_cannot_fit_target() {
        let mut candidates = propose_exhaustive_rules(1).unwrap();
        candidates.truncate(1);
        let rows: [&[bool]; 2] = [&[false], &[true]];
        assert_eq!(
            search_cegis(&candidates, &rows, &[false, true]),
            Err(CegisSearchError::NoConsistentCandidate)
        );
    }
}
