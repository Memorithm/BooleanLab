//! Exact static Boolean-mask construction for BL-14.1.
//!
//! Declared Boolean truth tables can either repeat over consecutive low index
//! bits or be evaluated on an explicit, ordered projection of index-address
//! bits. These constructions are input-independent and define exact mask
//! semantics only; they make no quality, latency, throughput, or hardware
//! claim.

use core::fmt;

use crate::sparsity::{ExactMask, SparsityError};

/// Errors for static Boolean-mask construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticMaskError {
    EmptyTruthTable,
    TruthTableLengthNotPowerOfTwo { length: usize },
    WidthNotMultipleOfTruthTable { total: usize, table_len: usize },
    AddressBitCountMismatch { expected: usize, actual: usize },
    DuplicateAddressBit { bit: usize },
    AddressBitOutOfRange { bit: usize },
    WidthNotMultipleOfAddressPeriod { total: usize, period: usize },
    Sparsity(SparsityError),
}

impl fmt::Display for StaticMaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for StaticMaskError {}

impl From<SparsityError> for StaticMaskError {
    fn from(error: SparsityError) -> Self {
        Self::Sparsity(error)
    }
}

fn validate_truth_table(truth_table: &[bool]) -> Result<(), StaticMaskError> {
    if truth_table.is_empty() {
        return Err(StaticMaskError::EmptyTruthTable);
    }
    if !truth_table.len().is_power_of_two() {
        return Err(StaticMaskError::TruthTableLengthNotPowerOfTwo {
            length: truth_table.len(),
        });
    }
    Ok(())
}

/// Materialize an input-independent Boolean mask from a declared truth table.
///
/// `truth_table[i] == true` means that index class is retained. The truth table
/// is repeated over `0..total`; requiring `total` to be an exact multiple of the
/// table length prevents an accidental partial final period from changing the
/// declared density.
///
/// The truth-table length must be a power of two so it can be interpreted as a
/// complete scalar Boolean function over some number of index-address bits.
/// A length-one table is therefore a valid zero-input constant function.
///
/// # Errors
///
/// Returns [`StaticMaskError::EmptyTruthTable`] for an empty table,
/// [`StaticMaskError::TruthTableLengthNotPowerOfTwo`] when the table cannot be a
/// complete Boolean truth table, [`StaticMaskError::WidthNotMultipleOfTruthTable`]
/// when the target width would truncate a period, or a wrapped
/// [`SparsityError`] for invalid mask widths.
pub fn static_mask_from_truth_table(
    total: usize,
    truth_table: &[bool],
) -> Result<ExactMask, StaticMaskError> {
    validate_truth_table(truth_table)?;
    if total == 0 {
        return Err(SparsityError::EmptyMask.into());
    }
    if !total.is_multiple_of(truth_table.len()) {
        return Err(StaticMaskError::WidthNotMultipleOfTruthTable {
            total,
            table_len: truth_table.len(),
        });
    }

    let retained_indices = (0..total)
        .filter(|index| truth_table[index % truth_table.len()])
        .collect::<Vec<_>>();
    ExactMask::from_retained_indices(total, &retained_indices).map_err(Into::into)
}

/// Materialize a static Boolean mask from explicitly selected index-address bits.
///
/// `address_bits[j]` supplies bit `j` of the truth-table row index, so the order
/// is part of the declared Boolean function. For a table of length `2^n`, exactly
/// `n` distinct address bits are required. The target width must cover a complete
/// period through the highest selected address bit; this prevents a truncated
/// domain from silently changing the declared density.
///
/// This is still a static mask: only the integer position is inspected. No data,
/// activation, weight, token, score, or runtime state participates in selection.
///
/// # Errors
///
/// Returns an error for malformed truth tables, zero target width, the wrong
/// number of address bits, duplicate bits, bits whose complete period cannot be
/// represented by `usize`, or a target width that truncates the selected address
/// period.
pub fn static_mask_from_index_bits(
    total: usize,
    truth_table: &[bool],
    address_bits: &[usize],
) -> Result<ExactMask, StaticMaskError> {
    validate_truth_table(truth_table)?;
    if total == 0 {
        return Err(SparsityError::EmptyMask.into());
    }

    let expected_bits = truth_table.len().trailing_zeros() as usize;
    if address_bits.len() != expected_bits {
        return Err(StaticMaskError::AddressBitCountMismatch {
            expected: expected_bits,
            actual: address_bits.len(),
        });
    }

    for (offset, &bit) in address_bits.iter().enumerate() {
        if bit >= usize::BITS as usize - 1 {
            return Err(StaticMaskError::AddressBitOutOfRange { bit });
        }
        if address_bits[..offset].contains(&bit) {
            return Err(StaticMaskError::DuplicateAddressBit { bit });
        }
    }

    if let Some(&highest_bit) = address_bits.iter().max() {
        let period = 1usize << (highest_bit + 1);
        if !total.is_multiple_of(period) {
            return Err(StaticMaskError::WidthNotMultipleOfAddressPeriod { total, period });
        }
    }

    let retained_indices = (0..total)
        .filter(|&index| {
            let row = address_bits
                .iter()
                .enumerate()
                .fold(0usize, |row, (table_bit, &address_bit)| {
                    row | (((index >> address_bit) & 1) << table_bit)
                });
            truth_table[row]
        })
        .collect::<Vec<_>>();

    ExactMask::from_retained_indices(total, &retained_indices).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{
        StaticMaskError, static_mask_from_index_bits, static_mask_from_truth_table,
    };
    use crate::sparsity::SparsityError;

    #[test]
    fn repeats_declared_truth_table_exactly() {
        let mask = static_mask_from_truth_table(8, &[false, true, true, false]).unwrap();
        assert_eq!(
            mask.as_slice(),
            &[false, true, true, false, false, true, true, false]
        );
        assert_eq!(mask.cardinality().retained(), 4);
        assert_eq!(mask.cardinality().total(), 8);
    }

    #[test]
    fn accepts_constant_boolean_functions() {
        let keep_all = static_mask_from_truth_table(4, &[true]).unwrap();
        let drop_all = static_mask_from_truth_table(4, &[false]).unwrap();
        assert_eq!(keep_all.cardinality().retained(), 4);
        assert_eq!(drop_all.cardinality().retained(), 0);

        let projected = static_mask_from_index_bits(4, &[true], &[]).unwrap();
        assert_eq!(projected, keep_all);
    }

    #[test]
    fn projects_declared_nonconsecutive_address_bits_exactly() {
        // XOR over index bits [0, 2]. address_bits[0] is the truth-table LSB.
        let mask = static_mask_from_index_bits(
            8,
            &[false, true, true, false],
            &[0, 2],
        )
        .unwrap();
        assert_eq!(
            mask.as_slice(),
            &[false, true, false, true, true, false, true, false]
        );
        assert_eq!(mask.cardinality().retained(), 4);
    }

    #[test]
    fn address_order_is_declared_semantics() {
        // f(a, b) = a. Swapping the selected address-bit order changes which
        // physical index bit is interpreted as input a.
        let table = [false, true, false, true];
        let low_first = static_mask_from_index_bits(8, &table, &[0, 2]).unwrap();
        let high_first = static_mask_from_index_bits(8, &table, &[2, 0]).unwrap();
        assert_eq!(
            low_first.as_slice(),
            &[false, true, false, true, false, true, false, true]
        );
        assert_eq!(
            high_first.as_slice(),
            &[false, false, false, false, true, true, true, true]
        );
    }

    #[test]
    fn rejects_invalid_address_mappings_fail_closed() {
        assert_eq!(
            static_mask_from_index_bits(8, &[false, true, true, false], &[0]),
            Err(StaticMaskError::AddressBitCountMismatch {
                expected: 2,
                actual: 1,
            })
        );
        assert_eq!(
            static_mask_from_index_bits(8, &[false, true, true, false], &[1, 1]),
            Err(StaticMaskError::DuplicateAddressBit { bit: 1 })
        );
    }

    #[test]
    fn rejects_truncated_address_period() {
        assert_eq!(
            static_mask_from_index_bits(6, &[false, true], &[2]),
            Err(StaticMaskError::WidthNotMultipleOfAddressPeriod {
                total: 6,
                period: 8,
            })
        );
    }

    #[test]
    fn rejects_non_boolean_table_shape() {
        assert_eq!(
            static_mask_from_truth_table(6, &[true, false, true]),
            Err(StaticMaskError::TruthTableLengthNotPowerOfTwo { length: 3 })
        );
    }

    #[test]
    fn rejects_partial_final_period() {
        assert_eq!(
            static_mask_from_truth_table(6, &[true, false, false, true]),
            Err(StaticMaskError::WidthNotMultipleOfTruthTable {
                total: 6,
                table_len: 4,
            })
        );
    }

    #[test]
    fn rejects_empty_inputs_fail_closed() {
        assert_eq!(
            static_mask_from_truth_table(8, &[]),
            Err(StaticMaskError::EmptyTruthTable)
        );
        assert_eq!(
            static_mask_from_truth_table(0, &[true]),
            Err(StaticMaskError::Sparsity(SparsityError::EmptyMask))
        );
        assert_eq!(
            static_mask_from_index_bits(0, &[true], &[]),
            Err(StaticMaskError::Sparsity(SparsityError::EmptyMask))
        );
    }
}
