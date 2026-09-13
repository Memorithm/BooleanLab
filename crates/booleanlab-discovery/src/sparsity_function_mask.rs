//! Generic exact Boolean-function to BL-14 dynamic-mask materialization.
//!
//! Search method identity is deliberately irrelevant here. A function proposed
//! by exhaustive search, bounded circuit generation, SAT/MaxSAT, CEGIS, Forge,
//! or another future engine must pass through the same exact truth-table and
//! BL-14.3 predicate-row semantics before sparsity evidence is produced.

use booleanlab_core::{DynamicMaskError, ExactMask, dynamic_mask_from_predicates};

use crate::BooleanFunction;

/// Materialize one exact scalar Boolean function over explicit predicate rows.
///
/// `BooleanFunction` stores outputs as exact `0/1` bytes. This adapter converts
/// that representation losslessly and delegates evaluation to the already
/// qualified BL-14.3 dynamic-mask implementation. Predicate extraction remains
/// outside this function and must be frozen separately by the experiment.
///
/// # Errors
///
/// Propagates [`DynamicMaskError`] for empty predicate rows, predicate arity
/// drift, malformed mask domains, or any other BL-14.3 fail-closed condition.
pub fn materialize_boolean_function_mask(
    function: &BooleanFunction,
    predicate_rows: &[&[bool]],
) -> Result<ExactMask, DynamicMaskError> {
    let truth_table = function
        .truth_table()
        .iter()
        .map(|&bit| bit == 1)
        .collect::<Vec<_>>();
    dynamic_mask_from_predicates(&truth_table, predicate_rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materializes_xor_under_declared_truth_table_row_order() {
        let function = BooleanFunction::new(2, vec![0, 1, 1, 0]).unwrap();
        let rows: [&[bool]; 4] = [
            &[false, false],
            &[true, false],
            &[false, true],
            &[true, true],
        ];

        let mask = materialize_boolean_function_mask(&function, &rows).unwrap();
        assert_eq!(mask.as_slice(), &[false, true, true, false]);
        assert_eq!(mask.cardinality().retained(), 2);
        assert_eq!(mask.cardinality().total(), 4);
    }

    #[test]
    fn materialization_is_independent_of_search_method_metadata() {
        let function = BooleanFunction::new(2, vec![0, 1, 1, 0]).unwrap();
        let first_rows: [&[bool]; 3] = [&[true, false], &[false, true], &[true, true]];
        let second_rows: [&[bool]; 3] = [&[true, false], &[false, true], &[true, true]];

        let first = materialize_boolean_function_mask(&function, &first_rows).unwrap();
        let second = materialize_boolean_function_mask(&function, &second_rows).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn propagates_predicate_arity_failure() {
        let function = BooleanFunction::new(2, vec![0, 1, 1, 0]).unwrap();
        let one = [true];
        let rows: [&[bool]; 1] = [&one];

        assert_eq!(
            materialize_boolean_function_mask(&function, &rows),
            Err(DynamicMaskError::PredicateArityMismatch {
                index: 0,
                expected: 2,
                actual: 1,
            })
        );
    }
}
