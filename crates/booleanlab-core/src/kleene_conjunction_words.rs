//! Bounded multiword Strong-Kleene conjunction masks for BL-BE2.
//!
//! This representation extends the exact compiled conjunction oracle beyond
//! one `u64` without making a production-runtime or performance claim.

use crate::kleene::KleeneValue;
use crate::kleene_conjunction::KleeneLiteral;

/// Conservative bound for the experimental multiword representation.
pub const MAX_MULTIWORD_KLEENE_INPUTS: usize = 4096;

/// Failure while compiling or evaluating a multiword conjunction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MultiwordKleeneConjunctionError {
    /// The declared input arity exceeds the experimental bound.
    InputArityTooLarge { input_arity: usize, max: usize },
    /// A literal references an input outside the declared arity.
    InputOutOfRange { input: usize, input_arity: usize },
    /// Evaluation input length differs from the compiled arity.
    InputLengthMismatch { expected: usize, actual: usize },
}

/// Compiled conjunction backed by one or more `u64` words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompiledMultiwordKleeneConjunction {
    input_arity: usize,
    require_true_words: Vec<u64>,
    require_false_words: Vec<u64>,
}

impl CompiledMultiwordKleeneConjunction {
    /// Compile Strong-Kleene literals into bounded multiword masks.
    ///
    /// Duplicate literals are idempotent. Opposite literals remain explicit,
    /// because `x AND NOT(x)` evaluates to `Unknown` when `x` is `Unknown`.
    ///
    /// # Errors
    ///
    /// Returns [`MultiwordKleeneConjunctionError::InputArityTooLarge`] above
    /// [`MAX_MULTIWORD_KLEENE_INPUTS`], or
    /// [`MultiwordKleeneConjunctionError::InputOutOfRange`] for an undeclared
    /// input.
    pub fn compile(
        input_arity: usize,
        literals: &[KleeneLiteral],
    ) -> Result<Self, MultiwordKleeneConjunctionError> {
        if input_arity > MAX_MULTIWORD_KLEENE_INPUTS {
            return Err(MultiwordKleeneConjunctionError::InputArityTooLarge {
                input_arity,
                max: MAX_MULTIWORD_KLEENE_INPUTS,
            });
        }

        let word_count = input_arity.div_ceil(u64::BITS as usize);
        let mut require_true_words = vec![0_u64; word_count];
        let mut require_false_words = vec![0_u64; word_count];

        for literal in literals {
            if literal.input >= input_arity {
                return Err(MultiwordKleeneConjunctionError::InputOutOfRange {
                    input: literal.input,
                    input_arity,
                });
            }
            let word = literal.input / u64::BITS as usize;
            let bit = 1_u64 << (literal.input % u64::BITS as usize);
            if literal.require_true {
                require_true_words[word] |= bit;
            } else {
                require_false_words[word] |= bit;
            }
        }

        Ok(Self {
            input_arity,
            require_true_words,
            require_false_words,
        })
    }

    /// Declared input arity.
    #[must_use]
    pub const fn input_arity(&self) -> usize {
        self.input_arity
    }

    /// Number of packed words used by each polarity mask.
    #[must_use]
    pub fn word_count(&self) -> usize {
        self.require_true_words.len()
    }

    /// Whether at least one input appears with both polarities.
    #[must_use]
    pub fn has_opposing_literals(&self) -> bool {
        self.require_true_words
            .iter()
            .zip(&self.require_false_words)
            .any(|(true_word, false_word)| true_word & false_word != 0)
    }

    /// Evaluate the compiled conjunction under Strong-Kleene semantics.
    ///
    /// # Errors
    ///
    /// Returns [`MultiwordKleeneConjunctionError::InputLengthMismatch`] unless
    /// the supplied input length exactly matches the compiled arity.
    pub fn evaluate(
        &self,
        inputs: &[KleeneValue],
    ) -> Result<KleeneValue, MultiwordKleeneConjunctionError> {
        if inputs.len() != self.input_arity {
            return Err(MultiwordKleeneConjunctionError::InputLengthMismatch {
                expected: self.input_arity,
                actual: inputs.len(),
            });
        }

        let mut saw_unknown = false;
        for (input, value) in inputs.iter().copied().enumerate() {
            let word = input / u64::BITS as usize;
            let bit = 1_u64 << (input % u64::BITS as usize);

            if self.require_true_words[word] & bit != 0 {
                match value {
                    KleeneValue::False => return Ok(KleeneValue::False),
                    KleeneValue::Unknown => saw_unknown = true,
                    KleeneValue::True => {}
                }
            }
            if self.require_false_words[word] & bit != 0 {
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
    use crate::kleene::{KleeneInstruction, evaluate_kleene_program};

    fn inputs(value_0: KleeneValue, value_64: KleeneValue) -> Vec<KleeneValue> {
        let mut values = vec![KleeneValue::True; 65];
        values[0] = value_0;
        values[64] = value_64;
        values
    }

    #[test]
    fn crosses_the_u64_word_boundary_exactly() {
        let compiled = CompiledMultiwordKleeneConjunction::compile(
            65,
            &[KleeneLiteral::positive(0), KleeneLiteral::negative(64)],
        )
        .expect("65-input conjunction is within the multiword bound");
        assert_eq!(compiled.input_arity(), 65);
        assert_eq!(compiled.word_count(), 2);

        let generic = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(64),
            KleeneInstruction::Not,
            KleeneInstruction::And,
        ];
        for value_0 in [KleeneValue::False, KleeneValue::Unknown, KleeneValue::True] {
            for value_64 in [KleeneValue::False, KleeneValue::Unknown, KleeneValue::True] {
                let assignment = inputs(value_0, value_64);
                let expected = evaluate_kleene_program(&generic, &assignment)
                    .expect("generic program is valid");
                assert_eq!(compiled.evaluate(&assignment), Ok(expected));
            }
        }
    }

    #[test]
    fn opposing_literals_preserve_unknown_across_words() {
        let compiled = CompiledMultiwordKleeneConjunction::compile(
            65,
            &[KleeneLiteral::positive(64), KleeneLiteral::negative(64)],
        )
        .expect("opposing literals remain representable");
        assert!(compiled.has_opposing_literals());

        let mut assignment = vec![KleeneValue::False; 65];
        assignment[64] = KleeneValue::Unknown;
        assert_eq!(compiled.evaluate(&assignment), Ok(KleeneValue::Unknown));
    }

    #[test]
    fn bounds_fail_closed() {
        assert_eq!(
            CompiledMultiwordKleeneConjunction::compile(
                MAX_MULTIWORD_KLEENE_INPUTS + 1,
                &[KleeneLiteral::positive(0)],
            ),
            Err(MultiwordKleeneConjunctionError::InputArityTooLarge {
                input_arity: MAX_MULTIWORD_KLEENE_INPUTS + 1,
                max: MAX_MULTIWORD_KLEENE_INPUTS,
            })
        );
        assert_eq!(
            CompiledMultiwordKleeneConjunction::compile(65, &[KleeneLiteral::positive(65)]),
            Err(MultiwordKleeneConjunctionError::InputOutOfRange {
                input: 65,
                input_arity: 65,
            })
        );

        let compiled =
            CompiledMultiwordKleeneConjunction::compile(65, &[]).expect("valid bounded arity");
        assert_eq!(
            compiled.evaluate(&[KleeneValue::False; 64]),
            Err(MultiwordKleeneConjunctionError::InputLengthMismatch {
                expected: 65,
                actual: 64,
            })
        );
    }
}
