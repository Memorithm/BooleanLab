//! Bounded exhaustive analysis for Strong-Kleene expression programs.
//!
//! The helpers in this module are deliberately small-domain and exact. They
//! provide deterministic differential evidence for BL-BE1 without turning
//! BooleanLab into a production runtime or making performance claims.

use crate::kleene::{
    KLEENE_VALUES, KleeneEvalError, KleeneInstruction, KleeneValue, evaluate_kleene_program,
};

/// Failure while exhaustively analyzing a Strong-Kleene program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneAnalysisError {
    /// `3^input_arity` cannot be represented as `usize`.
    EnumerationOverflow { input_arity: usize },
    /// The requested exhaustive domain exceeds the caller-declared row budget.
    EnumerationLimitExceeded { required_rows: usize, max_rows: usize },
    /// The underlying postfix program is structurally invalid.
    Evaluation(KleeneEvalError),
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

/// Exhaustively characterize a Strong-Kleene postfix program under an explicit
/// row budget.
///
/// The caller supplies `max_rows`; the function never silently expands beyond
/// that bound. This keeps BL-BE1 enumeration explicit and prevents an
/// accidental `3^n` explosion from being mistaken for completed evidence.
///
/// # Errors
///
/// Returns [`KleeneAnalysisError::EnumerationOverflow`] when `3^input_arity`
/// overflows `usize`, [`KleeneAnalysisError::EnumerationLimitExceeded`] when
/// the exact domain is larger than `max_rows`, or
/// [`KleeneAnalysisError::Evaluation`] when the postfix program is malformed.
pub fn analyze_kleene_program(
    program: &[KleeneInstruction],
    input_arity: usize,
    max_rows: usize,
) -> Result<KleeneProgramAnalysis, KleeneAnalysisError> {
    let rows = checked_pow3(input_arity)
        .ok_or(KleeneAnalysisError::EnumerationOverflow { input_arity })?;
    if rows > max_rows {
        return Err(KleeneAnalysisError::EnumerationLimitExceeded {
            required_rows: rows,
            max_rows,
        });
    }

    let mut outputs = Vec::with_capacity(rows);
    let mut inputs = vec![KleeneValue::False; input_arity];

    for assignment in 0..rows {
        decode_assignment(assignment, &mut inputs);
        let output = evaluate_kleene_program(program, &inputs)
            .map_err(KleeneAnalysisError::Evaluation)?;
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
        let tautology = analyze_kleene_program(
            &[KleeneInstruction::Constant(KleeneValue::True)],
            2,
            9,
        )
        .expect("constant true program is valid");
        assert!(tautology.always_true);
        assert!(!tautology.always_false);
        assert_eq!(tautology.redundant_inputs, vec![0, 1]);

        let contradiction = analyze_kleene_program(
            &[KleeneInstruction::Constant(KleeneValue::False)],
            2,
            9,
        )
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
