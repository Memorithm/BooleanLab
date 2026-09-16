//! Exact bounded component correlation-immunity profile for vectorial Boolean maps.
//!
//! Every non-zero output mask defines one scalar component. This module computes
//! the exact Walsh support of each component, derives its correlation-immunity
//! order, records component balancedness, and reports the vectorial resiliency
//! order only when every non-zero component is balanced. It is a BL-13
//! characterization oracle, not a cryptographic-security, novelty, or
//! performance claim.

use crate::{
    DEFAULT_VECTORIAL_MAX_WORK, MAX_VECTORIAL_INPUT_BITS, MAX_VECTORIAL_OUTPUT_BITS,
    VectorialMetricsError,
};

/// Exact per-component correlation-immunity summary for `F_2^n -> F_2^m`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VectorialComponentCorrelationProfile {
    /// Declared input width `n`.
    pub input_bits: u8,
    /// Declared output width `m`.
    pub output_bits: u8,
    /// Number of truth-table rows, exactly `2^n`.
    pub rows: usize,
    /// Number of non-zero output components, exactly `2^m - 1`.
    pub components: u128,
    /// `spectrum[t]` counts components with exact correlation-immunity order `t`.
    pub correlation_immunity_spectrum: Vec<u64>,
    /// Number of non-zero output components whose Walsh coefficient at zero is zero.
    pub balanced_components: u64,
    /// Minimum correlation-immunity order observed across non-zero components.
    pub min_correlation_immunity_order: u8,
    /// Maximum correlation-immunity order observed across non-zero components.
    pub max_correlation_immunity_order: u8,
    /// Vectorial resiliency order when every non-zero component is balanced.
    ///
    /// `None` means the map is not balanced in the vectorial sense and therefore
    /// is not resilient under this definition. `Some(0)` means balanced with no
    /// positive-order correlation-immunity guarantee.
    pub vectorial_resiliency_order: Option<u8>,
}

/// Compute the exact component correlation-immunity profile under the default
/// vectorial work budget.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] for unsupported dimensions, malformed
/// tables, out-of-range outputs, arithmetic/allocation failure, or work-limit
/// exhaustion.
pub fn vectorial_component_correlation_profile(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
) -> Result<VectorialComponentCorrelationProfile, VectorialMetricsError> {
    vectorial_component_correlation_profile_with_work_limit(
        table,
        input_bits,
        output_bits,
        DEFAULT_VECTORIAL_MAX_WORK,
    )
}

/// Compute the exact component correlation-immunity profile under an explicit
/// conservative work limit.
///
/// For one scalar component, correlation-immunity order `t` means every Walsh
/// coefficient at a non-zero input mask of Hamming weight at most `t` is zero.
/// The order is therefore one less than the smallest Hamming weight supporting a
/// non-zero Walsh coefficient, or `n` when no non-zero input mask is supported.
/// A vectorial map is reported as `t`-resilient only when every non-zero output
/// component is balanced and every such component has correlation-immunity order
/// at least `t`.
///
/// # Errors
///
/// Returns [`VectorialMetricsError`] under the same conditions as
/// [`vectorial_component_correlation_profile`].
pub fn vectorial_component_correlation_profile_with_work_limit(
    table: &[u16],
    input_bits: u8,
    output_bits: u8,
    max_work: u128,
) -> Result<VectorialComponentCorrelationProfile, VectorialMetricsError> {
    let (rows, output_values, components) =
        validate_table_and_work(table, input_bits, output_bits, max_work)?;

    let mut walsh = Vec::new();
    walsh
        .try_reserve_exact(rows)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    walsh.resize(rows, 0i64);

    let spectrum_len = usize::from(input_bits)
        .checked_add(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let mut spectrum = Vec::new();
    spectrum
        .try_reserve_exact(spectrum_len)
        .map_err(|_| VectorialMetricsError::AllocationFailed)?;
    spectrum.resize(spectrum_len, 0u64);

    let mut balanced_components = 0u64;
    let mut min_order = input_bits;
    let mut max_order = 0u8;

    for output_mask in 1..output_values {
        let output_mask =
            u16::try_from(output_mask).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        build_component(table, output_mask, &mut walsh);
        fwht(&mut walsh)?;

        if walsh[0] == 0 {
            balanced_components = balanced_components
                .checked_add(1)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        }
        let order = correlation_immunity_order(&walsh, input_bits)?;
        let bucket = spectrum
            .get_mut(usize::from(order))
            .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        *bucket = bucket
            .checked_add(1)
            .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
        min_order = min_order.min(order);
        max_order = max_order.max(order);
    }

    let component_count =
        u64::try_from(components).map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
    let vectorial_resiliency_order = (balanced_components == component_count).then_some(min_order);

    debug_assert_eq!(
        spectrum
            .iter()
            .map(|&count| u128::from(count))
            .sum::<u128>(),
        components
    );

    Ok(VectorialComponentCorrelationProfile {
        input_bits,
        output_bits,
        rows,
        components,
        correlation_immunity_spectrum: spectrum,
        balanced_components,
        min_correlation_immunity_order: min_order,
        max_correlation_immunity_order: max_order,
        vectorial_resiliency_order,
    })
}

fn build_component(table: &[u16], output_mask: u16, component: &mut [i64]) {
    for (value, slot) in table.iter().zip(component.iter_mut()) {
        let parity = (*value & output_mask).count_ones() & 1;
        *slot = if parity == 0 { 1 } else { -1 };
    }
}

fn correlation_immunity_order(walsh: &[i64], input_bits: u8) -> Result<u8, VectorialMetricsError> {
    let mut smallest_support_weight: Option<u8> = None;
    for (input_mask, &coefficient) in walsh.iter().enumerate().skip(1) {
        if coefficient == 0 {
            continue;
        }
        let weight = u8::try_from(input_mask.count_ones())
            .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?;
        smallest_support_weight = Some(match smallest_support_weight {
            Some(current) => current.min(weight),
            None => weight,
        });
    }
    match smallest_support_weight {
        Some(weight) => weight
            .checked_sub(1)
            .ok_or(VectorialMetricsError::ArithmeticOverflow),
        None => Ok(input_bits),
    }
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
    let components = u128::try_from(output_values)
        .map_err(|_| VectorialMetricsError::ArithmeticOverflow)?
        .checked_sub(1)
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let per_component = rows_u128
        .checked_mul(
            u128::from(input_bits)
                .checked_add(2)
                .ok_or(VectorialMetricsError::ArithmeticOverflow)?,
        )
        .ok_or(VectorialMetricsError::ArithmeticOverflow)?;
    let required = rows_u128
        .checked_add(
            components
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

    Ok((rows, output_values, components))
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
    fn identity_profile_matches_linear_component_weights() {
        let table: Vec<u16> = (0u16..16).collect();
        let profile = vectorial_component_correlation_profile(&table, 4, 4).unwrap();
        assert_eq!(profile.components, 15);
        assert_eq!(profile.balanced_components, 15);
        assert_eq!(profile.correlation_immunity_spectrum, vec![4, 6, 4, 1, 0]);
        assert_eq!(profile.min_correlation_immunity_order, 0);
        assert_eq!(profile.max_correlation_immunity_order, 3);
        assert_eq!(profile.vectorial_resiliency_order, Some(0));
    }

    #[test]
    fn present_profile_matches_exact_component_reference() {
        let present: [u16; 16] = [
            0xC, 0x5, 0x6, 0xB, 0x9, 0x0, 0xA, 0xD, 0x3, 0xE, 0xF, 0x8, 0x4, 0x7, 0x1, 0x2,
        ];
        let profile = vectorial_component_correlation_profile(&present, 4, 4).unwrap();
        assert_eq!(profile.balanced_components, 15);
        assert_eq!(profile.correlation_immunity_spectrum, vec![13, 2, 0, 0, 0]);
        assert_eq!(profile.vectorial_resiliency_order, Some(0));
    }

    #[test]
    fn zero_map_is_high_order_correlation_immune_but_not_resilient() {
        let table = [0u16; 16];
        let profile = vectorial_component_correlation_profile(&table, 4, 4).unwrap();
        assert_eq!(profile.balanced_components, 0);
        assert_eq!(profile.correlation_immunity_spectrum, vec![0, 0, 0, 0, 15]);
        assert_eq!(profile.min_correlation_immunity_order, 4);
        assert_eq!(profile.max_correlation_immunity_order, 4);
        assert_eq!(profile.vectorial_resiliency_order, None);
    }

    #[test]
    fn malformed_table_output_and_work_limit_fail_closed() {
        assert_eq!(
            vectorial_component_correlation_profile(&[0, 1, 2], 2, 2),
            Err(VectorialMetricsError::TableLengthMismatch {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            vectorial_component_correlation_profile(&[0, 1, 2, 4], 2, 2),
            Err(VectorialMetricsError::OutputOutOfRange {
                index: 3,
                value: 4,
                output_bits: 2,
            })
        );
        let identity: Vec<u16> = (0u16..16).collect();
        assert!(matches!(
            vectorial_component_correlation_profile_with_work_limit(&identity, 4, 4, 10),
            Err(VectorialMetricsError::WorkLimitExceeded { .. })
        ));
    }
}
