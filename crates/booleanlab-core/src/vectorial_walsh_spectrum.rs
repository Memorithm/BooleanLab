//! Exact bounded absolute Walsh spectrum for vectorial Boolean maps.
//!
//! The spectrum records how many component-Walsh coefficients have each exact
//! absolute value over every non-zero output mask. It is a small-domain BL-13
//! reference metric and does not establish cryptographic security, EA/CCZ
//! equivalence, novelty, mechanism, or performance.

use crate::{
    DEFAULT_VECTORIAL_MAX_WORK, MAX_VECTORIAL_INPUT_BITS, MAX_VECTORIAL_OUTPUT_BITS,
    VectorialMetricsError,
};

/// Exact histogram of absolute component-Walsh coefficients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorialWalshSpectrum {
    /// Declared input width `n` for `F_2^n`.
    pub input_bits: u8,
    /// Declared output width `m` for `F_2^m`.
    pub output_bits: u8,
    /// Number of truth-table rows, exactly `2^n`.
    pub rows: usize,
    /// Number of included coefficients, `(2^m - 1) * 2^n`.
    pub coefficients: u128,
    /// `spectrum[a]` is the number of included coefficients with absolute value `a`.
    pub spectrum: Vec<u64>,
    /// Largest absolute component-Walsh coefficient present.
    pub max_absolute_walsh: u64,
    /// Vectorial nonlinearity derived from [`Self::max_absolute_walsh`].
    pub vectorial_nonlinearity: u64,
}

/// Compute the exact absolute Walsh spectrum under the default vectorial work
/// budget.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] for unsupported dimensions, malformed
/// tables, out-of-range outputs, arithmetic/allocation failure, or work-limit
/// exhaustion.
pub fn vectorial_walsh_spectrum(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialWalshSpectrum, VectorialMetricsError> {
    vectorial_walsh_spectrum_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

/// Compute the exact absolute Walsh spectrum under an explicit conservative
/// work limit.
///
/// The work bound includes truth-table validation and, for every non-zero
/// output mask, component construction, all radix-2 Walsh stages, and a full
/// coefficient scan. It is checked before scratch allocation.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] under the same conditions as
/// [`vectorial_walsh_spectrum`].
pub fn vectorial_walsh_spectrum_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialWalshSpectrum, VectorialMetricsError> {
    let (rows, output_values, nonzero_outputs) =
        validate_table_and_work(table, input_bits, output_bits, max_work)?;

    let mut component = Vec::new();
    component
        .try_reserve_exact(rows)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    component.resize(rows, 0i64);

    let spectrum_len = rows
        .checked_add(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let mut spectrum = Vec::new();
    spectrum
        .try_reserve_exact(spectrum_len)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    spectrum.resize(spectrum_len, 0u64);

    let mut max_absolute_walsh = 0u64;
    for output_mask in 1..output_values {
        let output_mask_u16 =
            u16::try_from(output_mask).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        for (x, slot) in component.iter_mut().enumerate() {
            let parity = (table[x] & output_mask_u16).count_ones() & 1;
            *slot = if parity == 0 { 1 } else { -1 };
        }
        fwht(&mut component)?;
        for &coefficient in &component {
            let absolute = coefficient.unsigned_abs();
            let bucket = spectrum
                .get_mut(
                    usize::try_from(absolute)
                        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?,
                )
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            *bucket = bucket
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            max_absolute_walsh = max_absolute_walsh.max(absolute);
        }
    }

    let rows_u128 = u128::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let coefficients = nonzero_outputs
        .checked_mul(rows_u128)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    debug_assert_eq!(
        spectrum
            .iter()
            .map(|&count| u128::from(count))
            .sum::<u128>(),
        coefficients
    );

    let rows_u64 = u64::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let vectorial_nonlinearity = rows_u64
        .checked_sub(max_absolute_walsh)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?
        / 2;

    Ok(VectorialWalshSpectrum {
        input_bits,
        output_bits,
        rows,
        coefficients,
        spectrum,
        max_absolute_walsh,
        vectorial_nonlinearity,
    })
}

fn validate_table_and_work(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<(usize, usize, u128), VectorialMetricsError> {
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
    let outputs_u128 =
        u128::try_from(output_values).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let nonzero_outputs = outputs_u128
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let per_component = rows_u128
        .checked_mul(u128::from(input_bits).checked_add(2).ok_or(
            VectorialMetricsError::ArithmeticOverflow,
        )?)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let required = rows_u128
        .checked_add(
            nonzero_outputs
                .checked_mul(per_component)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?,
        )
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    if required > max_work {
        return Err(VectorialMetricsError::WorkLimitExceeded {
            required,
            limit: max_work,
        });
    }

    Ok((rows, output_values, nonzero_outputs))
}

fn fwht(values: &mut [i64]) -> Result<(), VectorialMetricsError> {
    let mut half = 1usize;
    while half < values.len() {
        let width = half
            .checked_mul(2)
            .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        for start in (0..values.len()).step_by(width) {
            for offset in 0..half {
                let left = values[start + offset];
                let right = values[start + offset + half];
                values[start + offset] = left
                    .checked_add(right)
                    .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
                values[start + offset + half] = left
                    .checked_sub(right)
                    .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            }
        }
        half = width;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_map_has_only_zero_and_full_absolute_coefficients() {
        let table: Vec<u16> = (0u16..16).collect();
        let spectrum = vectorial_walsh_spectrum(&table, 4, 4).unwrap();
        assert_eq!(spectrum.coefficients, 240);
        assert_eq!(spectrum.max_absolute_walsh, 16);
        assert_eq!(spectrum.vectorial_nonlinearity, 0);
        assert_eq!(spectrum.spectrum[0], 225);
        assert_eq!(spectrum.spectrum[16], 15);
    }

    #[test]
    fn present_sbox_matches_exact_absolute_reference_spectrum() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let spectrum = vectorial_walsh_spectrum(&present, 4, 4).unwrap();
        assert_eq!(spectrum.max_absolute_walsh, 8);
        assert_eq!(spectrum.vectorial_nonlinearity, 4);
        assert_eq!(spectrum.spectrum[0], 108);
        assert_eq!(spectrum.spectrum[4], 96);
        assert_eq!(spectrum.spectrum[8], 36);
        assert_eq!(
            spectrum
                .spectrum
                .iter()
                .enumerate()
                .filter(|(_, count)| **count != 0)
                .map(|(value, count)| (value, *count))
                .collect::<Vec<_>>(),
            vec![(0, 108), (4, 96), (8, 36)]
        );
    }

    #[test]
    fn spectrum_obeys_component_parseval_accounting() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let spectrum = vectorial_walsh_spectrum(&present, 4, 4).unwrap();
        let coefficient_count = spectrum
            .spectrum
            .iter()
            .map(|&count| u128::from(count))
            .sum::<u128>();
        let square_sum = spectrum
            .spectrum
            .iter()
            .enumerate()
            .map(|(absolute, &frequency)| {
                let absolute = u128::try_from(absolute).unwrap();
                absolute * absolute * u128::from(frequency)
            })
            .sum::<u128>();
        assert_eq!(coefficient_count, spectrum.coefficients);
        assert_eq!(square_sum, 15 * 16 * 16);
    }

    #[test]
    fn malformed_table_output_and_work_limit_fail_closed() {
        assert_eq!(
            vectorial_walsh_spectrum(&[0, 1, 2], 2, 2),
            Err(VectorialMetricsError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_walsh_spectrum(&[0, 1, 2, 4], 2, 2),
            Err(VectorialMetricsError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
        let identity: Vec<u16> = (0u16..16).collect();
        assert!(matches!(
            vectorial_walsh_spectrum_with_work_limit(&identity, 4, 4, 10),
            Err(VectorialMetricsError::WorkLimitExceeded { .. })
        ));
    }
}
