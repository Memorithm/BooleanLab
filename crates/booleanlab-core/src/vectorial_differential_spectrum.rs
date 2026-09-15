//! Exact bounded differential spectrum for vectorial Boolean maps.
//!
//! The spectrum records how many DDT cells have each exact count when the input
//! difference is non-zero. It is a small-domain reference metric for BL-13 and
//! does not by itself establish cryptographic security, equivalence, novelty or
//! performance.

use crate::{
    DEFAULT_VECTORIAL_MAX_WORK, MAX_VECTORIAL_INPUT_BITS, MAX_VECTORIAL_OUTPUT_BITS,
    VectorialMetricsError,
};

/// Exact histogram of DDT cell counts over every non-zero input difference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorialDifferentialSpectrum {
    /// Declared input width `n` for `F_2^n`.
    pub input_bits: u8,
    /// Declared output width `m` for `F_2^m`.
    pub output_bits: u8,
    /// Number of truth-table rows, exactly `2^n`.
    pub rows: usize,
    /// Number of DDT cells included, `(2^n - 1) * 2^m`.
    pub cells: u128,
    /// `spectrum[c]` is the number of included DDT cells whose exact count is `c`.
    pub spectrum: Vec<u64>,
    /// Largest exact DDT cell count present in the spectrum.
    pub differential_uniformity: u32,
}

/// Compute the exact differential spectrum under the default vectorial work
/// budget.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] for unsupported dimensions, malformed
/// tables, out-of-range outputs, arithmetic/allocation failure or work-limit
/// exhaustion.
pub fn vectorial_differential_spectrum(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialDifferentialSpectrum, VectorialMetricsError> {
    vectorial_differential_spectrum_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

/// Compute the exact differential spectrum under an explicit conservative work
/// limit.
///
/// The work bound includes truth-table validation plus, for every non-zero input
/// difference, complete output-histogram reset, exact difference evaluation and
/// complete histogram scan. The bound is checked before scratch allocation.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] under the same conditions as
/// [`vectorial_differential_spectrum`].
pub fn vectorial_differential_spectrum_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialDifferentialSpectrum, VectorialMetricsError> {
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

    let rows_u128 =
        u128::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let outputs_u128 =
        u128::try_from(output_values).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let nonzero_inputs = rows_u128
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let per_difference = rows_u128
        .checked_add(
            outputs_u128
                .checked_mul(2)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?,
        )
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let required = rows_u128
        .checked_add(
            nonzero_inputs
                .checked_mul(per_difference)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?,
        )
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    if required > max_work {
        return Err(VectorialMetricsError::WorkLimitExceeded {
            required,
            limit: max_work,
        });
    }

    let mut counts = Vec::new();
    counts
        .try_reserve_exact(output_values)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    counts.resize(output_values, 0u32);

    let spectrum_len = rows
        .checked_add(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let mut spectrum = Vec::new();
    spectrum
        .try_reserve_exact(spectrum_len)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    spectrum.resize(spectrum_len, 0u64);

    let mut differential_uniformity = 0u32;
    for input_difference in 1..rows {
        counts.fill(0);
        for x in 0..rows {
            let output_difference = table[x] ^ table[x ^ input_difference];
            let slot = &mut counts[usize::from(output_difference)];
            *slot = slot
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        }
        for &count in &counts {
            let bucket = spectrum
                .get_mut(usize::try_from(count).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            *bucket = bucket
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
            differential_uniformity = differential_uniformity.max(count);
        }
    }

    let cells = nonzero_inputs
        .checked_mul(outputs_u128)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    debug_assert_eq!(
        spectrum.iter().map(|&count| u128::from(count)).sum::<u128>(),
        cells
    );

    Ok(VectorialDifferentialSpectrum {
        input_bits,
        output_bits,
        rows,
        cells,
        spectrum,
        differential_uniformity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_map_has_only_zero_and_full_count_cells() {
        let table: Vec<u16> = (0u16..16).collect();
        let spectrum = vectorial_differential_spectrum(&table, 4, 4).unwrap();
        assert_eq!(spectrum.cells, 240);
        assert_eq!(spectrum.differential_uniformity, 16);
        assert_eq!(spectrum.spectrum[0], 225);
        assert_eq!(spectrum.spectrum[16], 15);
        assert_eq!(
            spectrum.spectrum.iter().copied().sum::<u64>(),
            u64::try_from(spectrum.cells).unwrap()
        );
    }

    #[test]
    fn present_sbox_matches_exact_reference_spectrum() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let spectrum = vectorial_differential_spectrum(&present, 4, 4).unwrap();
        assert_eq!(spectrum.differential_uniformity, 4);
        assert_eq!(spectrum.spectrum[0], 144);
        assert_eq!(spectrum.spectrum[2], 72);
        assert_eq!(spectrum.spectrum[4], 24);
        assert_eq!(
            spectrum
                .spectrum
                .iter()
                .enumerate()
                .filter(|(_, count)| **count != 0)
                .map(|(value, count)| (value, *count))
                .collect::<Vec<_>>(),
            vec![(0, 144), (2, 72), (4, 24)]
        );
    }

    #[test]
    fn spectrum_invariants_match_ddt_row_accounting() {
        let table = [0u16, 3, 1, 2];
        let spectrum = vectorial_differential_spectrum(&table, 2, 2).unwrap();
        let cell_count = spectrum.spectrum.iter().map(|&count| u128::from(count)).sum::<u128>();
        let weighted = spectrum
            .spectrum
            .iter()
            .enumerate()
            .map(|(value, &frequency)| value as u128 * u128::from(frequency))
            .sum::<u128>();
        assert_eq!(cell_count, spectrum.cells);
        assert_eq!(weighted, 3 * 4);
    }

    #[test]
    fn malformed_table_output_and_work_limit_fail_closed() {
        assert_eq!(
            vectorial_differential_spectrum(&[0, 1, 2], 2, 2),
            Err(VectorialMetricsError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_differential_spectrum(&[0, 1, 2, 4], 2, 2),
            Err(VectorialMetricsError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
        let identity: Vec<u16> = (0u16..16).collect();
        assert!(matches!(
            vectorial_differential_spectrum_with_work_limit(&identity, 4, 4, 10),
            Err(VectorialMetricsError::WorkLimitExceeded { .. })
        ));
    }
}
