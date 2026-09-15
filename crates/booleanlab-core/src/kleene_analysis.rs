//! Bounded exhaustive analysis for Strong-Kleene expression programs.
//!
//! The helpers in this module are deliberately small-domain and exact. They
//! provide deterministic differential evidence for BL-BE1 without turning
//! `BooleanLab` into a production runtime or making performance claims.

use crate::kleene::{
    KLEENE_VALUES, KleeneEvalError, KleeneInstruction, KleeneValue, evaluate_kleene_program,
};

/// Default worst-case instruction-evaluation budget used by
/// [`analyze_kleene_program`].
///
/// Callers that need a different explicit bound should use
/// [`analyze_kleene_program_with_work_budget`].
pub const DEFAULT_KLEENE_ANALYSIS_MAX_INSTRUCTION_EVALUATIONS: usize = 1_000_000;

/// Failure while exhaustively analyzing a Strong-Kleene program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneAnalysisError {
    /// `3^input_arity` cannot be represented as `usize`.
    EnumerationOverflow { input_arity: usize },
    /// The requested exhaustive domain exceeds the caller-declared row budget.
    EnumerationLimitExceeded {
        required_rows: usize,
        max_rows: usize,
    },
    /// The exact instruction-evaluation work estimate cannot be represented.
    WorkEstimateOverflow { rows: usize, instructions: usize },
    /// The requested exact analysis exceeds the declared work budget.
    ///
    /// This is an explicit non-result: it does not establish tautology,
    /// contradiction, redundancy, or any other semantic property.
    WorkLimitExceeded {
        required_instruction_evaluations: usize,
        max_instruction_evaluations: usize,
    },
    /// The underlying postfix program is structurally invalid.
    Evaluation(KleeneEvalError),
}

/// Failure while differentially comparing two Strong-Kleene programs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneComparisonError {
    /// `3^input_arity` cannot be represented as `usize`.
    EnumerationOverflow { input_arity: usize },
    /// The requested exhaustive domain exceeds the caller-declared row budget.
    EnumerationLimitExceeded {
        required_rows: usize,
        max_rows: usize,
    },
    /// The exact instruction-evaluation work estimate cannot be represented.
    WorkEstimateOverflow {
        rows: usize,
        left_instructions: usize,
        right_instructions: usize,
    },
    /// The requested exact comparison exceeds the caller-declared work budget.
    ///
    /// This is an explicit non-result: it does not establish equivalence or a
    /// semantic mismatch.
    WorkLimitExceeded {
        required_instruction_evaluations: usize,
        max_instruction_evaluations: usize,
    },
    /// The left postfix program is structurally invalid.
    LeftEvaluation(KleeneEvalError),
    /// The right postfix program is structurally invalid.
    RightEvaluation(KleeneEvalError),
}

/// First exact counterexample found while comparing two programs.
///
/// The assignment uses canonical input order and preserves `Unknown`; it is a
/// reproducible semantic witness, not a sampled or heuristic discrepancy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KleeneProgramMismatch {
    /// Canonical base-3 assignment index, with input 0 as the least significant trit.
    pub assignment_index: usize,
    /// Exact input values for the mismatching assignment.
    pub inputs: Vec<KleeneValue>,
    /// Output produced by the left program.
    pub left: KleeneValue,
    /// Output produced by the right program.
    pub right: KleeneValue,
}

/// Result of an exact bounded differential comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KleeneProgramComparison {
    /// Both programs produced exactly the same Strong-Kleene value on every row.
    Equivalent { rows: usize },
    /// The programs differ; `witness` is the first mismatch in canonical row order.
    Different {
        rows: usize,
        witness: KleeneProgramMismatch,
    },
}

/// Exact small-domain characterization of one Strong-Kleene program.
///
/// `outputs` is ordered by base-3 assignment index. Input 0 is the least
/// significant trit and each trit uses the canonical order in
/// [`KLEENE_VALUES`]: `False`, `Unknown`, `True`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KleeneProgramAnalysis {
    /// Number of declared input variables.
    pub input_arity: usize,
    /// Number of exhaustively evaluated rows (`3^input_arity`).
    pub rows: usize,
    /// Canonical output vector for the exhaustive domain.
    pub outputs: Vec<KleeneValue>,
    /// Whether every exhaustive row evaluates to `True`.
    pub always_true: bool,
    /// Whether every exhaustive row evaluates to `False`.
    pub always_false: bool,
    /// Input indices whose value never changes the output while all other
    /// inputs are held fixed over the exhaustive domain.
    pub redundant_inputs: Vec<usize>,
}

/// Exhaustively characterize a Strong-Kleene postfix program under explicit
/// row and default work budgets.
///
/// The caller supplies `max_rows`; the function never silently expands beyond
/// that bound. It also caps worst-case postfix instruction visits at
/// [`DEFAULT_KLEENE_ANALYSIS_MAX_INSTRUCTION_EVALUATIONS`]. Call
/// [`analyze_kleene_program_with_work_budget`] when a different explicit work
/// budget is required.
///
/// # Errors
///
/// Returns [`KleeneAnalysisError::EnumerationOverflow`] when `3^input_arity`
/// overflows `usize`, [`KleeneAnalysisError::EnumerationLimitExceeded`] when
/// the exact domain is larger than `max_rows`, a work-budget error when the
/// exact worst-case instruction count cannot be represented or exceeds the
/// default limit, or [`KleeneAnalysisError::Evaluation`] when the postfix
/// program is malformed.
pub fn analyze_kleene_program(
    program: &[KleeneInstruction],
    input_arity: usize,
    max_rows: usize,
) -> Result<KleeneProgramAnalysis, KleeneAnalysisError> {
    analyze_kleene_program_with_work_budget(
        program,
        input_arity,
        max_rows,
        DEFAULT_KLEENE_ANALYSIS_MAX_INSTRUCTION_EVALUATIONS,
    )
}

/// Exhaustively characterize a Strong-Kleene postfix program under explicit
/// row and instruction-evaluation budgets.
///
/// The exact worst-case work estimate is `3^input_arity * program.len()` and is
/// checked before allocating the output or input vectors and before evaluating
/// the program. An empty program is rejected as structurally invalid before
/// allocation. Exhausting the work budget is an explicit non-result, never
/// evidence for tautology, contradiction, redundancy, or any other property.
///
/// # Errors
///
/// Returns [`KleeneAnalysisError::EnumerationOverflow`] when `3^input_arity`
/// overflows `usize`, [`KleeneAnalysisError::EnumerationLimitExceeded`] when
/// the exact domain is larger than `max_rows`,
/// [`KleeneAnalysisError::WorkEstimateOverflow`] when the exact work estimate
/// cannot be represented, [`KleeneAnalysisError::WorkLimitExceeded`] when the
/// declared work budget is insufficient, or [`KleeneAnalysisError::Evaluation`]
/// when the postfix program is malformed.
pub fn analyze_kleene_program_with_work_budget(
    program: &[KleeneInstruction],
    input_arity: usize,
    max_rows: usize,
    max_instruction_evaluations: usize,
) -> Result<KleeneProgramAnalysis, KleeneAnalysisError> {
    let rows = checked_pow3(input_arity)
        .ok_or(KleeneAnalysisError::EnumerationOverflow { input_arity })?;
    if rows > max_rows {
        return Err(KleeneAnalysisError::EnumerationLimitExceeded {
            required_rows: rows,
            max_rows,
        });
    }
    if program.is_empty() {
        return Err(KleeneAnalysisError::Evaluation(
            KleeneEvalError::InvalidFinalStackDepth { depth: 0 },
        ));
    }

    let required_instruction_evaluations =
        rows.checked_mul(program.len())
            .ok_or(KleeneAnalysisError::WorkEstimateOverflow {
                rows,
                instructions: program.len(),
            })?;
    if required_instruction_evaluations > max_instruction_evaluations {
        return Err(KleeneAnalysisError::WorkLimitExceeded {
            required_instruction_evaluations,
            max_instruction_evaluations,
        });
    }

    let mut outputs = Vec::with_capacity(rows);
    let mut inputs = vec![KleeneValue::False; input_arity];

    for assignment in 0..rows {
        decode_assignment(assignment, &mut inputs);
        let output =
            evaluate_kleene_program(program, &inputs).map_err(KleeneAnalysisError::Evaluation)?;
        outputs.push(output);
    }

    let always_true = outputs.iter().all(|value| *value == KleeneValue::True);
    let always_false = outputs.iter().all(|value| *value == KleeneValue::False);
    let redundant_inputs = redundant_inputs(&outputs, input_arity);

    Ok(KleeneProgramAnalysis {
        input_arity,
        rows,
        outputs,
        always_true,
        always_false,
        redundant_inputs,
    })
}

/// Compare two Strong-Kleene postfix programs exactly over a bounded domain.
///
/// Rows are evaluated in the same canonical base-3 order used by
/// [`analyze_kleene_program`]. The comparison stops at the first mismatch and
/// returns the complete input assignment as a deterministic counterexample.
/// Equality therefore means exact equality over the entire declared exhaustive
/// domain, not equality on a sample. `Unknown` is compared as its own value and
/// is never collapsed into `False`.
///
/// `max_instruction_evaluations` bounds the exact worst-case number of postfix
/// instruction visits as `rows * (left.len() + right.len())`. Exhausting that
/// budget is reported before allocating the assignment vector or evaluating a
/// program and is a non-result, never evidence of equivalence or difference.
///
/// # Errors
///
/// Returns [`KleeneComparisonError::EnumerationOverflow`] when `3^input_arity`
/// overflows `usize`, [`KleeneComparisonError::EnumerationLimitExceeded`] when
/// the exact domain is larger than `max_rows`,
/// [`KleeneComparisonError::WorkEstimateOverflow`] when the exact work estimate
/// cannot be represented, [`KleeneComparisonError::WorkLimitExceeded`] when the
/// declared work budget is insufficient, or a side-specific evaluation error
/// when either postfix program is malformed.
pub fn compare_kleene_programs(
    left: &[KleeneInstruction],
    right: &[KleeneInstruction],
    input_arity: usize,
    max_rows: usize,
    max_instruction_evaluations: usize,
) -> Result<KleeneProgramComparison, KleeneComparisonError> {
    let rows = checked_pow3(input_arity)
        .ok_or(KleeneComparisonError::EnumerationOverflow { input_arity })?;
    if rows > max_rows {
        return Err(KleeneComparisonError::EnumerationLimitExceeded {
            required_rows: rows,
            max_rows,
        });
    }

    let instruction_count =
        left.len()
            .checked_add(right.len())
            .ok_or(KleeneComparisonError::WorkEstimateOverflow {
                rows,
                left_instructions: left.len(),
                right_instructions: right.len(),
            })?;
    let required_instruction_evaluations =
        rows.checked_mul(instruction_count)
            .ok_or(KleeneComparisonError::WorkEstimateOverflow {
                rows,
                left_instructions: left.len(),
                right_instructions: right.len(),
            })?;
    if required_instruction_evaluations > max_instruction_evaluations {
        return Err(KleeneComparisonError::WorkLimitExceeded {
            required_instruction_evaluations,
            max_instruction_evaluations,
        });
    }

    let mut inputs = vec![KleeneValue::False; input_arity];
    for assignment in 0..rows {
        decode_assignment(assignment, &mut inputs);
        let left_output = evaluate_kleene_program(left, &inputs)
            .map_err(KleeneComparisonError::LeftEvaluation)?;
        let right_output = evaluate_kleene_program(right, &inputs)
            .map_err(KleeneComparisonError::RightEvaluation)?;

        if left_output != right_output {
            return Ok(KleeneProgramComparison::Different {
                rows,
                witness: KleeneProgramMismatch {
                    assignment_index: assignment,
                    inputs: inputs.clone(),
                    left: left_output,
                    right: right_output,
                },
            });
        }
    }

    Ok(KleeneProgramComparison::Equivalent { rows })
}

fn checked_pow3(exponent: usize) -> Option<usize> {
    let mut value = 1usize;
    for _ in 0..exponent {
        value = value.checked_mul(3)?;
    }
    Some(value)
}

fn decode_assignment(mut assignment: usize, inputs: &mut [KleeneValue]) {
    for input in inputs {
        *input = KLEENE_VALUES[assignment % 3];
        assignment /= 3;
    }
}

fn redundant_inputs(outputs: &[KleeneValue], input_arity: usize) -> Vec<usize> {
    let mut redundant = Vec::new();
    let mut stride = 1usize;

    for input in 0..input_arity {
        let group_width = stride * 3;
        let mut is_redundant = true;

        for group_start in (0..outputs.len()).step_by(group_width) {
            for offset in 0..stride {
                let false_value = outputs[group_start + offset];
                let unknown_value = outputs[group_start + stride + offset];
                let true_value = outputs[group_start + (2 * stride) + offset];
                if false_value != unknown_value || false_value != true_value {
                    is_redundant = false;
                    break;
                }
            }
            if !is_redundant {
                break;
            }
        }

        if is_redundant {
            redundant.push(input);
        }
        stride *= 3;
    }

    redundant
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_outputs_follow_base3_assignment_order() {
        let analysis = analyze_kleene_program(&[KleeneInstruction::Input(0)], 1, 3)
            .expect("one-input exhaustive domain must fit the declared budget");

        assert_eq!(analysis.rows, 3);
        assert_eq!(analysis.outputs, KLEENE_VALUES);
        assert!(!analysis.always_true);
        assert!(!analysis.always_false);
        assert!(analysis.redundant_inputs.is_empty());
    }

    #[test]
    fn detects_three_valued_tautology_and_contradiction() {
        let tautology =
            analyze_kleene_program(&[KleeneInstruction::Constant(KleeneValue::True)], 2, 9)
                .expect("constant true program is valid");
        assert!(tautology.always_true);
        assert!(!tautology.always_false);
        assert_eq!(tautology.redundant_inputs, vec![0, 1]);

        let contradiction =
            analyze_kleene_program(&[KleeneInstruction::Constant(KleeneValue::False)], 2, 9)
                .expect("constant false program is valid");
        assert!(!contradiction.always_true);
        assert!(contradiction.always_false);
        assert_eq!(contradiction.redundant_inputs, vec![0, 1]);
    }

    #[test]
    fn detects_redundant_input_without_collapsing_unknown() {
        // x0 OR (x1 AND False) == x0 in Strong-Kleene logic.
        let program = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(1),
            KleeneInstruction::Constant(KleeneValue::False),
            KleeneInstruction::And,
            KleeneInstruction::Or,
        ];
        let analysis = analyze_kleene_program(&program, 2, 9)
            .expect("two-input exhaustive domain must fit the declared budget");

        assert_eq!(analysis.redundant_inputs, vec![1]);
        assert_eq!(analysis.outputs[1], KleeneValue::Unknown);
    }

    #[test]
    fn semantically_equivalent_programs_have_identical_canonical_outputs() {
        let direct = analyze_kleene_program(&[KleeneInstruction::Input(0)], 2, 9)
            .expect("direct program is valid");
        let with_redundancy = analyze_kleene_program(
            &[
                KleeneInstruction::Input(0),
                KleeneInstruction::Input(1),
                KleeneInstruction::Constant(KleeneValue::False),
                KleeneInstruction::And,
                KleeneInstruction::Or,
            ],
            2,
            9,
        )
        .expect("redundant program is valid");

        assert_eq!(direct.outputs, with_redundancy.outputs);
    }

    #[test]
    fn analysis_work_budget_is_an_explicit_non_result() {
        let program = [KleeneInstruction::Input(0), KleeneInstruction::Not];
        assert_eq!(
            analyze_kleene_program_with_work_budget(&program, 1, 3, 5),
            Err(KleeneAnalysisError::WorkLimitExceeded {
                required_instruction_evaluations: 6,
                max_instruction_evaluations: 5,
            })
        );
    }

    #[test]
    fn analysis_work_budget_is_checked_before_malformed_program_evaluation() {
        assert_eq!(
            analyze_kleene_program_with_work_budget(&[KleeneInstruction::And], 1, 3, 2),
            Err(KleeneAnalysisError::WorkLimitExceeded {
                required_instruction_evaluations: 3,
                max_instruction_evaluations: 2,
            })
        );
    }

    #[test]
    fn empty_program_is_rejected_before_output_allocation() {
        assert_eq!(
            analyze_kleene_program_with_work_budget(&[], 40, usize::MAX, usize::MAX),
            Err(KleeneAnalysisError::Evaluation(
                KleeneEvalError::InvalidFinalStackDepth { depth: 0 }
            ))
        );
    }

    #[test]
    fn differential_comparison_proves_exact_small_domain_equivalence() {
        let direct = [KleeneInstruction::Input(0)];
        let redundant = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(1),
            KleeneInstruction::Constant(KleeneValue::False),
            KleeneInstruction::And,
            KleeneInstruction::Or,
        ];

        assert_eq!(
            compare_kleene_programs(&direct, &redundant, 2, 9, 54),
            Ok(KleeneProgramComparison::Equivalent { rows: 9 })
        );
    }

    #[test]
    fn differential_comparison_returns_first_unknown_sensitive_witness() {
        let left = [KleeneInstruction::Input(0)];
        let right = [KleeneInstruction::Input(1)];

        assert_eq!(
            compare_kleene_programs(&left, &right, 2, 9, 18),
            Ok(KleeneProgramComparison::Different {
                rows: 9,
                witness: KleeneProgramMismatch {
                    assignment_index: 1,
                    inputs: vec![KleeneValue::Unknown, KleeneValue::False],
                    left: KleeneValue::Unknown,
                    right: KleeneValue::False,
                },
            })
        );
    }

    #[test]
    fn differential_comparison_budget_fails_closed_before_evaluation() {
        assert_eq!(
            compare_kleene_programs(
                &[KleeneInstruction::And],
                &[KleeneInstruction::Input(0)],
                4,
                80,
                usize::MAX,
            ),
            Err(KleeneComparisonError::EnumerationLimitExceeded {
                required_rows: 81,
                max_rows: 80,
            })
        );
    }

    #[test]
    fn differential_comparison_work_budget_is_an_explicit_non_result() {
        let left = [KleeneInstruction::Input(0), KleeneInstruction::Not];
        let right = [KleeneInstruction::Input(0)];
        assert_eq!(
            compare_kleene_programs(&left, &right, 1, 3, 8),
            Err(KleeneComparisonError::WorkLimitExceeded {
                required_instruction_evaluations: 9,
                max_instruction_evaluations: 8,
            })
        );
    }

    #[test]
    fn differential_comparison_identifies_malformed_side() {
        assert_eq!(
            compare_kleene_programs(
                &[KleeneInstruction::And],
                &[KleeneInstruction::Input(0)],
                1,
                3,
                6,
            ),
            Err(KleeneComparisonError::LeftEvaluation(
                KleeneEvalError::StackUnderflow {
                    instruction: 0,
                    needed: 2,
                    available: 0,
                }
            ))
        );
        assert_eq!(
            compare_kleene_programs(
                &[KleeneInstruction::Input(0)],
                &[KleeneInstruction::And],
                1,
                3,
                6,
            ),
            Err(KleeneComparisonError::RightEvaluation(
                KleeneEvalError::StackUnderflow {
                    instruction: 0,
                    needed: 2,
                    available: 0,
                }
            ))
        );
    }

    #[test]
    fn enumeration_budget_fails_closed() {
        assert_eq!(
            analyze_kleene_program(&[KleeneInstruction::Input(0)], 4, 80),
            Err(KleeneAnalysisError::EnumerationLimitExceeded {
                required_rows: 81,
                max_rows: 80,
            })
        );
    }

    #[test]
    fn malformed_program_error_is_preserved() {
        assert_eq!(
            analyze_kleene_program(&[KleeneInstruction::And], 0, 1),
            Err(KleeneAnalysisError::Evaluation(
                KleeneEvalError::StackUnderflow {
                    instruction: 0,
                    needed: 2,
                    available: 0,
                }
            ))
        );
    }
}
