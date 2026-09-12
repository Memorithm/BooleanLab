//! Exact `GF(2^k)`-induced Boolean-function generators for BL-13.

use std::fmt;

use scirust_modalg::gf2::Gf2Field;

use crate::{BooleanFunction, FunctionError};

/// Practical exhaustive bound for the finite-field inversion generator.
///
/// BL-13's general Boolean analysis can go wider, but materialising one truth
/// table per field-coordinate scales as `k * 2^k` bytes before analysis.
pub const MAX_GF2_INVERSION_BITS: u32 = 16;

/// Errors raised while generating Boolean component functions from a binary
/// extension field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gf2GeneratorError {
    WidthTooLarge { width: u32, maximum: u32 },
    NonInvertibleElement { element: u64 },
    Function(FunctionError),
}

/// Generates the coordinate Boolean functions of field inversion.
///
/// For a field `GF(2^k)`, this constructs `k` scalar functions
/// `f_j: F_2^k -> F_2`. The domain value `x` is mapped to `x^-1` for nonzero
/// `x`, while zero maps to zero. Function `j` is bit `j` of that exact field
/// element in `Gf2Field`'s polynomial basis.
///
/// # Errors
///
/// Returns [`Gf2GeneratorError::WidthTooLarge`] above the declared exhaustive
/// resource bound, [`Gf2GeneratorError::NonInvertibleElement`] when the supplied
/// reduction polynomial does not define a field for a nonzero element, and
/// [`Gf2GeneratorError::Function`] if the resulting truth table violates the
/// Boolean-function representation contract.
pub fn inversion_component_functions(
    field: Gf2Field,
) -> Result<Vec<BooleanFunction>, Gf2GeneratorError> {
    let degree = field.degree();
    if degree > MAX_GF2_INVERSION_BITS {
        return Err(Gf2GeneratorError::WidthTooLarge {
            width: degree,
            maximum: MAX_GF2_INVERSION_BITS,
        });
    }

    let order = 1usize << degree;
    let mut tables: Vec<Vec<u8>> = (0..degree)
        .map(|_| Vec::with_capacity(order))
        .collect();

    for element in 0..field.order() {
        let inverse = if element == 0 {
            0
        } else {
            field
                .inv(element)
                .ok_or(Gf2GeneratorError::NonInvertibleElement { element })?
        };
        for (bit, table) in tables.iter_mut().enumerate() {
            table.push(u8::from(((inverse >> bit) & 1) != 0));
        }
    }

    tables
        .into_iter()
        .map(|table| BooleanFunction::new(degree, table).map_err(Gf2GeneratorError::Function))
        .collect()
}

impl fmt::Display for Gf2GeneratorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidthTooLarge { width, maximum } => write!(
                formatter,
                "GF(2^k) inversion generation supports at most {maximum} bits, got {width}"
            ),
            Self::NonInvertibleElement { element } => write!(
                formatter,
                "nonzero field element {element:#x} has no inverse under the supplied modulus"
            ),
            Self::Function(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Gf2GeneratorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DedupIndex, DedupOutcome};

    #[test]
    fn primitive8_inversion_components_have_expected_exact_metrics() {
        let functions = inversion_component_functions(Gf2Field::primitive8()).unwrap();
        assert_eq!(functions.len(), 8);

        let mut index = DedupIndex::new();
        for function in functions {
            assert_eq!(function.input_bits(), 8);
            let metrics = function.exact_metrics();
            assert_eq!(metrics.algebraic_degree, 7);
            assert_eq!(metrics.nonlinearity, 112);
            assert!(metrics.balanced);
            assert!(!metrics.bent);
            assert_eq!(index.insert(function), DedupOutcome::New);
        }
        assert_eq!(index.unique_functions(), 8);
    }

    #[test]
    fn reducible_modulus_is_rejected_when_an_element_is_not_invertible() {
        let reducible = Gf2Field::new(4, 0b1_0101);
        assert!(matches!(
            inversion_component_functions(reducible),
            Err(Gf2GeneratorError::NonInvertibleElement { .. })
        ));
    }

    #[test]
    fn generator_enforces_resource_bound() {
        assert_eq!(
            inversion_component_functions(Gf2Field::gf2_16()).unwrap().len(),
            16
        );
        let too_wide = Gf2Field::new(17, (1_u64 << 17) | 0b11);
        assert_eq!(
            inversion_component_functions(too_wide),
            Err(Gf2GeneratorError::WidthTooLarge {
                width: 17,
                maximum: MAX_GF2_INVERSION_BITS
            })
        );
    }
}
