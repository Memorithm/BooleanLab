//! Exact bounded conjunction entailment for BL-BE4 pseudo-Boolean constraints.
//!
//! This module checks whether a conjunction of weighted/cardinality predicates
//! implies another predicate over the complete shared Boolean domain. It is a
//! small-domain reference oracle for redundancy analysis, not a SAT/PB solver,
//! production runtime gate, performance claim, or automatic rule-deletion
//! authority.

use core::fmt;

use crate::{DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS, PseudoBooleanConstraint, PseudoBooleanError};

/// Default conservative upper bound on pseudo-Boolean predicate evaluations.
///
/// One complete assignment may evaluate every antecedent plus the consequent,
/// so this is deliberately a work-unit bound rather than merely a domain-size
/// bound.
pub const DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS: u128 =
    DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS;

/// First exact assignment showing that a conjunction does not entail its target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PseudoBooleanConjunctionWitness {
    /// Canonical Boolean assignment mask. Bit `i` is variable `i`.
    pub assignment_mask: u64,
}

/// Exact exhaustive result for conjunction implication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoBooleanConjunctionImplication {
    /// Every assignment satisfying all antecedents also satisfies the consequent.
    Entails {
        /// Complete Boolean domain size checked.
        assignments: u128,
        /// Assignments satisfying every antecedent.
        antecedent_true_assignments: u128,
    },
    /// A canonical assignment satisfies all antecedents but violates the target.
    DoesNotEntail {
        /// Complete Boolean domain size declared by the shared arity.
        assignments: u128,
        /// First violating assignment in ascending mask order.
        witness: PseudoBooleanConjunctionWitness,
    },
}

/// Errors from bounded conjunction implication.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PseudoBooleanConjunctionError {
    /// One antecedent has a different variable domain from the consequent.
    ArityMismatch {
        antecedent_index: usize,
        antecedent_arity: usize,
        consequent_arity: usize,
    },
    /// Checked work accounting overflowed.
    WorkAccountingOverflow,
    /// The conservative predicate-evaluation upper bound exceeds the limit.
    WorkLimitExceeded {
        required_evaluations: u128,
        limit: u128,
    },
    /// A constraint evaluation failed its own shape/integer contract.
    Constraint(PseudoBooleanError),
}

impl fmt::Display for PseudoBooleanConjunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ArityMismatch {
                antecedent_index,
                antecedent_arity,
                consequent_arity,
            } => write!(
                formatter,
                "pseudo-Boolean antecedent {antecedent_index} has arity {antecedent_arity}; consequent arity is {consequent_arity}"
            ),
            Self::WorkAccountingOverflow => formatter
                .write_str("pseudo-Boolean conjunction work accounting overflowed"),
            Self::WorkLimitExceeded {
                required_evaluations,
                limit,
            } => write!(
                formatter,
                "pseudo-Boolean conjunction requires at most {required_evaluations} predicate evaluations; limit is {limit}"
            ),
            Self::Constraint(error) => write!(formatter, "pseudo-Boolean constraint error: {error}"),
        }
    }
}

impl std::error::Error for PseudoBooleanConjunctionError {}

impl From<PseudoBooleanError> for PseudoBooleanConjunctionError {
    fn from(error: PseudoBooleanError) -> Self {
        Self::Constraint(error)
    }
}

/// Exhaustively check whether the conjunction of `antecedents` implies
/// `consequent` under the default conservative predicate-evaluation budget.
///
/// An empty antecedent slice is the logical constant `true`; the oracle then
/// checks whether `consequent` is a tautology over its complete declared domain.
///
/// # Errors
///
/// Returns [`PseudoBooleanConjunctionError::ArityMismatch`] when an antecedent
/// uses another domain, a work-accounting error on overflow, a work-limit error
/// before evaluation when the conservative upper bound is too large, or a
/// wrapped constraint-evaluation error.
pub fn pseudo_boolean_conjunction_implies(
    antecedents: &[PseudoBooleanConstraint],
    consequent: &PseudoBooleanConstraint,
) -> Result<PseudoBooleanConjunctionImplication, PseudoBooleanConjunctionError> {
    pseudo_boolean_conjunction_implies_with_work_limit(
        antecedents,
        consequent,
        DEFAULT_PSEUDO_BOOLEAN_CONJUNCTION_MAX_EVALUATIONS,
    )
}

/// Exhaustively check conjunction implication under an explicit conservative
/// predicate-evaluation budget.
///
/// The required upper bound is `2^arity * (antecedent_count + 1)`, checked before
/// evaluating any constraint. Short-circuiting may perform less actual work, but
/// never permits a larger declared problem to bypass the budget. Resource-limit
/// exhaustion is therefore a non-result, never entailment evidence.
///
/// # Errors
///
/// Returns the same errors as [`pseudo_boolean_conjunction_implies`].
pub fn pseudo_boolean_conjunction_implies_with_work_limit(
    antecedents: &[PseudoBooleanConstraint],
    consequent: &PseudoBooleanConstraint,
    max_evaluations: u128,
) -> Result<PseudoBooleanConjunctionImplication, PseudoBooleanConjunctionError> {
    for (antecedent_index, antecedent) in antecedents.iter().enumerate() {
        if antecedent.arity() != consequent.arity() {
            return Err(PseudoBooleanConjunctionError::ArityMismatch {
                antecedent_index,
                antecedent_arity: antecedent.arity(),
                consequent_arity: consequent.arity(),
            });
        }
    }

    let shift = u32::try_from(consequent.arity())
        .map_err(|_| PseudoBooleanConjunctionError::WorkAccountingOverflow)?;
    let assignments = 1u128
        .checked_shl(shift)
        .ok_or(PseudoBooleanConjunctionError::WorkAccountingOverflow)?;
    let predicates = u128::try_from(antecedents.len())
        .map_err(|_| PseudoBooleanConjunctionError::WorkAccountingOverflow)?
        .checked_add(1)
        .ok_or(PseudoBooleanConjunctionError::WorkAccountingOverflow)?;
    let required_evaluations = assignments
        .checked_mul(predicates)
        .ok_or(PseudoBooleanConjunctionError::WorkAccountingOverflow)?;
    if required_evaluations > max_evaluations {
        return Err(PseudoBooleanConjunctionError::WorkLimitExceeded {
            required_evaluations,
            limit: max_evaluations,
        });
    }

    let assignments_u64 = u64::try_from(assignments)
        .map_err(|_| PseudoBooleanConjunctionError::WorkAccountingOverflow)?;
    let mut assignment = vec![false; consequent.arity()];
    let mut antecedent_true_assignments = 0u128;

    for mask in 0..assignments_u64 {
        for (index, value) in assignment.iter_mut().enumerate() {
            *value = mask & (1u64 << index) != 0;
        }

        let mut conjunction_true = true;
        for antecedent in antecedents {
            if !antecedent.evaluate(&assignment)? {
                conjunction_true = false;
                break;
            }
        }
        if !conjunction_true {
            continue;
        }
        antecedent_true_assignments = antecedent_true_assignments
            .checked_add(1)
            .ok_or(PseudoBooleanConjunctionError::WorkAccountingOverflow)?;

        if !consequent.evaluate(&assignment)? {
            return Ok(PseudoBooleanConjunctionImplication::DoesNotEntail {
                assignments,
                witness: PseudoBooleanConjunctionWitness {
                    assignment_mask: mask,
                },
            });
        }
    }

    Ok(PseudoBooleanConjunctionImplication::Entails {
        assignments,
        antecedent_true_assignments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PseudoBooleanRelation;

    #[test]
    fn conjunction_can_entail_when_neither_antecedent_does_alone() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_most_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtMost).unwrap();
        let exactly_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_conjunction_implies(&[at_least_one, at_most_one], &exactly_one),
            Ok(PseudoBooleanConjunctionImplication::Entails {
                assignments: 4,
                antecedent_true_assignments: 2,
            })
        );
    }

    #[test]
    fn empty_conjunction_returns_first_tautology_counterexample() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_conjunction_implies(&[], &at_least_one),
            Ok(PseudoBooleanConjunctionImplication::DoesNotEntail {
                assignments: 4,
                witness: PseudoBooleanConjunctionWitness { assignment_mask: 0 },
            })
        );
    }

    #[test]
    fn contradictory_antecedents_entail_vacuously_and_report_zero_support() {
        let at_least_two =
            PseudoBooleanConstraint::cardinality(2, 2, PseudoBooleanRelation::AtLeast).unwrap();
        let at_most_one =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtMost).unwrap();
        let impossible =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::Exactly).unwrap();

        assert_eq!(
            pseudo_boolean_conjunction_implies(&[at_least_two, at_most_one], &impossible),
            Ok(PseudoBooleanConjunctionImplication::Entails {
                assignments: 4,
                antecedent_true_assignments: 0,
            })
        );
    }

    #[test]
    fn arity_mismatch_fails_closed_with_antecedent_index() {
        let two =
            PseudoBooleanConstraint::cardinality(2, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let three =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();

        assert_eq!(
            pseudo_boolean_conjunction_implies(&[two], &three),
            Err(PseudoBooleanConjunctionError::ArityMismatch {
                antecedent_index: 0,
                antecedent_arity: 2,
                consequent_arity: 3,
            })
        );
    }

    #[test]
    fn conservative_evaluation_budget_fails_before_search() {
        let one =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let two =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::AtLeast).unwrap();
        let consequent =
            PseudoBooleanConstraint::cardinality(3, 1, PseudoBooleanRelation::AtMost).unwrap();

        assert_eq!(
            pseudo_boolean_conjunction_implies_with_work_limit(&[one, two], &consequent, 23),
            Err(PseudoBooleanConjunctionError::WorkLimitExceeded {
                required_evaluations: 24,
                limit: 23,
            })
        );
    }
}
