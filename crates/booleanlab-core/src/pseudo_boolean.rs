//! Exact bounded pseudo-Boolean constraints for BL-BE4.
//!
//! This module is a small-domain reference oracle for weighted/cardinality
//! predicates. It evaluates `sum_i w_i x_i` with exact integer arithmetic,
//! provides checked positive integer scaling, and can exhaustively compare two
//! constraints over the full declared Boolean domain under an explicit work
//! bound. It is not a solver-performance result or a production runtime gate.

use core::fmt;

/// Maximum arity accepted by the bounded exhaustive oracle.
pub const MAX_PSEUDO_BOOLEAN_TERMS: usize = 63;
/// Default maximum number of assignments visited by exhaustive comparison.
pub const DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS: u128 = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoBooleanRelation {
    AtLeast,
    AtMost,
    Exactly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoBooleanConstraint {
    weights: Vec<u64>,
    threshold: u64,
    relation: PseudoBooleanRelation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoBooleanError {
    TooManyTerms { terms: usize, max: usize },
    AssignmentLengthMismatch { expected: usize, actual: usize },
    ArityMismatch { left: usize, right: usize },
    ZeroScale,
    ScalingOverflow { term: Option<usize> },
    WorkAccountingOverflow,
    WorkLimitExceeded { required: u128, limit: u128 },
}

impl fmt::Display for PseudoBooleanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyTerms { terms, max } => {
                write!(
                    f,
                    "pseudo-Boolean constraint has {terms} terms; maximum is {max}"
                )
            }
            Self::AssignmentLengthMismatch { expected, actual } => write!(
                f,
                "pseudo-Boolean assignment length mismatch: expected {expected}, got {actual}"
            ),
            Self::ArityMismatch { left, right } => write!(
                f,
                "pseudo-Boolean arity mismatch: left has {left} terms, right has {right}"
            ),
            Self::ZeroScale => {
                f.write_str("pseudo-Boolean exact scaling requires a positive factor")
            }
            Self::ScalingOverflow { term: Some(term) } => {
                write!(
                    f,
                    "pseudo-Boolean weight {term} overflowed during exact scaling"
                )
            }
            Self::ScalingOverflow { term: None } => {
                f.write_str("pseudo-Boolean threshold overflowed during exact scaling")
            }
            Self::WorkAccountingOverflow => {
                f.write_str("pseudo-Boolean exhaustive work accounting overflowed")
            }
            Self::WorkLimitExceeded { required, limit } => write!(
                f,
                "pseudo-Boolean exhaustive comparison requires {required} assignments, limit is {limit}"
            ),
        }
    }
}

impl std::error::Error for PseudoBooleanError {}

impl PseudoBooleanConstraint {
    /// Build a weighted pseudo-Boolean constraint.
    ///
    /// # Errors
    ///
    /// Returns [`PseudoBooleanError::TooManyTerms`] when the declared arity
    /// exceeds the bounded exact-oracle limit.
    pub fn new(
        weights: Vec<u64>,
        threshold: u64,
        relation: PseudoBooleanRelation,
    ) -> Result<Self, PseudoBooleanError> {
        if weights.len() > MAX_PSEUDO_BOOLEAN_TERMS {
            return Err(PseudoBooleanError::TooManyTerms {
                terms: weights.len(),
                max: MAX_PSEUDO_BOOLEAN_TERMS,
            });
        }
        Ok(Self {
            weights,
            threshold,
            relation,
        })
    }

    /// Construct an unweighted cardinality constraint with unit weights.
    ///
    /// Thresholds larger than `arity` remain valid exact predicates: `AtMost`
    /// is then a tautology while `AtLeast` and `Exactly` are contradictions.
    ///
    /// # Errors
    ///
    /// Returns an error when `arity` exceeds the bounded exact-oracle limit.
    pub fn cardinality(
        arity: usize,
        threshold: u64,
        relation: PseudoBooleanRelation,
    ) -> Result<Self, PseudoBooleanError> {
        if arity > MAX_PSEUDO_BOOLEAN_TERMS {
            return Err(PseudoBooleanError::TooManyTerms {
                terms: arity,
                max: MAX_PSEUDO_BOOLEAN_TERMS,
            });
        }
        Self::new(vec![1; arity], threshold, relation)
    }

    #[must_use]
    pub fn weights(&self) -> &[u64] {
        &self.weights
    }

    #[must_use]
    pub const fn threshold(&self) -> u64 {
        self.threshold
    }

    #[must_use]
    pub const fn relation(&self) -> PseudoBooleanRelation {
        self.relation
    }

    #[must_use]
    pub fn arity(&self) -> usize {
        self.weights.len()
    }

    /// Evaluate one assignment with a `u128` accumulator so selected `u64`
    /// weights cannot wrap the mathematical sum.
    ///
    /// # Errors
    ///
    /// Returns an error when the assignment arity differs from the constraint
    /// or if exact work accounting cannot be represented.
    pub fn evaluate(&self, assignment: &[bool]) -> Result<bool, PseudoBooleanError> {
        if assignment.len() != self.weights.len() {
            return Err(PseudoBooleanError::AssignmentLengthMismatch {
                expected: self.weights.len(),
                actual: assignment.len(),
            });
        }
        let mut sum = 0u128;
        for (&weight, &selected) in self.weights.iter().zip(assignment) {
            if selected {
                sum = sum
                    .checked_add(u128::from(weight))
                    .ok_or(PseudoBooleanError::WorkAccountingOverflow)?;
            }
        }
        Ok(self.accepts_sum(sum))
    }

    /// Multiply every coefficient and threshold by the same positive integer.
    ///
    /// For representable coefficients this preserves the Boolean truth table
    /// exactly. Overflow is an explicit non-result rather than saturation.
    ///
    /// # Errors
    ///
    /// Returns an error for a zero scale or when any scaled coefficient or the
    /// threshold is not representable as `u64`.
    pub fn scaled(&self, factor: u64) -> Result<Self, PseudoBooleanError> {
        if factor == 0 {
            return Err(PseudoBooleanError::ZeroScale);
        }
        let mut weights = Vec::with_capacity(self.weights.len());
        for (index, &weight) in self.weights.iter().enumerate() {
            weights.push(
                weight
                    .checked_mul(factor)
                    .ok_or(PseudoBooleanError::ScalingOverflow { term: Some(index) })?,
            );
        }
        let threshold = self
            .threshold
            .checked_mul(factor)
            .ok_or(PseudoBooleanError::ScalingOverflow { term: None })?;
        Self::new(weights, threshold, self.relation)
    }

    fn accepts_sum(&self, sum: u128) -> bool {
        let threshold = u128::from(self.threshold);
        match self.relation {
            PseudoBooleanRelation::AtLeast => sum >= threshold,
            PseudoBooleanRelation::AtMost => sum <= threshold,
            PseudoBooleanRelation::Exactly => sum == threshold,
        }
    }

    fn evaluate_mask(&self, mask: u64) -> bool {
        let mut sum = 0u128;
        for (index, &weight) in self.weights.iter().enumerate() {
            if mask & (1u64 << index) != 0 {
                sum += u128::from(weight);
            }
        }
        self.accepts_sum(sum)
    }
}

/// Exhaustively compare two constraints over all assignments of their common
/// arity using the default assignment budget.
///
/// # Errors
///
/// Returns an error for mismatched arity, arithmetic/work-accounting overflow,
/// or when the default work bound is insufficient for the exhaustive domain.
pub fn pseudo_boolean_equivalent(
    left: &PseudoBooleanConstraint,
    right: &PseudoBooleanConstraint,
) -> Result<bool, PseudoBooleanError> {
    pseudo_boolean_equivalent_with_work_limit(left, right, DEFAULT_PSEUDO_BOOLEAN_MAX_ASSIGNMENTS)
}

/// Exhaustively compare two constraints under an explicit assignment budget.
///
/// The complete work bound is checked before any assignment is evaluated.
///
/// # Errors
///
/// Returns an error for mismatched arity, arithmetic/work-accounting overflow,
/// or when the requested exhaustive domain exceeds `max_assignments`.
pub fn pseudo_boolean_equivalent_with_work_limit(
    left: &PseudoBooleanConstraint,
    right: &PseudoBooleanConstraint,
    max_assignments: u128,
) -> Result<bool, PseudoBooleanError> {
    if left.arity() != right.arity() {
        return Err(PseudoBooleanError::ArityMismatch {
            left: left.arity(),
            right: right.arity(),
        });
    }
    let shift =
        u32::try_from(left.arity()).map_err(|_| PseudoBooleanError::WorkAccountingOverflow)?;
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
    for mask in 0..assignments_u64 {
        if left.evaluate_mask(mask) != right.evaluate_mask(mask) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_and_cardinality_constraints_are_exact() {
        let weighted =
            PseudoBooleanConstraint::new(vec![2, 3, 5], 5, PseudoBooleanRelation::AtLeast).unwrap();
        assert!(weighted.evaluate(&[false, false, true]).unwrap());
        assert!(weighted.evaluate(&[true, true, false]).unwrap());
        assert!(!weighted.evaluate(&[true, false, false]).unwrap());

        let cardinality =
            PseudoBooleanConstraint::cardinality(3, 2, PseudoBooleanRelation::Exactly).unwrap();
        assert!(cardinality.evaluate(&[true, true, false]).unwrap());
        assert!(!cardinality.evaluate(&[true, true, true]).unwrap());
    }

    #[test]
    fn cardinality_threshold_above_arity_keeps_exact_boundary_semantics() {
        let at_most =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::AtMost).unwrap();
        let at_least =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::AtLeast).unwrap();
        let exactly =
            PseudoBooleanConstraint::cardinality(2, 3, PseudoBooleanRelation::Exactly).unwrap();
        assert!(at_most.evaluate(&[true, true]).unwrap());
        assert!(!at_least.evaluate(&[true, true]).unwrap());
        assert!(!exactly.evaluate(&[true, true]).unwrap());
    }

    #[test]
    fn selected_weight_sum_does_not_wrap_u64() {
        let constraint = PseudoBooleanConstraint::new(
            vec![u64::MAX, u64::MAX],
            u64::MAX,
            PseudoBooleanRelation::AtLeast,
        )
        .unwrap();
        assert!(constraint.evaluate(&[true, true]).unwrap());
    }

    #[test]
    fn positive_integer_scaling_preserves_truth_table_exhaustively() {
        let original =
            PseudoBooleanConstraint::new(vec![2, 5, 7, 11], 13, PseudoBooleanRelation::AtMost)
                .unwrap();
        let scaled = original.scaled(7).unwrap();
        assert!(pseudo_boolean_equivalent(&original, &scaled).unwrap());
    }

    #[test]
    fn scaling_overflow_and_zero_scale_fail_closed() {
        let constraint =
            PseudoBooleanConstraint::new(vec![u64::MAX], 1, PseudoBooleanRelation::AtLeast)
                .unwrap();
        assert_eq!(constraint.scaled(0), Err(PseudoBooleanError::ZeroScale));
        assert_eq!(
            constraint.scaled(2),
            Err(PseudoBooleanError::ScalingOverflow { term: Some(0) })
        );

        let threshold_overflow =
            PseudoBooleanConstraint::new(vec![1], u64::MAX, PseudoBooleanRelation::AtMost).unwrap();
        assert_eq!(
            threshold_overflow.scaled(2),
            Err(PseudoBooleanError::ScalingOverflow { term: None })
        );
    }

    #[test]
    fn exhaustive_comparison_detects_difference_and_bounds_work() {
        let at_least_one =
            PseudoBooleanConstraint::cardinality(10, 1, PseudoBooleanRelation::AtLeast).unwrap();
        let at_least_two =
            PseudoBooleanConstraint::cardinality(10, 2, PseudoBooleanRelation::AtLeast).unwrap();
        assert!(!pseudo_boolean_equivalent(&at_least_one, &at_least_two).unwrap());
        assert_eq!(
            pseudo_boolean_equivalent_with_work_limit(&at_least_one, &at_least_two, 100),
            Err(PseudoBooleanError::WorkLimitExceeded {
                required: 1024,
                limit: 100,
            })
        );
    }

    #[test]
    fn invalid_shape_inputs_are_rejected() {
        let constraint =
            PseudoBooleanConstraint::new(vec![1, 2], 1, PseudoBooleanRelation::AtLeast).unwrap();
        assert_eq!(
            constraint.evaluate(&[true]),
            Err(PseudoBooleanError::AssignmentLengthMismatch {
                expected: 2,
                actual: 1,
            })
        );
        assert_eq!(
            PseudoBooleanConstraint::cardinality(
                MAX_PSEUDO_BOOLEAN_TERMS + 1,
                0,
                PseudoBooleanRelation::AtLeast,
            ),
            Err(PseudoBooleanError::TooManyTerms {
                terms: MAX_PSEUDO_BOOLEAN_TERMS + 1,
                max: MAX_PSEUDO_BOOLEAN_TERMS,
            })
        );
    }
}
