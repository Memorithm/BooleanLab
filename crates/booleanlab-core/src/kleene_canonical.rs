//! Exact bounded semantic-class canonicalization for Strong-Kleene programs.
//!
//! This is a `BooleanLab` research oracle for BL-BE3.  It groups a caller-owned
//! bounded candidate set by the collision-free exhaustive semantic key already
//! qualified by BL-BE1 and selects one deterministic structural representative
//! per exact semantic class.  It is not a simplifier, solver, runtime guard
//! representation, or actuation authority.

use std::collections::BTreeMap;

use crate::{KleeneInstruction, KleeneSemanticKey, KleeneSemanticKeyError, KleeneValue};

/// Maximum number of candidate programs accepted by one canonicalization call.
pub const MAX_KLEENE_CANONICAL_CANDIDATES: usize = 4_096;

/// One exact Strong-Kleene semantic class in canonical key order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KleeneCanonicalClass {
    /// Collision-free exhaustive semantic identity for this class.
    pub semantic_key: KleeneSemanticKey,
    /// Deterministic structural representative selected from caller candidates.
    pub representative: Vec<KleeneInstruction>,
    /// Number of caller candidates with this exact exhaustive semantics.
    pub equivalent_programs: usize,
}

/// Failure while grouping a bounded candidate set into exact semantic classes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KleeneCanonicalizationError {
    /// No candidate was supplied, so no semantic class can be constructed.
    EmptyCandidateSet,
    /// The candidate collection exceeds the explicit public bound.
    CandidateLimitExceeded {
        candidates: usize,
        max_candidates: usize,
    },
    /// `3^input_arity` cannot be represented as `usize`.
    EnumerationOverflow { input_arity: usize },
    /// The complete Strong-Kleene domain exceeds the caller-declared row bound.
    EnumerationLimitExceeded {
        required_rows: usize,
        max_rows: usize,
    },
    /// The total exact instruction-visit estimate cannot be represented.
    WorkEstimateOverflow,
    /// The complete candidate panel exceeds the caller-declared work bound.
    WorkLimitExceeded {
        required_instruction_evaluations: usize,
        max_instruction_evaluations: usize,
    },
    /// One candidate could not produce an exact exhaustive semantic key.
    Candidate {
        index: usize,
        error: KleeneSemanticKeyError,
    },
    /// Counting exact class membership overflowed `usize`.
    ClassCountOverflow,
}

#[derive(Clone)]
struct ClassBuilder {
    representative: Vec<KleeneInstruction>,
    rank: Vec<(u8, usize, u8)>,
    count: usize,
}

/// Group bounded Strong-Kleene programs by exact exhaustive semantics.
///
/// The entire candidate panel is costed before any semantic evaluation.  The
/// exact worst-case work estimate is `3^input_arity * sum(program.len())`.
/// Exhausting either the row or work budget is an explicit non-result and never
/// becomes evidence of equivalence or non-equivalence.
///
/// Within one exact semantic class the representative is selected by shortest
/// instruction count and then by a stable lexicographic instruction encoding.
/// This canonicalizes the *observed bounded candidate set* only; it does not
/// prove that the representative is globally minimal among all possible
/// Strong-Kleene expressions.
///
/// # Errors
///
/// Fails closed for an empty/oversized candidate panel, domain/work overflow or
/// exhaustion, or any malformed candidate program.
pub fn canonicalize_kleene_programs(
    programs: &[Vec<KleeneInstruction>],
    input_arity: usize,
    max_rows: usize,
    max_instruction_evaluations: usize,
) -> Result<Vec<KleeneCanonicalClass>, KleeneCanonicalizationError> {
    if programs.is_empty() {
        return Err(KleeneCanonicalizationError::EmptyCandidateSet);
    }
    if programs.len() > MAX_KLEENE_CANONICAL_CANDIDATES {
        return Err(KleeneCanonicalizationError::CandidateLimitExceeded {
            candidates: programs.len(),
            max_candidates: MAX_KLEENE_CANONICAL_CANDIDATES,
        });
    }

    let rows = checked_pow3(input_arity)
        .ok_or(KleeneCanonicalizationError::EnumerationOverflow { input_arity })?;
    if rows > max_rows {
        return Err(KleeneCanonicalizationError::EnumerationLimitExceeded {
            required_rows: rows,
            max_rows,
        });
    }

    let instructions = programs
        .iter()
        .try_fold(0_usize, |total, program| total.checked_add(program.len()));
    let required_instruction_evaluations = instructions
        .and_then(|instructions| rows.checked_mul(instructions))
        .ok_or(KleeneCanonicalizationError::WorkEstimateOverflow)?;
    if required_instruction_evaluations > max_instruction_evaluations {
        return Err(KleeneCanonicalizationError::WorkLimitExceeded {
            required_instruction_evaluations,
            max_instruction_evaluations,
        });
    }

    let mut classes = BTreeMap::<KleeneSemanticKey, ClassBuilder>::new();
    for (index, program) in programs.iter().enumerate() {
        let key =
            crate::kleene_semantic_key(program, input_arity, max_rows, max_instruction_evaluations)
                .map_err(|error| KleeneCanonicalizationError::Candidate { index, error })?;
        let rank = structural_rank(program);
        match classes.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(ClassBuilder {
                    representative: program.clone(),
                    rank,
                    count: 1,
                });
            }
            std::collections::btree_map::Entry::Occupied(mut entry) => {
                let class = entry.get_mut();
                class.count = class
                    .count
                    .checked_add(1)
                    .ok_or(KleeneCanonicalizationError::ClassCountOverflow)?;
                if (program.len(), &rank) < (class.representative.len(), &class.rank) {
                    class.representative.clone_from(program);
                    class.rank = rank;
                }
            }
        }
    }

    Ok(classes
        .into_iter()
        .map(|(semantic_key, class)| KleeneCanonicalClass {
            semantic_key,
            representative: class.representative,
            equivalent_programs: class.count,
        })
        .collect())
}

fn structural_rank(program: &[KleeneInstruction]) -> Vec<(u8, usize, u8)> {
    program
        .iter()
        .copied()
        .map(|instruction| match instruction {
            KleeneInstruction::Input(index) => (0, index, 0),
            KleeneInstruction::Constant(value) => (1, 0, value_rank(value)),
            KleeneInstruction::Not => (2, 0, 0),
            KleeneInstruction::And => (3, 0, 0),
            KleeneInstruction::Or => (4, 0, 0),
            KleeneInstruction::Xor => (5, 0, 0),
            KleeneInstruction::Implies => (6, 0, 0),
        })
        .collect()
}

const fn value_rank(value: KleeneValue) -> u8 {
    match value {
        KleeneValue::False => 0,
        KleeneValue::Unknown => 1,
        KleeneValue::True => 2,
    }
}

fn checked_pow3(exponent: usize) -> Option<usize> {
    let mut value = 1_usize;
    for _ in 0..exponent {
        value = value.checked_mul(3)?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(index: usize) -> KleeneInstruction {
        KleeneInstruction::Input(index)
    }

    #[test]
    fn exact_classes_choose_shortest_representative_and_keep_unknown_semantics() {
        let direct = vec![input(0)];
        let double_not = vec![input(0), KleeneInstruction::Not, KleeneInstruction::Not];
        let constant_unknown = vec![KleeneInstruction::Constant(KleeneValue::Unknown)];
        let classes = canonicalize_kleene_programs(
            &[double_not, constant_unknown.clone(), direct.clone()],
            1,
            3,
            64,
        )
        .unwrap();

        assert_eq!(classes.len(), 2);
        let identity = classes
            .iter()
            .find(|class| class.equivalent_programs == 2)
            .unwrap();
        assert_eq!(identity.representative, direct);
        let unknown = classes
            .iter()
            .find(|class| class.equivalent_programs == 1)
            .unwrap();
        assert_eq!(unknown.representative, constant_unknown);
    }

    #[test]
    fn equal_length_commutative_forms_use_stable_lexicographic_tie_break() {
        let forward = vec![input(0), input(1), KleeneInstruction::Or];
        let reverse = vec![input(1), input(0), KleeneInstruction::Or];
        let classes = canonicalize_kleene_programs(&[reverse, forward.clone()], 2, 9, 64).unwrap();

        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].equivalent_programs, 2);
        assert_eq!(classes[0].representative, forward);
    }

    #[test]
    fn classes_are_independent_of_candidate_input_order() {
        let identity = vec![input(0)];
        let negated = vec![input(0), KleeneInstruction::Not];
        let first =
            canonicalize_kleene_programs(&[identity.clone(), negated.clone()], 1, 3, 16).unwrap();
        let second = canonicalize_kleene_programs(&[negated, identity], 1, 3, 16).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn total_work_is_bounded_before_semantic_evaluation() {
        let programs = vec![vec![input(0)], vec![input(0), KleeneInstruction::Not]];
        assert_eq!(
            canonicalize_kleene_programs(&programs, 1, 3, 8),
            Err(KleeneCanonicalizationError::WorkLimitExceeded {
                required_instruction_evaluations: 9,
                max_instruction_evaluations: 8,
            })
        );
    }

    #[test]
    fn malformed_candidate_is_an_indexed_non_result() {
        let malformed = vec![KleeneInstruction::And];
        let error =
            canonicalize_kleene_programs(&[vec![input(0)], malformed], 1, 3, 32).unwrap_err();
        assert!(matches!(
            error,
            KleeneCanonicalizationError::Candidate { index: 1, .. }
        ));
    }

    #[test]
    fn empty_and_oversized_candidate_sets_fail_closed() {
        assert_eq!(
            canonicalize_kleene_programs(&[], 0, 1, 1),
            Err(KleeneCanonicalizationError::EmptyCandidateSet)
        );
        let oversized = vec![
            vec![KleeneInstruction::Constant(KleeneValue::True)];
            MAX_KLEENE_CANONICAL_CANDIDATES + 1
        ];
        assert_eq!(
            canonicalize_kleene_programs(&oversized, 0, 1, usize::MAX),
            Err(KleeneCanonicalizationError::CandidateLimitExceeded {
                candidates: MAX_KLEENE_CANONICAL_CANDIDATES + 1,
                max_candidates: MAX_KLEENE_CANONICAL_CANDIDATES,
            })
        );
    }
}
