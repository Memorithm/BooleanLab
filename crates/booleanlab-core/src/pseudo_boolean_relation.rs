//! Exact bounded set relations for BL-BE4 pseudo-Boolean constraints.
//!
//! This module classifies how the satisfying assignment sets of two
//! pseudo-Boolean constraints relate on one shared Boolean domain. It performs
//! one exhaustive pass under an explicit assignment budget and retains the first
//! canonical witness in each strict direction.
//!
//! The classification is a small-domain reference oracle only. It is not a
//! production rule-removal authorization, SAT/PB solver, performance claim, or
//! novelty result. Work-limit exhaustion remains an explicit non-result.

use crate::{DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS, PseudoBooleanConstraint, PseudoBooleanError};

/// First canonical assignment present in one satisfying set but not the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PseudoBooleanDifferenceWitness {
    /// Canonical Boolean assignment mask. Bit `i` is the value of variable `i`.
    pub assignment_mask: u64,
}

/// Exact relation between the satisfying assignment sets of two constraints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoBooleanSetRelation {
    /// Both constraints accept exactly the same assignments.
    Equivalent {
        /// Complete domain size checked by the oracle.
        assignments: u128,
        /// Number of assignments accepted by both constraints.
        satisfying_assignments: u128,
    },
    /// The antecedent accepts a strict subset of the consequent's assignments.
    AntecedentMoreRestrictive {
        /// Complete domain size checked by the oracle.
        assignments: u128,
        /// Number of assignments accepted by the antecedent.
        antecedent_true_assignments: u128,
        /// Number of assignments accepted by the consequent.
        consequent_true_assignments: u128,
        /// First assignment accepted only by the consequent.
        consequent_only_witness: PseudoBooleanDifferenceWitness,
    },
    /// The consequent accepts a strict subset of the antecedent's assignments.
    ConsequentMoreRestrictive {
        /// Complete domain size checked by the oracle.
        assignments: u128,
        /// Number of assignments accepted by the antecedent.
        antecedent_true_assignments: u128,
        /// Number of assignments accepted by the consequent.
        consequent_true_assignments: u128,
        /// First assignment accepted only by the antecedent.
        antecedent_only_witness: PseudoBooleanDifferenceWitness,
    },
    /// Each constraint accepts at least one assignment rejected by the other.
    Incomparable {
        /// Complete domain size checked by the oracle.
        assignments: u128,
        /// Number of assignments accepted by the antecedent.
        antecedent_true_assignments: u128,
        /// Number of assignments accepted by the consequent.
        consequent_true_assignments: u128,
        /// First assignment accepted only by the antecedent.
        antecedent_only_witness: PseudoBooleanDifferenceWitness,
        /// First assignment accepted only by the consequent.
        consequent_only_witness: PseudoBooleanDifferenceWitness,
    },
}

/// Classify the exact satisfying-set relation under the default BL-BE4 budget.
///
/// # Errors
///
/// Returns an error for mismatched arity, work-accounting overflow, work-limit
/// exhaustion, or an unexpected evaluation-shape failure.
pub fn pseudo_boolean_set_relation(
    antecedent: &PseudoBooleanConstraint,
    consequent: &PseudoBooleanConstraint,
) -> Result<PseudoBooleanSetRelation, PseudoBooleanError> {
    pseudo_boolean_set_relation_with_work_limit(
        antecedent,
        consequent,
        DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS,
    )
}

/// Classify the exact satisfying-set relation under an explicit assignment
/// budget.
///
/// The complete domain size is checked before the reusable assignment buffer is
/// allocated or either constraint is evaluated. Resource exhaustion therefore
/// remains a non-result rather than relation evidence.
///
/// # Errors
///
/// Returns an error for mismatched arity, work-accounting overflow, work-limit
/// exhaustion, or an unexpected evaluation-shape failure.
pub fn pseudo_boolean_set_relation_with_work_limit(
    antecedent: &PseudoBooleanConstraint,
    consequent: &PseudoBooleanConstraint,
    max_assignments: u128,
) -> Result<PseudoBooleanSetRelation, PseudoBooleanError> {
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
    let mut consequent_true_assignments = 0u128;
    let mut antecedent_only_witness = None;
    let mut consequent_only_witness = None;

    for mask in 0..assignments_u64 {
        for (index, value) in assignment.iter_mut().enumerate() {
            *value = mask & (1u64 << index) != 0;
        }

        let antecedent_value = antecedent.evaluate(&assignment)?;
        let consequent_value = consequent.evaluate(&assignment)?;

        if antecedent_value {
            antecedent_true_assignments += 1;
        }
        if consequent_value {
            consequent_true_assignments += 1;
        }

        match (antecedent_value, consequent_value) {
            (true, false) if antecedent_only_witness.is_none() => {
                antecedent_only_witness = Some(PseudoBooleanDifferenceWitness {
                    assignment_mask: mask,
                });
            }
            (false, true) if consequent_only_witness.is_none() => {
                consequent_only_witness = Some(PseudoBooleanDifferenceWitness {
                    assignment_mask: mask,
                });
            }
            _ => {}
        }
    }

    match (antecedent_only_witness, consequent_only_witness) {
        (None, None) => {
            debug_assert_eq!(antecedent_true_assignments, consequent_true_assignments);
            Ok(PseudoBooleanSetRelation::Equivalent {
                assignments,
                satisfying_assignments: antecedent_true_assignments,
            })
        }
        (None, Some(consequent_only_witness)) => {
            Ok(PseudoBooleanSetRelation::AntecedentMoreRestrictive {
                assignments,
                antecedent_true_assignments,
                consequent_true_assignments,
                consequent_only_witness,
            })
        }
        (Some(antecedent_only_witness), None) => {
            Ok(PseudoBooleanSetRelation::ConsequentMoreRestrictive {
                assignments,
                antecedent_true_assignments,
                consequent_true_assignments,
                antecedent_only_witness,
            })
        }
        (Some(antecedent_only_witness), Some(consequent_only_witness)) => {
            Ok(PseudoBooleanSetRelation::Incomparable {
                assignments,
                antecedent_true_assignments,
                consequent_true_assignments,
                antecedent_only_witness,
                consequent_only_witness,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PseudoBooleanRelation;

    #[test]
    fn exact_scaling_is_equivalent_with_exact_satisfying_count() {
        let original =
            PseudoBooleanConstraint::new(vec![2, 5, 7], 7, PseudoBooleanRelation::AtMost).unwrap();
        let scaled = original.scaled(9).unwrap();

        assert_eq!(
            pseudo_boolean_set_relation(&original, &scaled),
            Ok(PseudoBooleanSetRelation::Equivalent {
                assignments: 8,
                satisfying_assignments: 5,
            })
        );
    }

    #[test]
    fn stronger_cardinality_is_more_restrictive_with_canonical_witness() {
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_set_relation(&at_least_two, &at_least_one),
            Ok(PseudoBooleanSetRelation::AntecedentMoreRestrictive {
                assignments: 8,
                antecedent_true_assignments: 4,
                consequent_true_assignments: 7,
                consequent_only_witness: PseudoBooleanDifferenceWitness { assignment_mask: 1 },
            })
        );
    }

    #[test]
    fn reverse_order_reports_consequent_more_restrictive() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_set_relation(&at_least_one, &at_least_two),
            Ok(PseudoBooleanSetRelation::ConsequentMoreRestrictive {
                assignments: 8,
                antecedent_true_assignments: 7,
                consequent_true_assignments: 4,
                antecedent_only_witness: PseudoBooleanDifferenceWitness { assignment_mask: 1 },
            })
        );
    }

    #[test]
    fn independent_single_variable_predicates_are_incomparable() {
        let x0 =
            PseudoBooleanConstraint::new(vec![1, 0], 1, PseudoBooleanRelation::Exactly).unwrap();
        let x1 =
            PseudoBooleanConstraint::new(vec![0, 1], 1, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_set_relation(&x0, &x1),
            Ok(PseudoBooleanSetRelation::Incomparable {
                assignments: 4,
                antecedent_true_assignments: 2,
                consequent_true_assignments: 2,
                antecedent_only_witness: PseudoBooleanDifferenceWitness { assignment_mask: 1 },
                consequent_only_witness: PseudoBooleanDifferenceWitness { assignment_mask: 2 },
            })
        );
    }

    #[test]
    fn two_contradictions_are_equivalent_without_true_assignments() {
        let at_least_three =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::AtLeast).unwrap();
        let exactly_three =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_set_relation(&at_least_three, &exactly_three),
            Ok(PseudoBooleanSetRelation::Equivalent {
                assignments: 4,
                satisfying_assignments: 0,
            })
        );
    }

    #[test]
    fn mismatched_arity_and_insufficient_work_fail_closed() {
        let one =
            PseudoBooleanConstraint::cardinality(1, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let two =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        assert_eq!(
            pseudo_boolean_set_relation(&one, &two),
            Err(PseudoBooleanError::ArityMismatch { left: 1, right: 2 })
        );

        let large =
            PseudoBooleanConstraint::cardinality(63, 1, PseudoBooleanRelation::AtLeast).unwrap();
        assert_eq!(
            pseudo_boolean_set_relation_with_work_limit(&large, &large, 1024),
            Err(PseudoBooleanError::WorkLimitExceeded {
                required: 1u128 << 63,
                limit: 1024,
            })
        );
    }
}
