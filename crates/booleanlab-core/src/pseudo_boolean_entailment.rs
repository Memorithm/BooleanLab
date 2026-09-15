//! Exact bounded implication for BL-BE4 pseudo-Boolean constraints.
//!
//! This module provides a small-domain reference oracle for logical implication
//! between two weighted/cardinality predicates. It exhaustively checks the
//! complete shared Boolean domain under an explicit assignment budget and
//! returns the first violating assignment in canonical mask order.
//!
//! The oracle is deliberately not a SAT/PB solver, production runtime gate, or
//! performance claim. Work-limit exhaustion is an explicit non-result.

use crate::{
    DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS, PseudoBooleanConstraint, PseudoBooleanError,
};

/// First exact assignment showing that pseudo-Boolean implication does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PseudoBooleanImplicationWitness {
    /// Canonical Boolean assignment mask. Bit `i` is the value of variable `i`.
    pub assignment_mask: u64,
}

/// Result of exhaustive pseudo-Boolean implication over one declared domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoBooleanImplication {
    /// Every assignment satisfying the antecedent also satisfies the consequent.
    Entails {
        /// Complete domain size checked by the oracle.
        assignments: u128,
        /// Number of assignments satisfying the antecedent.
        antecedent_true_assignments: u128,
    },
    /// At least one antecedent-satisfying assignment violates the consequent.
    DoesNotEntail {
        /// Complete domain size declared by the shared arity.
        assignments: u128,
        /// First violating assignment in ascending mask order.
        witness: PseudoBooleanImplicationWitness,
    },
}

/// Exhaustively check whether `antecedent` implies `consequent` under the
/// default BL-BE4 assignment budget.
///
/// # Errors
///
/// Returns an error for mismatched arity, work-accounting overflow, work-limit
/// exhaustion, or an unexpected evaluation-shape failure.
pub fn pseudo_boolean_implies(
    antecedent: &PseudoBooleanConstraint,
    consequent: &PseudoBooleanConstraint,
) -> Result<PseudoBooleanImplication, PseudoBooleanError> {
    pseudo_boolean_implies_with_work_limit(
        antecedent,
        consequent,
        DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS,
    )
}

/// Exhaustively check pseudo-Boolean implication under an explicit assignment
/// budget.
///
/// The full domain size is computed and compared with `max_assignments` before
/// allocating the reusable assignment buffer or evaluating either predicate.
/// A work-limit failure is therefore a non-result, never entailment evidence.
///
/// # Errors
///
/// Returns an error for mismatched arity, work-accounting overflow, work-limit
/// exhaustion, or an unexpected evaluation-shape failure.
pub fn pseudo_boolean_implies_with_work_limit(
    antecedent: &PseudoBooleanConstraint,
    consequent: &PseudoBooleanConstraint,
    max_assignments: u128,
) -> Result<PseudoBooleanImplication, PseudoBooleanError> {
    if antecedent.arity() != consequent.arity() {
        return Err(PseudoBooleanError::ArityMismatch {
            left: antecedent.arity(),
            right: consequent.arity(),
        });
    }

    let shift = u32::try_from(antecedent.arity())
        .map_err(|_| PseudoBooleanError::WorkAccountingOverflow)?;
    let assignments = 1u128
        .checked_shl(shift)
        .ok_or(PseudoBooleanError::WorkAccountingOverflow)?;
    if assignments > max_assignments {
        return Err(PseudoBooleanError::WorkLimitExceeded {
            required: assignments,
            limit: max_assignments,
        });
    }

    let assignments_u64 =
        u64::try_from(assignments).map_err(|_| PseudoBooleanError::WorkAccountingOverflow)?;
    let mut assignment = vec![false; antecedent.arity()];
    let mut antecedent_true_assignments = 0u128;

    for mask in 0..assignments_u64 {
        for (index, value) in assignment.iter_mut().enumerate() {
            *value = mask & (1u64 << index) != 0;
        }

        if !antecedent.evaluate(&assignment)? {
            continue;
        }
        antecedent_true_assignments += 1;

        if !consequent.evaluate(&assignment)? {
            return Ok(PseudoBooleanImplication::DoesNotEntail {
                assignments,
                witness: PseudoBooleanImplicationWitness {
                    assignment_mask: mask,
                },
            });
        }
    }

    Ok(PseudoBooleanImplication::Entails {
        assignments,
        antecedent_true_assignments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PseudoBooleanRelation;

    #[test]
    fn stronger_cardinality_constraint_entails_weaker_one() {
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_implies(&at_least_two, &at_least_one),
            Ok(PseudoBooleanImplication::Entails {
                assignments: 8,
                antecedent_true_assignments: 4,
            })
        );
    }

    #[test]
    fn reverse_implication_returns_first_canonical_counterexample() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_implies(&at_least_one, &at_least_two),
            Ok(PseudoBooleanImplication::DoesNotEntail {
                assignments: 8,
                witness: PseudoBooleanImplicationWitness { assignment_mask: 1 },
            })
        );
    }

    #[test]
    fn exact_positive_scaling_implies_in_both_directions() {
        let original =
            PseudoBooleanConstraint::new(vec![2, 5, 7], 7, PseudoBooleanRelation::AtMost).unwrap();
        let scaled = original.scaled(9).unwrap();

        assert!(matches!(
            pseudo_boolean_implies(&original, &scaled),
            Ok(PseudoBooleanImplication::Entails { .. })
        ));
        assert!(matches!(
            pseudo_boolean_implies(&scaled, &original),
            Ok(PseudoBooleanImplication::Entails { .. })
        ));
    }

    #[test]
    fn mismatched_arity_and_insufficient_work_fail_closed() {
        let one =
            PseudoBooleanConstraint::cardinality(1, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let two =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        assert_eq!(
            pseudo_boolean_implies(&one, &two),
            Err(PseudoBooleanError::ArityMismatch { left: 1, right: 2 })
        );

        let large =
            PseudoBooleanConstraint::cardinality(63, 1, PseudoBooleanRelation::AtLeast).unwrap();
        assert_eq!(
            pseudo_boolean_implies_with_work_limit(&large, &large, 1024),
            Err(PseudoBooleanError::WorkLimitExceeded {
                required: 1u128 << 63,
                limit: 1024,
            })
        );
    }

    #[test]
    fn vacuous_antecedent_is_reported_exactly() {
        let contradiction =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::AtLeast).unwrap();
        let impossible_exact =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_implies(&contradiction, &impossible_exact),
            Ok(PseudoBooleanImplication::Entails {
                assignments: 4,
                antecedent_true_assignments: 0,
            })
        );
    }
}
