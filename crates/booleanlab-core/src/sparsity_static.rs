//! Exact static Boolean-mask construction for BL-14.1.
//!
//! A declared Boolean truth table is repeated over a fixed index domain to
//! produce one input-independent keep/drop mask. This module only defines the
//! exact mask semantics; it makes no quality, latency, throughput, or hardware
//! claim.

use core::fmt;

use crate::sparsity::{ExactMask, SparsityError};

/// Errors for static Boolean-mask construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticMaskError {
    EmptyTruthTable,
    TruthTableLengthNotPowerOfTwo { length: usize },
    WidthNotMultipleOfTruthTable { total: usize, table_len: usize },
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
    if truth_table.is_empty() {
        return Err(StaticMaskError::EmptyTruthTable);
    }
    if !truth_table.len().is_power_of_two() {
        return Err(StaticMaskError::TruthTableLengthNotPowerOfTwo {
            length: truth_table.len(),
        });
    }
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

#[cfg(test)]
mod tests {
    use super::{StaticMaskError, static_mask_from_truth_table};
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
    }
}
