//! Bounded BL-13 input-permutation equivalence screen.
//!
//! This module extends the exact/output-complement first-stage screen with one
//! additional declared relation: permutation of input variables, optionally
//! followed by output complementation. It intentionally does not claim affine,
//! extended-affine (EA), CCZ, algebraic, or circuit equivalence.

use crate::BooleanFunction;
use crate::equivalence_screen::{ScreenedEquivalence, screen_exact_or_complement};

/// Maximum input width for exhaustive permutation screening.
///
/// BL-13 currently screens scalar `F_2^8 -> F_2` candidates. Keeping the bound
/// explicit prevents accidental factorial work at larger widths.
pub const MAX_EXHAUSTIVE_PERMUTATION_BITS: u32 = 8;

/// The strongest relation demonstrated by the expanded screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PermutationScreenedEquivalence {
    /// Same complete truth table.
    Exact,
    /// Equal after global output complementation.
    OutputComplement,
    /// Equal after renaming/permuting input variables.
    InputPermutation { permutation: Vec<u32> },
    /// Equal after input permutation and global output complementation.
    InputPermutationWithOutputComplement { permutation: Vec<u32> },
}

/// A bounded-reference match and the relation that established it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PermutationEquivalenceMatch {
    pub reference_index: usize,
    pub relation: PermutationScreenedEquivalence,
}

/// Errors that make exhaustive permutation screening invalid or unsafe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PermutationScreenError {
    /// Exhaustive input permutations would exceed the declared bound.
    InputWidthTooLarge { width: u32, maximum: u32 },
}

/// Screens one scalar Boolean function under exact, complement, and bounded
/// input-permutation equivalence.
///
/// Exact and output-complement matches retain precedence. If neither relation
/// matches, every non-identity input permutation is enumerated deterministically
/// in lexicographic order. Different input widths are skipped.
///
/// A successful permutation result proves only variable renaming (with optional
/// global output complementation) against this supplied corpus. `Ok(None)` is not
/// evidence of novelty and does not establish affine, EA, CCZ, algebraic, or
/// circuit non-equivalence.
///
/// # Errors
///
/// Returns [`PermutationScreenError::InputWidthTooLarge`] when the candidate
/// width exceeds [`MAX_EXHAUSTIVE_PERMUTATION_BITS`].
pub fn screen_with_input_permutations(
    candidate: &BooleanFunction,
    reference: &[BooleanFunction],
) -> Result<Option<PermutationEquivalenceMatch>, PermutationScreenError> {
    if candidate.input_bits() > MAX_EXHAUSTIVE_PERMUTATION_BITS {
        return Err(PermutationScreenError::InputWidthTooLarge {
            width: candidate.input_bits(),
            maximum: MAX_EXHAUSTIVE_PERMUTATION_BITS,
        });
    }

    if let Some(first_stage) = screen_exact_or_complement(candidate, reference) {
        let relation = match first_stage.relation {
            ScreenedEquivalence::Exact => PermutationScreenedEquivalence::Exact,
            ScreenedEquivalence::OutputComplement => {
                PermutationScreenedEquivalence::OutputComplement
            }
        };
        return Ok(Some(PermutationEquivalenceMatch {
            reference_index: first_stage.reference_index,
            relation,
        }));
    }

    let width = candidate.input_bits();
    let mut permutation: Vec<u32> = (0..width).collect();
    while next_permutation(&mut permutation) {
        for (reference_index, known) in reference.iter().enumerate() {
            if known.input_bits() != width {
                continue;
            }
            if matches_under_permutation(candidate, known, &permutation, false) {
                return Ok(Some(PermutationEquivalenceMatch {
                    reference_index,
                    relation: PermutationScreenedEquivalence::InputPermutation {
                        permutation: permutation.clone(),
                    },
                }));
            }
            if matches_under_permutation(candidate, known, &permutation, true) {
                return Ok(Some(PermutationEquivalenceMatch {
                    reference_index,
                    relation:
                        PermutationScreenedEquivalence::InputPermutationWithOutputComplement {
                            permutation: permutation.clone(),
                        },
                }));
            }
        }
    }

    Ok(None)
}

fn matches_under_permutation(
    candidate: &BooleanFunction,
    known: &BooleanFunction,
    permutation: &[u32],
    complement_output: bool,
) -> bool {
    candidate
        .truth_table()
        .iter()
        .enumerate()
        .all(|(row, &candidate_value)| {
            let source_row = remap_row(row, permutation);
            let mut known_value = known.truth_table()[source_row];
            if complement_output {
                known_value ^= 1;
            }
            known_value == candidate_value
        })
}

fn remap_row(row: usize, permutation: &[u32]) -> usize {
    let mut remapped = 0usize;
    for (target_bit, &source_bit) in permutation.iter().enumerate() {
        if row & (1usize << target_bit) != 0 {
            remapped |= 1usize << source_bit;
        }
    }
    remapped
}

fn next_permutation(values: &mut [u32]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };

    let successor = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
        .expect("pivot guarantees a successor");
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(bits: &[u8]) -> BooleanFunction {
        BooleanFunction::new(2, bits.to_vec()).unwrap()
    }

    #[test]
    fn exact_match_keeps_precedence() {
        let candidate = function(&[0, 1, 0, 1]);
        let reference = vec![candidate.clone()];
        assert_eq!(
            screen_with_input_permutations(&candidate, &reference).unwrap(),
            Some(PermutationEquivalenceMatch {
                reference_index: 0,
                relation: PermutationScreenedEquivalence::Exact,
            })
        );
    }

    #[test]
    fn finds_input_variable_swap() {
        let x1 = function(&[0, 0, 1, 1]);
        let x0 = function(&[0, 1, 0, 1]);
        assert_eq!(
            screen_with_input_permutations(&x1, &[x0]).unwrap(),
            Some(PermutationEquivalenceMatch {
                reference_index: 0,
                relation: PermutationScreenedEquivalence::InputPermutation {
                    permutation: vec![1, 0],
                },
            })
        );
    }

    #[test]
    fn finds_input_swap_with_output_complement() {
        let not_x1 = function(&[1, 1, 0, 0]);
        let x0 = function(&[0, 1, 0, 1]);
        assert_eq!(
            screen_with_input_permutations(&not_x1, &[x0]).unwrap(),
            Some(PermutationEquivalenceMatch {
                reference_index: 0,
                relation:
                    PermutationScreenedEquivalence::InputPermutationWithOutputComplement {
                        permutation: vec![1, 0],
                    },
            })
        );
    }

    #[test]
    fn absence_remains_bounded_none() {
        let and = function(&[0, 0, 0, 1]);
        let x0 = function(&[0, 1, 0, 1]);
        assert_eq!(screen_with_input_permutations(&and, &[x0]).unwrap(), None);
    }

    #[test]
    fn refuses_factorial_work_above_declared_bound() {
        let width = MAX_EXHAUSTIVE_PERMUTATION_BITS + 1;
        let candidate = BooleanFunction::new(width, vec![0; 1usize << width]).unwrap();
        assert_eq!(
            screen_with_input_permutations(&candidate, &[]),
            Err(PermutationScreenError::InputWidthTooLarge {
                width,
                maximum: MAX_EXHAUSTIVE_PERMUTATION_BITS,
            })
        );
    }
}
