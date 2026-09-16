//! Exact bounded individual-redundancy analysis for BL-BE4 pseudo-Boolean constraints.
//!
//! A constraint is individually redundant when the conjunction of every other
//! constraint entails it over the complete shared Boolean domain. The result is
//! a reference oracle for small-domain analysis only: several constraints may be
//! individually redundant without being safe to delete simultaneously. Reported
//! indices are diagnostic evidence and never an automatic rewrite plan.

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
    /// Conservative total-work accounting overflowed before any target check.
    WorkAccountingOverflow,
    /// The complete redundancy screen exceeds its total predicate-evaluation limit.
    WorkLimitExceeded {
        required_evaluations: u128,
        limit: u128,
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
            Self::WorkAccountingOverflow => {
                formatter.write_str("pseudo-Boolean redundancy work accounting overflowed")
            }
            Self::WorkLimitExceeded {
                required_evaluations,
                limit,
            } => write!(
                formatter,
                "pseudo-Boolean redundancy screen requires at most {required_evaluations} predicate evaluations; limit is {limit}"
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
            Self::ArityMismatch { .. }
            | Self::WorkAccountingOverflow
            | Self::WorkLimitExceeded { .. } => None,
            Self::Entailment { source, .. } => Some(source),
        }
    }
}

/// Return the indices of constraints that are individually entailed by all of
/// the other constraints under the default total work budget.
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
/// do not share one Boolean domain, a work-accounting error on overflow, a
/// work-limit error before any cloned antecedent is constructed, or an entailment
/// error when exact bounded analysis for one target cannot be completed. Resource
/// exhaustion is a non-result rather than evidence of redundancy.
pub fn pseudo_boolean_redundant_indices(
    constraints: &[PseudoBooleanConstraint],
) -> Result<Vec<usize>, PseudoBooleanRedundancyError> {
    pseudo_boolean_redundant_indices_with_work_limit(
        constraints,
        DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS,
    )
}

/// Return individually redundant constraint indices with an explicit
/// conservative total predicate-evaluation limit for the complete screen.
///
/// For `n` constraints over arity `a`, every target check has `n - 1`
/// antecedents plus one consequent. The conservative total upper bound is thus
/// `2^a * n^2`. It is checked before constructing any cloned antecedent vector,
/// so a large constraint set cannot evade the budget by resetting a per-target
/// limit.
///
/// # Errors
///
/// Returns the same failures as [`pseudo_boolean_redundant_indices`].
pub fn pseudo_boolean_redundant_indices_with_work_limit(
    constraints: &[PseudoBooleanConstraint],
    max_evaluations: u128,
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

    let shift = u32::try_from(expected_arity)
        .map_err(|_| PseudoBooleanRedundancyError::WorkAccountingOverflow)?;
    let assignments = 1u128
        .checked_shl(shift)
        .ok_or(PseudoBooleanRedundancyError::WorkAccountingOverflow)?;
    let constraint_count = u128::try_from(constraints.len())
        .map_err(|_| PseudoBooleanRedundancyError::WorkAccountingOverflow)?;
    let required_evaluations = assignments
        .checked_mul(constraint_count)
        .and_then(|work| work.checked_mul(constraint_count))
        .ok_or(PseudoBooleanRedundancyError::WorkAccountingOverflow)?;
    if required_evaluations > max_evaluations {
        return Err(PseudoBooleanRedundancyError::WorkLimitExceeded {
            required_evaluations,
            limit: max_evaluations,
        });
    }

    let per_target_evaluations = assignments
        .checked_mul(constraint_count)
        .ok_or(PseudoBooleanRedundancyError::WorkAccountingOverflow)?;
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
            per_target_evaluations,
        )
        .map_err(|source| PseudoBooleanRedundancyError::Entailment {
            target_index,
            source,
        })?;
        if matches!(result, PseudoBooleanConjunctionImplication::Entails { .. }) {
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
        assert_eq!(
            pseudo_boolean_redundant_indices(&[non_tautology]),
            Ok(vec![])
        );
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
    fn total_work_limit_exhaustion_is_a_non_result_before_cloning() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_redundant_indices_with_work_limit(&[at_least_one, at_least_two], 31),
            Err(PseudoBooleanRedundancyError::WorkLimitExceeded {
                required_evaluations: 32,
                limit: 31,
            })
        );
    }

    #[test]
    fn total_budget_accounts_for_every_target_check() {
        let tautology =
            PseudoBooleanConstraint::cardinality(0, 0, PseudoBooleanRelation::Exactly).unwrap();
        let constraints = vec![tautology; 8];

        assert_eq!(
            pseudo_boolean_redundant_indices_with_work_limit(&constraints, 63),
            Err(PseudoBooleanRedundancyError::WorkLimitExceeded {
                required_evaluations: 64,
                limit: 63,
            })
        );
    }
}
