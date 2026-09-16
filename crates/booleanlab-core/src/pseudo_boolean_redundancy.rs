//! Exact bounded individual-redundancy analysis for BL-BE4 pseudo-Boolean constraints.
//!
//! A constraint is individually redundant when the conjunction of every other
//! constraint entails it over the complete shared Boolean domain. The result is
//! a reference oracle for small-domain analysis only: several constraints may be
//! individually redundant without being safe to delete simultaneously.

use core::fmt;

use crate::{
    DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS, PseudoBooleanConjunctionError,
    PseudoBooleanConjunctionImplication, PseudoBooleanConstraint,
    pseudo_boolean_conjunction_implies_with_work_limit,
};

/// Failure while checking individual redundancy in a constraint set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PseudoBooleanRedundancyError {
    /// One constraint has a different variable domain from the first constraint.
    ArityMismatch {
        constraint_index: usize,
        expected_arity: usize,
        actual_arity: usize,
    },
    /// The bounded implication oracle failed for one target constraint.
    Entailment {
        target_index: usize,
        source: PseudoBooleanConjunctionError,
    },
}

impl fmt::Display for PseudoBooleanRedundancyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArityMismatch {
                constraint_index,
                expected_arity,
                actual_arity,
            } => write!(
                formatter,
                "pseudo-Boolean constraint {constraint_index} has arity {actual_arity}; expected shared arity {expected_arity}"
            ),
            Self::Entailment {
                target_index,
                source,
            } => write!(
                formatter,
                "pseudo-Boolean redundancy check for constraint {target_index} failed: {source}"
            ),
        }
    }
}

impl std::error::Error for PseudoBooleanRedundancyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ArityMismatch { .. } => None,
            Self::Entailment { source, .. } => Some(source),
        }
    }
}

/// Return the indices of constraints that are individually entailed by all of
/// the other constraints under the default per-target work budget.
///
/// The empty set has no redundant constraints. For a singleton, the empty
/// antecedent is logical `true`, so the sole constraint is redundant exactly
/// when it is a tautology.
///
/// This function does **not** return a jointly removable subset. In particular,
/// duplicate or mutually implied constraints can each be individually redundant
/// even though deleting all reported indices at once would change the theory.
///
/// # Errors
///
/// Returns [`PseudoBooleanRedundancyError::ArityMismatch`] when the constraints
/// do not share one Boolean domain, or an entailment error when exact bounded
/// analysis for a target cannot be completed. Resource exhaustion is a
/// non-result rather than evidence of redundancy.
pub fn pseudo_boolean_redundant_indices(
    constraints: &[PseudoBooleanConstraint],
) -> Result<Vec<usize>, PseudoBooleanRedundancyError> {
    pseudo_boolean_redundant_indices_with_work_limit(
        constraints,
        DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS,
    )
}

/// Return individually redundant constraint indices with an explicit
/// conservative predicate-evaluation limit for each target check.
///
/// # Errors
///
/// Returns the same failures as [`pseudo_boolean_redundant_indices`].
pub fn pseudo_boolean_redundant_indices_with_work_limit(
    constraints: &[PseudoBooleanConstraint],
    max_evaluations_per_target: u128,
) -> Result<Vec<usize>, PseudoBooleanRedundancyError> {
    let Some(first) = constraints.first() else {
        return Ok(Vec::new());
    };
    let expected_arity = first.arity();
    for (constraint_index, constraint) in constraints.iter().enumerate().skip(1) {
        if constraint.arity() != expected_arity {
            return Err(PseudoBooleanRedundancyError::ArityMismatch {
                constraint_index,
                expected_arity,
                actual_arity: constraint.arity(),
            });
        }
    }

    let mut redundant = Vec::new();
    for target_index in 0..constraints.len() {
        let antecedents: Vec<PseudoBooleanConstraint> = constraints
            .iter()
            .enumerate()
            .filter(|(index, _)| *index != target_index)
            .map(|(_, constraint)| constraint.clone())
            .collect();
        let result = pseudo_boolean_conjunction_implies_with_work_limit(
            &antecedents,
            &constraints[target_index],
            max_evaluations_per_target,
        )
        .map_err(|source| PseudoBooleanRedundancyError::Entailment {
            target_index,
            source,
        })?;
        if matches!(
            result,
            PseudoBooleanConjunctionImplication::Entails { .. }
        ) {
            redundant.push(target_index);
        }
    }
    Ok(redundant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PseudoBooleanRelation;

    #[test]
    fn stronger_constraint_makes_weaker_constraint_redundant() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(2, 2, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_redundant_indices(&[at_least_one, at_least_two]),
            Ok(vec![0])
        );
    }

    #[test]
    fn singleton_tautology_is_redundant_but_non_tautology_is_not() {
        let tautology =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::AtMost).unwrap();
        let non_tautology =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(pseudo_boolean_redundant_indices(&[tautology]), Ok(vec![0]));
        assert_eq!(pseudo_boolean_redundant_indices(&[non_tautology]), Ok(vec![]));
    }

    #[test]
    fn duplicate_constraints_are_only_individually_redundant() {
        let exactly_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_redundant_indices(&[exactly_one.clone(), exactly_one]),
            Ok(vec![0, 1])
        );
    }

    #[test]
    fn mixed_arities_fail_closed_before_entailment_work() {
        let two =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let three =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_redundant_indices(&[two, three]),
            Err(PseudoBooleanRedundancyError::ArityMismatch {
                constraint_index: 1,
                expected_arity: 2,
                actual_arity: 3,
            })
        );
    }

    #[test]
    fn per_target_work_limit_exhaustion_is_a_non_result() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();

        assert!(matches!(
            pseudo_boolean_redundant_indices_with_work_limit(&[at_least_one, at_least_two], 15),
            Err(PseudoBooleanRedundancyError::Entailment {
                target_index: 0,
                source: PseudoBooleanConjunctionError::WorkLimitExceeded {
                    required_evaluations: 16,
                    limit: 15,
                },
            })
        ));
    }
}
