//! Exact bounded algebraic-degree profile for vectorial Boolean maps.
//!
//! For a declared map `F: F_2^n -> F_2^m`, this module enumerates every
//! non-zero output mask, forms the scalar component `<mask, F(x)>`, computes
//! its ANF exactly with a Boolean Möbius transform, and records the degree
//! distribution. It is a small-domain reference oracle, not an EA/CCZ
//! classifier, novelty result, cryptographic certification, or performance
//! benchmark.

use core::fmt;

pub const MAX_VECTORIAL_DEGREE_INPUT_BITS: u8 = 16;
pub const MAX_VECTORIAL_DEGREE_OUTPUT_BITS: u8 = 16;
pub const DEFAULT_VECTORIAL_DEGREE_MAX_WORK: u128 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorialDegreeWitness {
    pub output_mask: u16,
    pub degree: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorialDegreeProfile {
    pub input_bits: u8,
    pub output_bits: u8,
    pub rows: usize,
    pub component_count: u64,
    /// Number of non-zero output-mask components at each algebraic degree.
    /// Index zero includes identically-zero scalar components by convention.
    pub degree_histogram: Vec<u64>,
    pub max_component_degree: u8,
    pub max_degree_witness: VectorialDegreeWitness,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorialDegreeError {
    InputBitsOutOfRange { input_bits: u8 },
    OutputBitsOutOfRange { output_bits: u8 },
    TableLengthMismatch { expected: usize, actual: usize },
    OutputOutOfRange {
        index: usize,
        value: u16,
        output_bits: u8,
    },
    ArithmeticOverflow,
    WorkLimitExceeded { required: u128, limit: u128 },
    AllocationFailed,
}

impl fmt::Display for VectorialDegreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputBitsOutOfRange { input_bits } => write!(
                f,
                "vectorial degree requires 1..={MAX_VECTORIAL_DEGREE_INPUT_BITS} input bits, got {input_bits}"
            ),
            Self::OutputBitsOutOfRange { output_bits } => write!(
                f,
                "vectorial degree requires 1..={MAX_VECTORIAL_DEGREE_OUTPUT_BITS} output bits, got {output_bits}"
            ),
            Self::TableLengthMismatch { expected, actual } => write!(
                f,
                "vectorial truth table length mismatch: expected {expected}, got {actual}"
            ),
            Self::OutputOutOfRange {
                index,
                value,
                output_bits,
            } => write!(
                f,
                "vectorial truth table output {value} at index {index} exceeds declared {output_bits}-bit range"
            ),
            Self::ArithmeticOverflow => f.write_str("vectorial degree work accounting overflowed"),
            Self::WorkLimitExceeded { required, limit } => write!(
                f,
                "vectorial degree exact work {required} exceeds declared limit {limit}"
            ),
            Self::AllocationFailed => f.write_str("vectorial degree scratch allocation failed"),
        }
    }
}

impl std::error::Error for VectorialDegreeError {}

fn checked_pow2(bits: u8) -> Result<usize, VectorialDegreeError> {
    1usize
        .checked_shl(u32::from(bits))
        .ok_or(VectorialDegreeError::ArithmeticOverflow)
}

fn validate_table(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<(usize, usize), VectorialDegreeError> {
    if !(1..=MAX_VECTORIAL_DEGREE_INPUT_BITS).contains(&input_bits) {
        return Err(VectorialDegreeError::InputBitsOutOfRange { input_bits });
    }
    if !(1..=MAX_VECTORIAL_DEGREE_OUTPUT_BITS).contains(&output_bits) {
        return Err(VectorialDegreeError::OutputBitsOutOfRange { output_bits });
    }

    let rows = checked_pow2(input_bits)?;
    if table.len() != rows {
        return Err(VectorialDegreeError::TableLengthMismatch {
            expected: rows,
            actual: table.len(),
        });
    }
    let output_values = checked_pow2(output_bits)?;
    let output_limit =
        u32::try_from(output_values).map_err(|_| VectorialDegreeError::ArithmeticOverflow)?;
    for (index, &value) in table.iter().enumerate() {
        if u32::from(value) >= output_limit {
            return Err(VectorialDegreeError::OutputOutOfRange {
                index,
                value,
                output_bits,
            });
        }
    }

    let rows_u128 =
        u128::try_from(rows).map_err(|_| VectorialDegreeError::ArithmeticOverflow)?;
    let components = u128::try_from(output_values - 1)
        .map_err(|_| VectorialDegreeError::ArithmeticOverflow)?;
    let mobius_updates = rows_u128
        .checked_mul(u128::from(input_bits))
        .and_then(|value| value.checked_div(2))
        .ok_or(VectorialDegreeError::ArithmeticOverflow)?;
    let per_component = rows_u128
        .checked_add(mobius_updates)
        .and_then(|value| value.checked_add(rows_u128))
        .ok_or(VectorialDegreeError::ArithmeticOverflow)?;
    let required = components
        .checked_mul(per_component)
        .ok_or(VectorialDegreeError::ArithmeticOverflow)?;
    if required > max_work {
        return Err(VectorialDegreeError::WorkLimitExceeded {
            required,
            limit: max_work,
        });
    }
    Ok((rows, output_values))
}

fn component_degree(
    table: &[u16],
    input_bits: u8,
    output_mask: u16,
    scratch: &mut [u8],
) -> u8 {
    for (x, slot) in scratch.iter_mut().enumerate() {
        *slot = u8::from(((table[x] & output_mask).count_ones() & 1) != 0);
    }

    for bit in 0..input_bits {
        let selector = 1usize << bit;
        for mask in 0..scratch.len() {
            if mask & selector != 0 {
                scratch[mask] ^= scratch[mask ^ selector];
            }
        }
    }

    scratch
        .iter()
        .enumerate()
        .filter_map(|(monomial, &coefficient)| {
            (coefficient != 0).then_some(monomial.count_ones() as u8)
        })
        .max()
        .unwrap_or(0)
}

/// Computes the exact algebraic-degree distribution over all non-zero output
/// components of a vectorial Boolean truth table.
///
/// The all-zero scalar component is assigned algebraic degree zero by explicit
/// convention. `max_degree_witness` is the first output mask attaining the
/// maximum degree in ascending mask order.
///
/// # Errors
///
/// Fails closed on unsupported dimensions, malformed table length, outputs
/// outside the declared width, arithmetic/allocation failure, or when the
/// conservative exact-work bound exceeds `max_work` before scratch allocation.
pub fn vectorial_degree_profile_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialDegreeProfile, VectorialDegreeError> {
    let (rows, output_values) = validate_table(table, input_bits, output_bits, max_work)?;

    let mut scratch = Vec::new();
    scratch
        .try_reserve_exact(rows)
        .map_err(|_| VectorialDegreeError::AllocationFailed)?;
    scratch.resize(rows, 0u8);

    let mut degree_histogram = Vec::new();
    degree_histogram
        .try_reserve_exact(usize::from(input_bits) + 1)
        .map_err(|_| VectorialDegreeError::AllocationFailed)?;
    degree_histogram.resize(usize::from(input_bits) + 1, 0u64);

    let mut max_component_degree = 0u8;
    let mut max_degree_witness = VectorialDegreeWitness {
        output_mask: 1,
        degree: 0,
    };
    for output_mask in 1..output_values {
        let output_mask =
            u16::try_from(output_mask).map_err(|_| VectorialDegreeError::ArithmeticOverflow)?;
        let degree = component_degree(table, input_bits, output_mask, &mut scratch);
        let slot = &mut degree_histogram[usize::from(degree)];
        *slot = slot
            .checked_add(1)
            .ok_or(VectorialDegreeError::ArithmeticOverflow)?;
        if degree > max_component_degree {
            max_component_degree = degree;
            max_degree_witness = VectorialDegreeWitness {
                output_mask,
                degree,
            };
        }
    }

    Ok(VectorialDegreeProfile {
        input_bits,
        output_bits,
        rows,
        component_count: u64::try_from(output_values - 1)
            .map_err(|_| VectorialDegreeError::ArithmeticOverflow)?,
        degree_histogram,
        max_component_degree,
        max_degree_witness,
    })
}

/// Computes the exact vectorial algebraic-degree profile under the default
/// work budget [`DEFAULT_VECTORIAL_DEGREE_MAX_WORK`].
///
/// # Errors
///
/// Returns [`VectorialDegreeError`] under the same conditions as
/// [`vectorial_degree_profile_with_work_limit`].
pub fn vectorial_degree_profile(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialDegreeProfile, VectorialDegreeError> {
    vectorial_degree_profile_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_DEGREE_MAX_WORK,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_map_has_only_nonzero_linear_components() {
        let table: Vec<u16> = (0u16..8).collect();
        let profile = vectorial_degree_profile(&table, 3, 3).unwrap();
        assert_eq!(profile.component_count, 7);
        assert_eq!(profile.degree_histogram, vec![0, 7, 0, 0]);
        assert_eq!(profile.max_component_degree, 1);
        assert_eq!(
            profile.max_degree_witness,
            VectorialDegreeWitness {
                output_mask: 1,
                degree: 1,
            }
        );
    }

    #[test]
    fn nonlinear_component_profile_is_exact() {
        // y0 = x0, y1 = x0*x1 under little-endian integer bit encoding.
        let table = [0u16, 1, 0, 3];
        let profile = vectorial_degree_profile(&table, 2, 2).unwrap();
        assert_eq!(profile.degree_histogram, vec![0, 1, 2]);
        assert_eq!(profile.max_component_degree, 2);
        assert_eq!(
            profile.max_degree_witness,
            VectorialDegreeWitness {
                output_mask: 2,
                degree: 2,
            }
        );
    }

    #[test]
    fn zero_map_uses_declared_degree_zero_convention() {
        let table = [0u16; 4];
        let profile = vectorial_degree_profile(&table, 2, 2).unwrap();
        assert_eq!(profile.degree_histogram, vec![3, 0, 0]);
        assert_eq!(profile.max_component_degree, 0);
        assert_eq!(profile.max_degree_witness.output_mask, 1);
    }

    #[test]
    fn malformed_input_and_work_budget_fail_closed() {
        assert_eq!(
            vectorial_degree_profile(&[0, 1, 2], 2, 2),
            Err(VectorialDegreeError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_degree_profile(&[0, 1, 2, 4], 2, 2),
            Err(VectorialDegreeError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
        assert_eq!(
            vectorial_degree_profile_with_work_limit(&[0, 1, 2, 3], 2, 2, 1),
            Err(VectorialDegreeError::WorkLimitExceeded {
                required: 36,
                limit: 1,
            })
        );
    }
}
