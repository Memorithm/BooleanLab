//! Exact bounded component-weight spectrum for vectorial Boolean maps.
//!
//! Every non-zero output mask defines one scalar component. This module counts
//! the exact Hamming weight of each such component and records how many are
//! balanced. It is a BL-13 characterization oracle only: balanced components do
//! not establish cryptographic suitability, EA/CCZ equivalence, novelty, or
//! hardware performance.

use crate::{
    DEFAULT_VECTORIAL_MAX_WORK, MAX_VECTORIAL_INPUT_BITS, MAX_VECTORIAL_OUTPUT_BITS,
    VectorialMetricsError,
};

/// Exact histogram of scalar-component Hamming weights for `F_2^n -> F_2^m`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorialComponentWeightSpectrum {
    /// Declared input width `n`.
    pub input_bits: u8,
    /// Declared output width `m`.
    pub output_bits: u8,
    /// Number of truth-table rows, exactly `2^n`.
    pub rows: usize,
    /// Number of non-zero output masks, exactly `2^m - 1`.
    pub components: u128,
    /// `spectrum[w]` is the number of non-zero output components with weight `w`.
    pub spectrum: Vec<u64>,
    /// Number of components having weight exactly `2^(n-1)`.
    pub balanced_components: u64,
    /// Minimum component weight observed across non-zero output masks.
    pub min_weight: u64,
    /// Maximum component weight observed across non-zero output masks.
    pub max_weight: u64,
}

/// Compute the exact component-weight spectrum under the default vectorial work
/// budget.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] for unsupported dimensions, malformed
/// tables, out-of-range outputs, arithmetic/allocation failure, or work-limit
/// exhaustion.
pub fn vectorial_component_weight_spectrum(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialComponentWeightSpectrum, VectorialMetricsError> {
    vectorial_component_weight_spectrum_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

/// Compute the exact component-weight spectrum under an explicit conservative
/// work limit.
///
/// The work bound counts table validation plus one parity evaluation for every
/// truth-table row and every non-zero output mask. It is checked before the
/// output histogram is allocated.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] under the same conditions as
/// [`vectorial_component_weight_spectrum`].
pub fn vectorial_component_weight_spectrum_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialComponentWeightSpectrum, VectorialMetricsError> {
    if !(1..=MAX_VECTORIAL_INPUT_BITS).contains(&input_bits) {
        return Err(VectorialMetricsError::InputBitsOutOfRange { input_bits });
    }
    if !(1..=MAX_VECTORIAL_OUTPUT_BITS).contains(&output_bits) {
        return Err(VectorialMetricsError::OutputBitsOutOfRange { output_bits });
    }

    let rows = 1usize
        .checked_shl(u32::from(input_bits))
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    if table.len() != rows {
        return Err(VectorialMetricsError::TableLengthMismatch {
            expected: rows,
            actual: table.len(),
        });
    }

    let output_values = 1usize
        .checked_shl(u32::from(output_bits))
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let output_limit =
        u32::try_from(output_values).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    for (index, &value) in table.iter().enumerate() {
        if u32::from(value) >= output_limit {
            return Err(VectorialMetricsError::OutputOutOfRange {
                index,
                value,
                output_bits,
            });
        }
    }

    let rows_u128 = u128::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let components = u128::try_from(output_values)
        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let required = rows_u128
        .checked_add(
            rows_u128
                .checked_mul(components)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?,
        )
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    if required > max_work {
        return Err(VectorialMetricsError::WorkLimitExceeded {
            required,
            limit: max_work,
        });
    }

    let spectrum_len = rows
        .checked_add(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let mut spectrum = Vec::new();
    spectrum
        .try_reserve_exact(spectrum_len)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    spectrum.resize(spectrum_len, 0u64);

    let balanced_weight = rows / 2;
    let mut balanced_components = 0u64;
    let mut min_weight = u64::MAX;
    let mut max_weight = 0u64;

    for output_mask in 1..output_values {
        let mask =
            u16::try_from(output_mask).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        let mut weight = 0usize;
        for &value in table {
            if (value & mask).count_ones() & 1 == 1 {
                weight = weight
                    .checked_add(1)
                    .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            }
        }

        let bucket = spectrum
            .get_mut(weight)
            .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        *bucket = bucket
            .checked_add(1)
            .ok_or(VectorialMetricsError::ArithmeticOverflow)?;

        if weight == balanced_weight {
            balanced_components = balanced_components
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        }
        let weight_u64 =
            u64::try_from(weight).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        min_weight = min_weight.min(weight_u64);
        max_weight = max_weight.max(weight_u64);
    }

    debug_assert_eq!(
        spectrum.iter().map(|&count| u128::from(count)).sum::<u128>(),
        components
    );

    Ok(VectorialComponentWeightSpectrum {
        input_bits,
        output_bits,
        rows,
        components,
        spectrum,
        balanced_components,
        min_weight,
        max_weight,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_has_every_nonzero_component_balanced() {
        let table: Vec<u16> = (0u16..16).collect();
        let spectrum = vectorial_component_weight_spectrum(&table, 4, 4).unwrap();
        assert_eq!(spectrum.components, 15);
        assert_eq!(spectrum.balanced_components, 15);
        assert_eq!(spectrum.min_weight, 8);
        assert_eq!(spectrum.max_weight, 8);
        assert_eq!(spectrum.spectrum[8], 15);
    }

    #[test]
    fn present_permutation_has_every_nonzero_component_balanced() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let spectrum = vectorial_component_weight_spectrum(&present, 4, 4).unwrap();
        assert_eq!(spectrum.balanced_components, 15);
        assert_eq!(spectrum.spectrum[8], 15);
        assert_eq!(
            spectrum.spectrum.iter().map(|&count| u128::from(count)).sum::<u128>(),
            15
        );
    }

    #[test]
    fn zero_map_has_only_zero_weight_components() {
        let table = [0u16; 16];
        let spectrum = vectorial_component_weight_spectrum(&table, 4, 4).unwrap();
        assert_eq!(spectrum.balanced_components, 0);
        assert_eq!(spectrum.min_weight, 0);
        assert_eq!(spectrum.max_weight, 0);
        assert_eq!(spectrum.spectrum[0], 15);
    }

    #[test]
    fn malformed_output_and_work_limit_fail_closed() {
        assert_eq!(
            vectorial_component_weight_spectrum(&[0, 1, 2], 2, 2),
            Err(VectorialMetricsError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_component_weight_spectrum(&[0, 1, 2, 4], 2, 2),
            Err(VectorialMetricsError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
        let identity: Vec<u16> = (0u16..16).collect();
        assert!(matches!(
            vectorial_component_weight_spectrum_with_work_limit(&identity, 4, 4, 10),
            Err(VectorialMetricsError::WorkLimitExceeded { .. })
        ));
    }
}
