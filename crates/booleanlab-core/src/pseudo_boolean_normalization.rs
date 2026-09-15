//! Exact common-factor normalization for BL-BE4 pseudo-Boolean constraints.
//!
//! The transformation divides every weight and the threshold by their greatest
//! common divisor. It is deliberately narrower than algebraic PB simplification:
//! no rounding, variable elimination, threshold tightening or solver heuristic is
//! performed.

use crate::{PseudoBooleanConstraint, PseudoBooleanError};

/// Return the greatest common divisor shared by all weights and the threshold.
///
/// A fully zero constraint reports factor `1` so normalization is idempotent
/// and never introduces a zero divisor.
#[must_use]
pub fn pseudo_boolean_common_factor(constraint: &PseudoBooleanConstraint) -> u64 {
    let mut factor = constraint.threshold();
    for &weight in constraint.weights() {
        factor = gcd(factor, weight);
        if factor == 1 {
            return 1;
        }
    }
    factor.max(1)
}

/// Divide a pseudo-Boolean constraint by its exact shared integer factor.
///
/// The returned factor is the integer by which the primitive constraint must
/// be scaled to recover the input representation. Because every coefficient
/// and the threshold are divided by the same positive integer, the Boolean
/// truth table is unchanged.
///
/// # Errors
///
/// Propagates construction errors from [`PseudoBooleanConstraint::new`]. The
/// input is already validated, so an error would indicate an internal contract
/// regression rather than a lossy normalization.
pub fn primitive_pseudo_boolean_constraint(
    constraint: &PseudoBooleanConstraint,
) -> Result<(PseudoBooleanConstraint, u64), PseudoBooleanError> {
    let factor = pseudo_boolean_common_factor(constraint);
    if factor == 1 {
        return Ok((constraint.clone(), 1));
    }
    let weights = constraint
        .weights()
        .iter()
        .map(|weight| weight / factor)
        .collect();
    let threshold = constraint.threshold() / factor;
    let primitive = PseudoBooleanConstraint::new(weights, threshold, constraint.relation())?;
    Ok((primitive, factor))
}

const fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PseudoBooleanRelation, pseudo_boolean_equivalent};

    #[test]
    fn common_factor_includes_threshold_and_ignores_zero_coefficients() {
        let constraint = PseudoBooleanConstraint::new(
            vec![0, 12, 18, 30],
            24,
            PseudoBooleanRelation::AtLeast,
        )
        .unwrap();
        assert_eq!(pseudo_boolean_common_factor(&constraint), 6);
    }

    #[test]
    fn primitive_form_round_trips_by_exact_scaling() {
        let original = PseudoBooleanConstraint::new(
            vec![0, 12, 18, 30],
            24,
            PseudoBooleanRelation::Exactly,
        )
        .unwrap();
        let (primitive, factor) = primitive_pseudo_boolean_constraint(&original).unwrap();
        assert_eq!(factor, 6);
        assert_eq!(primitive.weights(), &[0, 2, 3, 5]);
        assert_eq!(primitive.threshold(), 4);
        assert_eq!(primitive.scaled(factor).unwrap(), original);
        assert!(pseudo_boolean_equivalent(&primitive, &original).unwrap());
    }

    #[test]
    fn primitive_form_preserves_all_relations_exhaustively() {
        for relation in [
            PseudoBooleanRelation::AtLeast,
            PseudoBooleanRelation::AtMost,
            PseudoBooleanRelation::Exactly,
        ] {
            let original =
                PseudoBooleanConstraint::new(vec![14, 28, 42], 56, relation).unwrap();
            let (primitive, factor) = primitive_pseudo_boolean_constraint(&original).unwrap();
            assert_eq!(factor, 14);
            assert!(pseudo_boolean_equivalent(&primitive, &original).unwrap());
        }
    }

    #[test]
    fn coprime_and_all_zero_constraints_are_stable() {
        let coprime =
            PseudoBooleanConstraint::new(vec![2, 3], 5, PseudoBooleanRelation::AtMost).unwrap();
        let (same, factor) = primitive_pseudo_boolean_constraint(&coprime).unwrap();
        assert_eq!(factor, 1);
        assert_eq!(same, coprime);

        let zero =
            PseudoBooleanConstraint::new(vec![0, 0], 0, PseudoBooleanRelation::Exactly).unwrap();
        let (same, factor) = primitive_pseudo_boolean_constraint(&zero).unwrap();
        assert_eq!(factor, 1);
        assert_eq!(same, zero);
    }
}
