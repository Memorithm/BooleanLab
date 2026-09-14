//! Exact compiled conjunction masks for bounded Strong-Kleene differential tests.
//!
//! This BL-BE1 oracle compiles positive and negative literals into `u64`
//! masks for exhaustive equivalence checks against generic expression
//! evaluation. It makes no production-runtime or performance claim.

use crate::kleene::KleeneValue;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kleene::{KLEENE_VALUES, KleeneInstruction, evaluate_kleene_program};

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
