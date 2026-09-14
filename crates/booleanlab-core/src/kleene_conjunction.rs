//! Exact compiled conjunction masks for bounded Strong-Kleene differential tests.
//!
//! This module is an experimental BL-BE1 oracle. It provides a compact `u64`
//! representation for conjunctions of positive and negative literals and is
//! intended for exhaustive equivalence checks against generic expression
//! evaluation. It makes no performance claim about production runtimes.

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
    /// Evaluation received fewer values than the compiled arity requires.
    InputLengthMismatch { expected: usize, actual: usize },
}

/// One literal in a conjunction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneLiteral {
    /// Input index addressed by the literal.
    pub input: usize,
    /// When true, require the input to be `True`; otherwise require `False`.
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
///
/// `require_true_mask` and `require_false_mask` record which inputs are
/// required to be respectively `True` or `False`. If the same bit appears in
/// both masks, the conjunction is contradictory for every exhaustive row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledKleeneConjunction {
    input_arity: usize,
    require_true_mask: u64,
    require_false_mask: u64,
}

impl CompiledKleeneConjunction {
    /// Compile a literal conjunction into two bounded `u64` masks.
    ///
    /// Duplicate literals are idempotent. Opposite literals for the same input
    /// are preserved as an explicit contradiction rather than simplified away.
    ///
    /// # Errors
    ///
    /// Returns [`KleeneConjunctionError::InputArityTooLarge`] above the `u64`
    /// bound, or [`KleeneConjunctionError::InputOutOfRange`] when a literal
    /// addresses an undeclared input.
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

        let mut require_true_mask = 0u64;
        let mut require_false_mask = 0u64;
        for literal in literals {
            if literal.input >= input_arity {
                return Err(KleeneConjunctionError::InputOutOfRange {
                    input: literal.input,
                    input_arity,
                });
            }
            let bit = 1u64 << literal.input;
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

    /// Whether the compiled conjunction contains both polarities of one input.
    #[must_use]
    pub const fn is_contradictory(self) -> bool {
        (self.require_true_mask & self.require_false_mask) != 0
    }

    /// Evaluate the compiled conjunction under Strong-Kleene semantics.
    ///
    /// Evaluation returns `False` as soon as any known input violates a
    /// literal. Otherwise it returns `Unknown` when at least one required input
    /// is unknown, and `True` only when every literal is known and satisfied.
    /// The empty conjunction therefore evaluates to `True`.
    ///
    /// # Errors
    ///
    /// Returns [`KleeneConjunctionError::InputLengthMismatch`] unless the input
    /// slice length exactly matches the declared arity.
    pub fn evaluate(self, inputs: &[KleeneValue]) -> Result<KleeneValue, KleeneConjunctionError> {
        if inputs.len() != self.input_arity {
            return Err(KleeneConjunctionError::InputLengthMismatch {
                expected: self.input_arity,
                actual: inputs.len(),
            });
        }

        if self.is_contradictory() {
            return Ok(KleeneValue::False);
        }

        let mut saw_unknown = false;
        for (input, value) in inputs.iter().copied().enumerate() {
            let bit = 1u64 << input;
            if (self.require_true_mask & bit) != 0 {
                match value {
                    KleeneValue::False => return Ok(KleeneValue::False),
                    KleeneValue::Unknown => saw_unknown = true,
                    KleeneValue::True => {}
                }
            }
            if (self.require_false_mask & bit) != 0 {
                match value {
                    KleeneValue::True => return Ok(KleeneValue::False),
                    KleeneValue::Unknown => saw_unknown = true,
                    KleeneValue::False => {}
                }
            }
        }

        Ok(if saw_unknown {
            KleeneValue::Unknown
        } else {
            KleeneValue::True
        })
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
            assert_eq!(compiled.evaluate(&assignment(row, 2)), Ok(KleeneValue::True));
        }
    }

    #[test]
    fn duplicates_are_idempotent_and_opposites_are_contradictory() {
        let duplicate = CompiledKleeneConjunction::compile(
            2,
            &[KleeneLiteral::positive(0), KleeneLiteral::positive(0)],
        )
        .expect("duplicate literal is valid");
        assert_eq!(duplicate.require_true_mask(), 1);
        assert!(!duplicate.is_contradictory());

        let opposite = CompiledKleeneConjunction::compile(
            1,
            &[KleeneLiteral::positive(0), KleeneLiteral::negative(0)],
        )
        .expect("opposite literals remain representable");
        assert!(opposite.is_contradictory());
        for value in KLEENE_VALUES {
            assert_eq!(opposite.evaluate(&[value]), Ok(KleeneValue::False));
        }
    }

    #[test]
    fn compiled_conjunction_matches_generic_program_exhaustively() {
        // x0 AND NOT(x1) AND x2
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
            assert_eq!(
                compiled.evaluate(&inputs),
                Ok(evaluate_kleene_program(&generic, &inputs).expect("generic program is valid")),
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
                &[KleeneLiteral::positive(0)]
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
