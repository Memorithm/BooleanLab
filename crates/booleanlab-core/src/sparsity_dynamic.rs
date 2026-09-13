//! Exact dynamic Boolean-mask construction for BL-14.3.
//!
//! Callers provide one explicit Boolean predicate row per candidate element.
//! The declared truth table maps each predicate row to a retain/drop decision.
//! Predicate extraction remains outside this module so experiments can freeze
//! and audit that observation boundary independently from Boolean selection.

use core::fmt;

use crate::sparsity::{ExactMask, SparsityError};

/// Errors for dynamic Boolean-mask construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DynamicMaskError {
    EmptyTruthTable,
    TruthTableLengthNotPowerOfTwo {
        length: usize,
    },
    EmptyPredicateRows,
    PredicateArityMismatch {
        index: usize,
        expected: usize,
        actual: usize,
    },
    Sparsity(SparsityError),
}

impl fmt::Display for DynamicMaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DynamicMaskError {}

impl From<SparsityError> for DynamicMaskError {
    fn from(error: SparsityError) -> Self {
        Self::Sparsity(error)
    }
}

/// Materialize an input-conditioned Boolean mask from explicit predicate rows.
///
/// `predicate_rows[i]` is the declared Boolean observation for candidate `i`.
/// Predicate bit `j` supplies bit `j` of the truth-table row index. The truth
/// table must therefore contain exactly `2^n` entries when each predicate row
/// has arity `n`. A one-entry table is a valid zero-predicate constant rule.
///
/// This function does not derive predicates from activations, weights, tokens,
/// scores, or runtime state. Experiments remain responsible for freezing that
/// observation boundary and for measuring controller overhead separately.
///
/// # Errors
///
/// Returns an error for an empty or non-power-of-two truth table, an empty set
/// of candidates, predicate rows whose arity differs from the truth-table arity,
/// or an invalid exact mask.
pub fn dynamic_mask_from_predicates(
    truth_table: &[bool],
    predicate_rows: &[&[bool]],
) -> Result<ExactMask, DynamicMaskError> {
    if truth_table.is_empty() {
        return Err(DynamicMaskError::EmptyTruthTable);
    }
    if !truth_table.len().is_power_of_two() {
        return Err(DynamicMaskError::TruthTableLengthNotPowerOfTwo {
            length: truth_table.len(),
        });
    }
    if predicate_rows.is_empty() {
        return Err(DynamicMaskError::EmptyPredicateRows);
    }

    let expected_arity = truth_table.len().trailing_zeros() as usize;
    let mut retained_indices = Vec::new();

    for (index, predicates) in predicate_rows.iter().enumerate() {
        if predicates.len() != expected_arity {
            return Err(DynamicMaskError::PredicateArityMismatch {
                index,
                expected: expected_arity,
                actual: predicates.len(),
            });
        }

        let row = predicates
            .iter()
            .enumerate()
            .fold(0usize, |row, (bit, predicate)| {
                row | (usize::from(*predicate) << bit)
            });
        if truth_table[row] {
            retained_indices.push(index);
        }
    }

    ExactMask::from_retained_indices(predicate_rows.len(), &retained_indices).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::{DynamicMaskError, dynamic_mask_from_predicates};

    #[test]
    fn evaluates_each_predicate_row_exactly() {
        // XOR over two predicates, with predicate 0 as the truth-table LSB.
        let rows: [&[bool]; 4] = [
            &[false, false],
            &[true, false],
            &[false, true],
            &[true, true],
        ];
        let mask = dynamic_mask_from_predicates(&[false, true, true, false], &rows).unwrap();

        assert_eq!(mask.as_slice(), &[false, true, true, false]);
        assert_eq!(mask.cardinality().retained(), 2);
        assert_eq!(mask.cardinality().total(), 4);
    }

    #[test]
    fn predicate_order_is_declared_semantics() {
        // f(a, b) = a. Reversing the predicate order changes the decision.
        let low_true = [true, false];
        let high_true = [false, true];
        let rows: [&[bool]; 2] = [&low_true, &high_true];
        let mask = dynamic_mask_from_predicates(&[false, true, false, true], &rows).unwrap();

        assert_eq!(mask.as_slice(), &[true, false]);
    }

    #[test]
    fn accepts_constant_rules() {
        let empty: [bool; 0] = [];
        let rows: [&[bool]; 2] = [&empty, &empty];
        let keep = dynamic_mask_from_predicates(&[true], &rows).unwrap();
        let drop = dynamic_mask_from_predicates(&[false], &rows).unwrap();

        assert_eq!(keep.as_slice(), &[true, true]);
        assert_eq!(drop.as_slice(), &[false, false]);
    }

    #[test]
    fn rejects_malformed_inputs_fail_closed() {
        let one = [true];
        let two = [true, false];
        let mismatched: [&[bool]; 2] = [&one, &two];

        assert_eq!(
            dynamic_mask_from_predicates(&[], &[&one]),
            Err(DynamicMaskError::EmptyTruthTable)
        );
        assert_eq!(
            dynamic_mask_from_predicates(&[false, true, false], &[&one]),
            Err(DynamicMaskError::TruthTableLengthNotPowerOfTwo { length: 3 })
        );
        assert_eq!(
            dynamic_mask_from_predicates(&[false, true], &[]),
            Err(DynamicMaskError::EmptyPredicateRows)
        );
        assert_eq!(
            dynamic_mask_from_predicates(&[false, true], &mismatched),
            Err(DynamicMaskError::PredicateArityMismatch {
                index: 1,
                expected: 1,
                actual: 2,
            })
        );
    }
}
