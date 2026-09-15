//! Exact bounded metrics for vectorial Boolean maps `F_2^n -> F_2^m`.
//!
//! This module provides reference semantics for differential uniformity and
//! component-Walsh nonlinearity. It is an exact small-domain oracle, not a
//! cryptographic-security claim, novelty claim, or performance benchmark.

use core::fmt;

pub const MAX_VECTORIAL_INPUT_BITS: u8 = 16;
pub const MAX_VECTORIAL_OUTPUT_BITS: u8 = 16;
pub const DEFAULT_VECTORIAL_MAX_WORK: u128 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DifferentialUniformityWitness {
    pub input_difference: u16,
    pub output_difference: u16,
    pub count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorialWalshWitness {
    pub input_mask: u16,
    pub output_mask: u16,
    pub coefficient: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorialBooleanMetrics {
    pub input_bits: u8,
    pub output_bits: u8,
    pub rows: usize,
    pub differential_uniformity: u32,
    pub differential_witness: DifferentialUniformityWitness,
    pub max_absolute_walsh: u64,
    pub walsh_witness: VectorialWalshWitness,
    pub vectorial_nonlinearity: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VectorialMetricsError {
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

impl fmt::Display for VectorialMetricsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputBitsOutOfRange { input_bits } => write!(
                f,
                "vectorial metrics require 1..={MAX_VECTORIAL_INPUT_BITS} input bits, got {input_bits}"
            ),
            Self::OutputBitsOutOfRange { output_bits } => write!(
                f,
                "vectorial metrics require 1..={MAX_VECTORIAL_OUTPUT_BITS} output bits, got {output_bits}"
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
            Self::ArithmeticOverflow => {
                f.write_str("vectorial metric work accounting overflowed")
            }
            Self::WorkLimitExceeded { required, limit } => write!(
                f,
                "vectorial metric exact work {required} exceeds declared limit {limit}"
            ),
            Self::AllocationFailed => {
                f.write_str("vectorial metric scratch allocation failed")
            }
        }
    }
}

impl std::error::Error for VectorialMetricsError {}

fn checked_pow2(bits: u8) -> Result<usize, VectorialMetricsError> {
    1usize
        .checked_shl(u32::from(bits))
        .ok_or(VectorialMetricsError::ArithmeticOverflow)
}

fn checked_mul(a: u128, b: u128) -> Result<u128, VectorialMetricsError> {
    a.checked_mul(b)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)
}

fn checked_add(a: u128, b: u128) -> Result<u128, VectorialMetricsError> {
    a.checked_add(b)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)
}

fn exact_work_bound(
    rows: usize,
    output_values: usize,
    input_bits: u8,
) -> Result<u128, VectorialMetricsError> {
    let rows = u128::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let output_values =
        u128::try_from(output_values).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let nonzero_inputs = rows
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let nonzero_outputs = output_values
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;

    // Differential pass: one exact difference evaluation for every x and
    // non-zero input difference, then one deterministic scan of each output
    // histogram to choose the canonical first maximum witness.
    let differential_evaluations = checked_mul(nonzero_inputs, rows)?;
    let differential_histogram_scans = checked_mul(nonzero_inputs, output_values)?;

    // Spectral pass: build one +/-1 component vector for every non-zero output
    // mask, then perform `input_bits` radix-2 Walsh stages. We count two exact
    // integer writes per butterfly, giving `rows * input_bits` integer outputs
    // per component, followed by a deterministic maximum scan.
    let component_build = checked_mul(nonzero_outputs, rows)?;
    let walsh_outputs = checked_mul(
        checked_mul(nonzero_outputs, rows)?,
        u128::from(input_bits),
    )?;
    let spectral_scan = checked_mul(nonzero_outputs, rows)?;

    checked_add(
        checked_add(differential_evaluations, differential_histogram_scans)?,
        checked_add(checked_add(component_build, walsh_outputs)?, spectral_scan)?,
    )
}

fn validate_table(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<(usize, usize), VectorialMetricsError> {
    if !(1..=MAX_VECTORIAL_INPUT_BITS).contains(&input_bits) {
        return Err(VectorialMetricsError::InputBitsOutOfRange { input_bits });
    }
    if !(1..=MAX_VECTORIAL_OUTPUT_BITS).contains(&output_bits) {
        return Err(VectorialMetricsError::OutputBitsOutOfRange { output_bits });
    }

    let rows = checked_pow2(input_bits)?;
    if table.len() != rows {
        return Err(VectorialMetricsError::TableLengthMismatch {
            expected: rows,
            actual: table.len(),
        });
    }
    let output_values = checked_pow2(output_bits)?;
    let output_limit = u32::try_from(output_values)
        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    for (index, &value) in table.iter().enumerate() {
        if u32::from(value) >= output_limit {
            return Err(VectorialMetricsError::OutputOutOfRange {
                index,
                value,
                output_bits,
            });
        }
    }

    let required = exact_work_bound(rows, output_values, input_bits)?;
    if required > max_work {
        return Err(VectorialMetricsError::WorkLimitExceeded {
            required,
            limit: max_work,
        });
    }
    Ok((rows, output_values))
}

fn differential_uniformity(
    table: &[u16],
    rows: usize,
    output_values: usize,
) -> Result<DifferentialUniformityWitness, VectorialMetricsError> {
    let mut counts = Vec::new();
    counts
        .try_reserve_exact(output_values)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    counts.resize(output_values, 0u32);

    let mut best = DifferentialUniformityWitness {
        input_difference: 1,
        output_difference: 0,
        count: 0,
    };
    for input_difference in 1..rows {
        counts.fill(0);
        for x in 0..rows {
            let output_difference = table[x] ^ table[x ^ input_difference];
            let slot = &mut counts[usize::from(output_difference)];
            *slot = slot
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        }
        for (output_difference, &count) in counts.iter().enumerate() {
            if count > best.count {
                best = DifferentialUniformityWitness {
                    input_difference: u16::try_from(input_difference)
                        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?,
                    output_difference: u16::try_from(output_difference)
                        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?,
                    count,
                };
            }
        }
    }
    Ok(best)
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

fn spectral_metrics(
    table: &[u16],
    rows: usize,
    output_values: usize,
) -> Result<(u64, VectorialWalshWitness, u64), VectorialMetricsError> {
    let mut component = Vec::new();
    component
        .try_reserve_exact(rows)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    component.resize(rows, 0i64);

    let mut max_absolute = 0u64;
    let mut witness = VectorialWalshWitness {
        input_mask: 0,
        output_mask: 1,
        coefficient: 0,
    };
    for output_mask in 1..output_values {
        let output_mask_u16 = u16::try_from(output_mask)
            .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        for (x, slot) in component.iter_mut().enumerate() {
            let parity = (table[x] & output_mask_u16).count_ones() & 1;
            *slot = if parity == 0 { 1 } else { -1 };
        }
        fwht(&mut component)?;
        for (input_mask, &coefficient) in component.iter().enumerate() {
            let absolute = coefficient.unsigned_abs();
            if absolute > max_absolute {
                max_absolute = absolute;
                witness = VectorialWalshWitness {
                    input_mask: u16::try_from(input_mask)
                        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?,
                    output_mask: output_mask_u16,
                    coefficient,
                };
            }
        }
    }

    let rows_u64 = u64::try_from(rows).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let vectorial_nonlinearity = rows_u64
        .checked_sub(max_absolute)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?
        / 2;
    Ok((max_absolute, witness, vectorial_nonlinearity))
}

/// Computes exact bounded differential and spectral metrics for one declared
/// vectorial Boolean truth table.
///
/// The table is indexed by the canonical integer encoding of `F_2^n`; each
/// `u16` value is the canonical integer encoding of one `F_2^m` output. The
/// differential witness is the first lexicographic `(a, b)` attaining the
/// maximum for non-zero `a`. The Walsh witness is the first output-mask-major,
/// input-mask-minor location attaining the maximum absolute coefficient.
///
/// # Errors
///
/// Fails closed on unsupported dimensions, malformed table length, outputs
/// outside the declared width, arithmetic/allocation failure, or when the
/// conservative exact-work bound exceeds `max_work` before scratch allocation.
pub fn vectorial_boolean_metrics_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialBooleanMetrics, VectorialMetricsError> {
    let (rows, output_values) = validate_table(table, input_bits, output_bits, max_work)?;
    let differential_witness = differential_uniformity(table, rows, output_values)?;
    let (max_absolute_walsh, walsh_witness, vectorial_nonlinearity) =
        spectral_metrics(table, rows, output_values)?;
    Ok(VectorialBooleanMetrics {
        input_bits,
        output_bits,
        rows,
        differential_uniformity: differential_witness.count,
        differential_witness,
        max_absolute_walsh,
        walsh_witness,
        vectorial_nonlinearity,
    })
}

/// Computes exact bounded vectorial Boolean metrics under the default work
/// budget [`DEFAULT_VECTORIAL_MAX_WORK`].
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] under the same conditions as
/// [`vectorial_boolean_metrics_with_work_limit`].
pub fn vectorial_boolean_metrics(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialBooleanMetrics, VectorialMetricsError> {
    vectorial_boolean_metrics_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_map_has_maximal_differential_uniformity_and_zero_nonlinearity() {
        let table: Vec<u16> = (0u16..16).collect();
        let metrics = vectorial_boolean_metrics(&table, 4, 4).unwrap();
        assert_eq!(metrics.differential_uniformity, 16);
        assert_eq!(
            metrics.differential_witness,
            DifferentialUniformityWitness {
                input_difference: 1,
                output_difference: 1,
                count: 16,
            }
        );
        assert_eq!(metrics.max_absolute_walsh, 16);
        assert_eq!(metrics.vectorial_nonlinearity, 0);
    }

    #[test]
    fn present_sbox_matches_reference_differential_and_spectral_metrics() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7,
            0x1, 0x2,
        ];
        let metrics = vectorial_boolean_metrics(&present, 4, 4).unwrap();
        assert_eq!(metrics.differential_uniformity, 4);
        assert_eq!(
            metrics.differential_witness,
            DifferentialUniformityWitness {
                input_difference: 1,
                output_difference: 3,
                count: 4,
            }
        );
        assert_eq!(metrics.max_absolute_walsh, 8);
        assert_eq!(
            metrics.walsh_witness,
            VectorialWalshWitness {
                input_mask: 9,
                output_mask: 1,
                coefficient: 8,
            }
        );
        assert_eq!(metrics.vectorial_nonlinearity, 4);
    }

    #[test]
    fn malformed_tables_and_outputs_fail_closed() {
        assert_eq!(
            vectorial_boolean_metrics(&[0, 1, 2], 2, 2),
            Err(VectorialMetricsError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_boolean_metrics(&[0, 1, 2, 4], 2, 2),
            Err(VectorialMetricsError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
    }

    #[test]
    fn work_limit_is_checked_before_exact_analysis() {
        let table: Vec<u16> = (0u16..16).collect();
        let error = vectorial_boolean_metrics_with_work_limit(&table, 4, 4, 1).unwrap_err();
        assert!(matches!(
            error,
            VectorialMetricsError::WorkLimitExceeded {
                required: _,
                limit: 1
            }
        ));
    }
}
