//! Exact compiled conjunction masks for bounded Strong-Kleene differential tests.
//!
//! This BL-BE1 oracle compiles positive and negative literals into `u64`
//! masks for exhaustive equivalence checks against generic expression
//! evaluation. It makes no production-runtime or performance claim.

use crate::kleene::{
    KLEENE_VALUES, KleeneEvalError, KleeneInstruction, KleeneValue, evaluate_kleene_program,
};

/// Maximum input arity representable by [`CompiledKleeneConjunction`].
pub const MAX_COMPILED_KLEENE_INPUTS: usize = u64::BITS as usize;

/// Failure while compiling or evaluating a bounded conjunction mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneConjunctionError {
    /// The declared input arity exceeds the `u64` representation bound.
    InputArityTooLarge { input_arity: usize, max: usize },
    /// A literal references an input outside the declared arity.
    InputOutOfRange { input: usize, input_arity: usize },
    /// Evaluation input length differs from the compiled arity.
    InputLengthMismatch { expected: usize, actual: usize },
}

/// Failure while exactly comparing a compiled conjunction with a generic program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneConjunctionComparisonError {
    /// `3^input_arity` cannot be represented as `usize`.
    EnumerationOverflow { input_arity: usize },
    /// The exhaustive domain exceeds the caller-declared row budget.
    EnumerationLimitExceeded {
        required_rows: usize,
        max_rows: usize,
    },
    /// The conservative work estimate cannot be represented as `usize`.
    WorkEstimateOverflow {
        rows: usize,
        generic_instructions: usize,
        input_arity: usize,
    },
    /// The requested exact comparison exceeds the caller-declared work budget.
    ///
    /// This is a non-result and must not be interpreted as equivalence or a
    /// semantic mismatch.
    WorkLimitExceeded {
        required_work_units: usize,
        max_work_units: usize,
    },
    /// The generic postfix program is structurally invalid.
    GenericEvaluation(KleeneEvalError),
    /// The compiled conjunction could not be evaluated.
    CompiledEvaluation(KleeneConjunctionError),
}

/// One literal in a conjunction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneLiteral {
    /// Input index addressed by the literal.
    pub input: usize,
    /// `true` requires `True`; `false` requires `False`.
    pub require_true: bool,
}

impl KleeneLiteral {
    /// Construct a positive literal (`x_i`).
    #[must_use]
    pub const fn positive(input: usize) -> Self {
        Self {
            input,
            require_true: true,
        }
    }

    /// Construct a negative literal (`NOT x_i`).
    #[must_use]
    pub const fn negative(input: usize) -> Self {
        Self {
            input,
            require_true: false,
        }
    }
}

/// Compiled conjunction of positive and negative Strong-Kleene literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledKleeneConjunction {
    input_arity: usize,
    require_true_mask: u64,
    require_false_mask: u64,
}

impl CompiledKleeneConjunction {
    /// Compile a literal conjunction into two bounded masks.
    ///
    /// Duplicate literals are idempotent. Opposite literals remain explicit:
    /// under Strong-Kleene semantics `x AND NOT(x)` is `Unknown` when `x` is
    /// `Unknown`, so it must not be collapsed to constant `False`.
    ///
    /// # Errors
    ///
    /// Returns [`KleeneConjunctionError::InputArityTooLarge`] above the `u64`
    /// bound, or [`KleeneConjunctionError::InputOutOfRange`] for an undeclared
    /// input.
    pub fn compile(
        input_arity: usize,
        literals: &[KleeneLiteral],
    ) -> Result<Self, KleeneConjunctionError> {
        if input_arity > MAX_COMPILED_KLEENE_INPUTS {
            return Err(KleeneConjunctionError::InputArityTooLarge {
                input_arity,
                max: MAX_COMPILED_KLEENE_INPUTS,
            });
        }

        let mut require_true_mask = 0_u64;
        let mut require_false_mask = 0_u64;
        for literal in literals {
            if literal.input >= input_arity {
                return Err(KleeneConjunctionError::InputOutOfRange {
                    input: literal.input,
                    input_arity,
                });
            }
            let bit = 1_u64 << literal.input;
            if literal.require_true {
                require_true_mask |= bit;
            } else {
                require_false_mask |= bit;
            }
        }

        Ok(Self {
            input_arity,
            require_true_mask,
            require_false_mask,
        })
    }

    /// Declared input arity.
    #[must_use]
    pub const fn input_arity(self) -> usize {
        self.input_arity
    }

    /// Mask of inputs required to be `True`.
    #[must_use]
    pub const fn require_true_mask(self) -> u64 {
        self.require_true_mask
    }

    /// Mask of inputs required to be `False`.
    #[must_use]
    pub const fn require_false_mask(self) -> u64 {
        self.require_false_mask
    }

    /// Whether at least one input appears with both polarities.
    #[must_use]
    pub const fn has_opposing_literals(self) -> bool {
        (self.require_true_mask & self.require_false_mask) != 0
    }

    /// Evaluate the compiled conjunction under Strong-Kleene semantics.
    ///
    /// Any known violated literal makes the result `False`. Otherwise any
    /// required `Unknown` makes it `Unknown`; only fully satisfied known
    /// literals produce `True`. The empty conjunction is therefore `True`.
    ///
    /// # Errors
    ///
    /// Returns [`KleeneConjunctionError::InputLengthMismatch`] unless the
    /// supplied input length exactly matches the compiled arity.
    pub fn evaluate(self, inputs: &[KleeneValue]) -> Result<KleeneValue, KleeneConjunctionError> {
        if inputs.len() != self.input_arity {
            return Err(KleeneConjunctionError::InputLengthMismatch {
                expected: self.input_arity,
                actual: inputs.len(),
            });
        }

        let mut saw_unknown = false;
        for (input, value) in inputs.iter().copied().enumerate() {
            let bit = 1_u64 << input;
            if self.require_true_mask & bit != 0 {
                match value {
                    KleeneValue::False => return Ok(KleeneValue::False),
                    KleeneValue::Unknown => saw_unknown = true,
                    KleeneValue::True => {}
                }
            }
            if self.require_false_mask & bit != 0 {
                match value {
                    KleeneValue::True => return Ok(KleeneValue::False),
                    KleeneValue::Unknown => saw_unknown = true,
                    KleeneValue::False => {}
                }
            }
        }

        if saw_unknown {
            Ok(KleeneValue::Unknown)
        } else {
            Ok(KleeneValue::True)
        }
    }
}

/// First exact mismatch between a compiled conjunction and a generic program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KleeneConjunctionMismatch {
    /// Canonical base-3 assignment index, with input 0 as the least significant trit.
    pub assignment_index: usize,
    /// Exact input values for the mismatching assignment.
    pub inputs: Vec<KleeneValue>,
    /// Output produced by the compiled conjunction.
    pub compiled: KleeneValue,
    /// Output produced by the generic postfix program.
    pub generic: KleeneValue,
}

/// Result of an exact bounded compiled-vs-generic comparison.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KleeneConjunctionComparison {
    /// Both evaluators agree on every row in the declared exhaustive domain.
    Equivalent { rows: usize },
    /// The evaluators differ on the first canonical row described by `witness`.
    Different {
        rows: usize,
        witness: KleeneConjunctionMismatch,
    },
}

/// Compare a compiled conjunction with a generic Strong-Kleene postfix program.
///
/// The comparison enumerates the complete `3^n` domain in canonical base-3
/// order and preserves `Unknown` as a distinct value. `max_rows` bounds the
/// exhaustive domain before allocation. `max_work_units` bounds a conservative
/// deterministic estimate of per-row work: every generic postfix instruction
/// counts as one unit and each compiled input contributes two mask tests. A
/// budget error is therefore an explicit non-result rather than evidence for
/// or against equivalence.
///
/// # Errors
///
/// Returns [`KleeneConjunctionComparisonError`] when enumeration or work bounds
/// cannot be satisfied, or when either evaluator rejects the input/program.
pub fn compare_compiled_conjunction_with_program(
    compiled: CompiledKleeneConjunction,
    generic: &[KleeneInstruction],
    max_rows: usize,
    max_work_units: usize,
) -> Result<KleeneConjunctionComparison, KleeneConjunctionComparisonError> {
    let input_arity = compiled.input_arity();
    let rows = checked_pow3(input_arity)
        .ok_or(KleeneConjunctionComparisonError::EnumerationOverflow { input_arity })?;
    if rows > max_rows {
        return Err(KleeneConjunctionComparisonError::EnumerationLimitExceeded {
            required_rows: rows,
            max_rows,
        });
    }

    let compiled_work_per_row = input_arity.checked_mul(2).ok_or(
        KleeneConjunctionComparisonError::WorkEstimateOverflow {
            rows,
            generic_instructions: generic.len(),
            input_arity,
        },
    )?;
    let work_per_row = generic.len().checked_add(compiled_work_per_row).ok_or(
        KleeneConjunctionComparisonError::WorkEstimateOverflow {
            rows,
            generic_instructions: generic.len(),
            input_arity,
        },
    )?;
    let required_work_units = rows.checked_mul(work_per_row).ok_or(
        KleeneConjunctionComparisonError::WorkEstimateOverflow {
            rows,
            generic_instructions: generic.len(),
            input_arity,
        },
    )?;
    if required_work_units > max_work_units {
        return Err(KleeneConjunctionComparisonError::WorkLimitExceeded {
            required_work_units,
            max_work_units,
        });
    }

    let mut inputs = vec![KleeneValue::False; input_arity];
    for assignment_index in 0..rows {
        decode_assignment(assignment_index, &mut inputs);
        let compiled_output = compiled
            .evaluate(&inputs)
            .map_err(KleeneConjunctionComparisonError::CompiledEvaluation)?;
        let generic_output = evaluate_kleene_program(generic, &inputs)
            .map_err(KleeneConjunctionComparisonError::GenericEvaluation)?;

        if compiled_output != generic_output {
            return Ok(KleeneConjunctionComparison::Different {
                rows,
                witness: KleeneConjunctionMismatch {
                    assignment_index,
                    inputs: inputs.clone(),
                    compiled: compiled_output,
                    generic: generic_output,
                },
            });
        }
    }

    Ok(KleeneConjunctionComparison::Equivalent { rows })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assignment(mut index: usize, arity: usize) -> Vec<KleeneValue> {
        let mut inputs = vec![KleeneValue::False; arity];
        for value in &mut inputs {
            *value = KLEENE_VALUES[index % 3];
            index /= 3;
        }
        inputs
    }

    #[test]
    fn empty_conjunction_is_true() {
        let compiled = CompiledKleeneConjunction::compile(2, &[]).expect("empty mask is valid");
        for row in 0..9 {
            assert_eq!(
                compiled.evaluate(&assignment(row, 2)),
                Ok(KleeneValue::True)
            );
        }
    }

    #[test]
    fn duplicates_are_idempotent() {
        let compiled = CompiledKleeneConjunction::compile(
            2,
            &[KleeneLiteral::positive(0), KleeneLiteral::positive(0)],
        )
        .expect("duplicate literal is valid");
        assert_eq!(compiled.require_true_mask(), 1);
        assert!(!compiled.has_opposing_literals());
    }

    #[test]
    fn opposing_literals_preserve_strong_kleene_unknown() {
        let compiled = CompiledKleeneConjunction::compile(
            1,
            &[KleeneLiteral::positive(0), KleeneLiteral::negative(0)],
        )
        .expect("opposing literals remain representable");
        assert!(compiled.has_opposing_literals());
        assert_eq!(
            compiled.evaluate(&[KleeneValue::False]),
            Ok(KleeneValue::False)
        );
        assert_eq!(
            compiled.evaluate(&[KleeneValue::Unknown]),
            Ok(KleeneValue::Unknown)
        );
        assert_eq!(
            compiled.evaluate(&[KleeneValue::True]),
            Ok(KleeneValue::False)
        );
    }

    #[test]
    fn compiled_conjunction_matches_generic_program_exhaustively() {
        let literals = [
            KleeneLiteral::positive(0),
            KleeneLiteral::negative(1),
            KleeneLiteral::positive(2),
        ];
        let compiled = CompiledKleeneConjunction::compile(3, &literals)
            .expect("three-input conjunction must compile");
        let generic = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(1),
            KleeneInstruction::Not,
            KleeneInstruction::And,
            KleeneInstruction::Input(2),
            KleeneInstruction::And,
        ];

        for row in 0..27 {
            let inputs = assignment(row, 3);
            let expected =
                evaluate_kleene_program(&generic, &inputs).expect("generic program is valid");
            assert_eq!(
                compiled.evaluate(&inputs),
                Ok(expected),
                "row {row}: {inputs:?}"
            );
        }
    }

    #[test]
    fn exact_comparison_certifies_compiled_generic_equivalence() {
        let compiled = CompiledKleeneConjunction::compile(
            3,
            &[
                KleeneLiteral::positive(0),
                KleeneLiteral::negative(1),
                KleeneLiteral::positive(2),
            ],
        )
        .expect("conjunction compiles");
        let generic = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(1),
            KleeneInstruction::Not,
            KleeneInstruction::And,
            KleeneInstruction::Input(2),
            KleeneInstruction::And,
        ];

        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &generic, 27, 324),
            Ok(KleeneConjunctionComparison::Equivalent { rows: 27 })
        );
    }

    #[test]
    fn exact_comparison_returns_first_canonical_witness() {
        let compiled = CompiledKleeneConjunction::compile(1, &[KleeneLiteral::positive(0)])
            .expect("conjunction compiles");
        let generic = [KleeneInstruction::Input(0), KleeneInstruction::Not];

        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &generic, 3, 12),
            Ok(KleeneConjunctionComparison::Different {
                rows: 3,
                witness: KleeneConjunctionMismatch {
                    assignment_index: 0,
                    inputs: vec![KleeneValue::False],
                    compiled: KleeneValue::False,
                    generic: KleeneValue::True,
                },
            })
        );
    }

    #[test]
    fn exact_comparison_preserves_opposing_literal_unknown() {
        let compiled = CompiledKleeneConjunction::compile(
            1,
            &[KleeneLiteral::positive(0), KleeneLiteral::negative(0)],
        )
        .expect("opposing conjunction compiles");
        let generic = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(0),
            KleeneInstruction::Not,
            KleeneInstruction::And,
        ];

        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &generic, 3, 18),
            Ok(KleeneConjunctionComparison::Equivalent { rows: 3 })
        );
    }

    #[test]
    fn exact_comparison_limits_enumeration_before_allocation() {
        let compiled = CompiledKleeneConjunction::compile(4, &[]).expect("arity is representable");
        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &[], 80, usize::MAX),
            Err(KleeneConjunctionComparisonError::EnumerationLimitExceeded {
                required_rows: 81,
                max_rows: 80,
            })
        );
    }

    #[test]
    fn exact_comparison_work_budget_is_non_result() {
        let compiled = CompiledKleeneConjunction::compile(3, &[]).expect("arity is representable");
        let generic = [KleeneInstruction::Constant(KleeneValue::True)];
        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &generic, 27, 188),
            Err(KleeneConjunctionComparisonError::WorkLimitExceeded {
                required_work_units: 189,
                max_work_units: 188,
            })
        );
    }

    #[test]
    fn exact_comparison_identifies_malformed_generic_program() {
        let compiled = CompiledKleeneConjunction::compile(0, &[]).expect("empty conjunction valid");
        assert_eq!(
            compare_compiled_conjunction_with_program(compiled, &[KleeneInstruction::And], 1, 1,),
            Err(KleeneConjunctionComparisonError::GenericEvaluation(
                KleeneEvalError::StackUnderflow
            ))
        );
    }

    #[test]
    fn false_dominates_unknown_in_compiled_conjunction() {
        let compiled = CompiledKleeneConjunction::compile(
            2,
            &[KleeneLiteral::positive(0), KleeneLiteral::positive(1)],
        )
        .expect("conjunction is valid");
        assert_eq!(
            compiled.evaluate(&[KleeneValue::Unknown, KleeneValue::False]),
            Ok(KleeneValue::False)
        );
        assert_eq!(
            compiled.evaluate(&[KleeneValue::Unknown, KleeneValue::True]),
            Ok(KleeneValue::Unknown)
        );
    }

    #[test]
    fn bounds_fail_closed() {
        assert_eq!(
            CompiledKleeneConjunction::compile(
                MAX_COMPILED_KLEENE_INPUTS + 1,
                &[KleeneLiteral::positive(0)],
            ),
            Err(KleeneConjunctionError::InputArityTooLarge {
                input_arity: MAX_COMPILED_KLEENE_INPUTS + 1,
                max: MAX_COMPILED_KLEENE_INPUTS,
            })
        );
        assert_eq!(
            CompiledKleeneConjunction::compile(2, &[KleeneLiteral::positive(2)]),
            Err(KleeneConjunctionError::InputOutOfRange {
                input: 2,
                input_arity: 2,
            })
        );

        let compiled = CompiledKleeneConjunction::compile(2, &[]).expect("valid arity");
        assert_eq!(
            compiled.evaluate(&[KleeneValue::False]),
            Err(KleeneConjunctionError::InputLengthMismatch {
                expected: 2,
                actual: 1,
            })
        );
    }
}
