//! Sedenion-induced Boolean-function controls for BL-13.
//!
//! This module is available only with the `sedenion-experiments` feature and
//! therefore on the nightly toolchain required by `SciRust` `portable-simd`.

use std::fmt;

use scirust_simd::hypercomplex::SedenionSimd;

use crate::{BooleanFunction, FunctionError};

/// Input width of the first BL-13 sedenion control.
pub const CONTROL_INPUT_BITS: u32 = 8;
/// Number of scalar coordinate predicates produced by a sedenion.
pub const SEDENION_COORDINATES: usize = 16;

/// Fail-closed errors for the integer-lattice sedenion control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SedenionGeneratorError {
    NonFiniteOutput {
        input: u64,
        coordinate: usize,
        value_bits: u32,
    },
    NonIntegralOutput {
        input: u64,
        coordinate: usize,
        value_bits: u32,
    },
    Function(FunctionError),
}

const FIXED_MIXER: [f32; SEDENION_COORDINATES] = [
    1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0, -1.0,
];

#[inline]
fn bipolar(input: u64, bit: u32) -> f32 {
    if ((input >> bit) & 1) == 1 { 1.0 } else { -1.0 }
}

fn left_operand(input: u64) -> SedenionSimd {
    let mut coefficients = [0.0; SEDENION_COORDINATES];
    coefficients[0] = 1.0;
    for bit in 0..4_u32 {
        coefficients[1 + bit as usize] = bipolar(input, bit);
        coefficients[9 + bit as usize] = bipolar(input, bit + 4);
    }
    SedenionSimd::from_array(coefficients)
}

fn right_operand(input: u64) -> SedenionSimd {
    let mut coefficients = [0.0; SEDENION_COORDINATES];
    coefficients[0] = 1.0;
    for bit in 0..4_u32 {
        coefficients[1 + bit as usize] = bipolar(input, bit + 4);
        coefficients[9 + bit as usize] = bipolar(input, bit);
    }
    SedenionSimd::from_array(coefficients)
}

fn control_state(input: u64) -> Result<[f32; SEDENION_COORDINATES], SedenionGeneratorError> {
    let mixer = SedenionSimd::from_array(FIXED_MIXER);
    let state = (left_operand(input) * right_operand(input)) * mixer;
    let coefficients = state.to_array();

    for (coordinate, value) in coefficients.iter().copied().enumerate() {
        if !value.is_finite() {
            return Err(SedenionGeneratorError::NonFiniteOutput {
                input,
                coordinate,
                value_bits: value.to_bits(),
            });
        }
        if value.to_bits() != value.round().to_bits() {
            return Err(SedenionGeneratorError::NonIntegralOutput {
                input,
                coordinate,
                value_bits: value.to_bits(),
            });
        }
    }

    Ok(coefficients)
}

/// Generates sixteen scalar Boolean predicates from a bounded sedenion control.
///
/// Eight Boolean input bits are embedded as bipolar coefficients (`-1` or `+1`)
/// in two `SciRust` sedenions. The control computes `(left * right) * mixer`, where
/// the fixed mixer also has integer coefficients. Each output coordinate is
/// projected with the declared predicate `coordinate > 0`.
///
/// The generator verifies that every observed coefficient remains a finite exact
/// integer in `f32`. This is a control discipline for the chosen bounded inputs;
/// it is not a general claim that floating-point sedenion arithmetic is exact.
///
/// # Errors
///
/// Returns [`SedenionGeneratorError::NonFiniteOutput`] or
/// [`SedenionGeneratorError::NonIntegralOutput`] if the bounded integer-lattice
/// contract is violated, and [`SedenionGeneratorError::Function`] if a generated
/// truth table violates the Boolean-function representation contract.
pub fn control_component_functions() -> Result<Vec<BooleanFunction>, SedenionGeneratorError> {
    let row_count = 1usize << CONTROL_INPUT_BITS;
    let mut tables: Vec<Vec<u8>> = (0..SEDENION_COORDINATES)
        .map(|_| Vec::with_capacity(row_count))
        .collect();

    for input in 0..row_count as u64 {
        let coefficients = control_state(input)?;
        for (coordinate, table) in tables.iter_mut().enumerate() {
            table.push(u8::from(coefficients[coordinate] > 0.0));
        }
    }

    tables
        .into_iter()
        .map(|table| {
            BooleanFunction::new(CONTROL_INPUT_BITS, table)
                .map_err(SedenionGeneratorError::Function)
        })
        .collect()
}

impl fmt::Display for SedenionGeneratorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteOutput {
                input,
                coordinate,
                value_bits,
            } => write!(
                formatter,
                "sedenion control produced non-finite output at input {input:#x}, coordinate {coordinate}, bits {value_bits:#010x}"
            ),
            Self::NonIntegralOutput {
                input,
                coordinate,
                value_bits,
            } => write!(
                formatter,
                "sedenion control left the exact integer lattice at input {input:#x}, coordinate {coordinate}, bits {value_bits:#010x}"
            ),
            Self::Function(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for SedenionGeneratorError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DedupIndex;

    #[test]
    fn generates_sixteen_bounded_predicate_functions() {
        let functions = control_component_functions().unwrap();
        assert_eq!(functions.len(), SEDENION_COORDINATES);
        assert!(
            functions
                .iter()
                .all(|function| function.input_bits() == CONTROL_INPUT_BITS)
        );
    }

    #[test]
    fn generation_is_deterministic() {
        assert_eq!(
            control_component_functions().unwrap(),
            control_component_functions().unwrap()
        );
    }

    #[test]
    fn control_contains_nonconstant_predicates() {
        let functions = control_component_functions().unwrap();
        let nonconstant = functions
            .iter()
            .filter(|function| function.exact_metrics().algebraic_degree > 0)
            .count();
        assert!(nonconstant > 0);

        let mut index = DedupIndex::new();
        for function in functions {
            let _ = index.insert(function);
        }
        assert!(index.unique_functions() > 1);
    }
}
