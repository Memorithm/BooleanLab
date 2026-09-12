use crate::BitState;

/// Extracts explicit Boolean predicates from a value in mathematical domain `X`.
///
/// The bridge must document the semantics of every produced bit. BooleanLab
/// treats these predicates as part of the experimental contract rather than as
/// an opaque embedding.
pub trait PredicateBridge<X> {
    type Error;

    fn predicates(&self, value: &X) -> Result<BitState, Self::Error>;
}

/// Applies a Boolean control state to an operation in mathematical domain `X`.
///
/// Implementations may mask components, select an operator, route data or
/// enforce a constraint, but must preserve the declared semantics of `X`.
pub trait HybridOperator<X> {
    type Error;

    fn apply(&self, control: &BitState, value: &X) -> Result<X, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StateError;

    struct SignPredicate;

    impl PredicateBridge<f64> for SignPredicate {
        type Error = StateError;

        fn predicates(&self, value: &f64) -> Result<BitState, Self::Error> {
            BitState::from_bools(&[*value >= 0.0])
        }
    }

    struct KeepOrZero;

    impl HybridOperator<f64> for KeepOrZero {
        type Error = StateError;

        fn apply(&self, control: &BitState, value: &f64) -> Result<f64, Self::Error> {
            Ok(if control.get(0)? { *value } else { 0.0 })
        }
    }

    #[test]
    fn bridges_boolean_control_and_numeric_domain() {
        let bridge = SignPredicate;
        let operator = KeepOrZero;
        let control = bridge.predicates(&3.0).unwrap();
        assert_eq!(operator.apply(&control, &3.0), Ok(3.0));

        let control = bridge.predicates(&-3.0).unwrap();
        assert_eq!(operator.apply(&control, &-3.0), Ok(0.0));
    }
}
