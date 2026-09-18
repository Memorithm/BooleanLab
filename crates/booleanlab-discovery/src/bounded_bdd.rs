//! Resource-bounded BDD experiment for BL-BE3.
//!
//! This module is feature-gated and experimental. It uses an optional external
//! BDD implementation only after `BooleanLab` has bounded the enumerable input
//! domain, every binary apply operation's operand-pair work estimate, and the
//! result-node count. A limit hit is a non-result: it is never SAT/UNSAT
//! evidence and grants no production-runtime or actuation authority.

use biodivine_lib_bdd::{Bdd, BddValuation, BddVariableSet, op_function};

use crate::BooleanFunction;

/// Largest input domain admitted by this deliberately small BDD experiment.
pub const MAX_BDD_EXPERIMENT_INPUT_BITS: u32 = 8;

/// Explicit resource ceilings for one experimental BDD construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BddExperimentLimits {
    /// Maximum nodes in the result of each BDD binary operation.
    pub max_result_nodes: usize,
    /// Maximum `left_nodes * right_nodes` admitted before a binary operation.
    pub max_apply_pairs: usize,
}

impl Default for BddExperimentLimits {
    fn default() -> Self {
        Self {
            max_result_nodes: 512,
            max_apply_pairs: 65_536,
        }
    }
}
/// Why a bounded BDD experiment produced no qualified result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BddExperimentError {
    /// The exact Boolean function is wider than this bounded experiment.
    InputWidthTooLarge { input_bits: u32, maximum: u32 },
    /// A zero ceiling would make the resource contract ambiguous.
    ZeroResourceLimit,
    /// The operation was refused before dispatch because pair work was too large.
    ApplyPairLimit {
        estimated_pairs: usize,
        limit: usize,
    },
    /// The BDD backend refused an operation because its result-node limit hit.
    ResultNodeLimit { limit: usize },
    /// Exhaustive replay disagreed with the exact `BooleanLab` truth table.
    SemanticMismatch { assignment: usize },
}

/// Differential report from a qualified bounded construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BddExperimentReport {
    pub input_bits: u32,
    pub exhaustive_rows: usize,
    pub satisfying_rows: usize,
    pub final_nodes: usize,
    pub result_node_limit: usize,
    pub apply_pair_limit: usize,
}
fn bounded_binary_op(
    left: &Bdd,
    right: &Bdd,
    operation: fn(Option<bool>, Option<bool>) -> Option<bool>,
    limits: BddExperimentLimits,
) -> Result<Bdd, BddExperimentError> {
    let estimated_pairs =
        left.size()
            .checked_mul(right.size())
            .ok_or(BddExperimentError::ApplyPairLimit {
                estimated_pairs: usize::MAX,
                limit: limits.max_apply_pairs,
            })?;
    if estimated_pairs > limits.max_apply_pairs {
        return Err(BddExperimentError::ApplyPairLimit {
            estimated_pairs,
            limit: limits.max_apply_pairs,
        });
    }
    Bdd::binary_op_with_limit(limits.max_result_nodes, left, right, operation).ok_or(
        BddExperimentError::ResultNodeLimit {
            limit: limits.max_result_nodes,
        },
    )
}

/// Build and exhaustively differential-check an exact Boolean function with a
/// result-node-bounded BDD backend.
///
/// Construction streams a row-major DNF from the exact truth table. Every
/// binary AND/OR is screened before dispatch by `max_apply_pairs` and during
/// construction by the backend's result-node limit.
/// The final BDD is replayed on every `2^n` assignment against `BooleanLab`'s
/// exact source table. The pair ceiling is a conservative work proxy, not an
/// allocator-byte or wall-clock bound.
///
/// # Errors
///
/// Returns an input/resource-limit error or a semantic mismatch. Every error is
/// an explicit non-result.
pub fn run_bounded_bdd_experiment(
    function: &BooleanFunction,
    limits: BddExperimentLimits,
) -> Result<BddExperimentReport, BddExperimentError> {
    if function.input_bits() > MAX_BDD_EXPERIMENT_INPUT_BITS {
        return Err(BddExperimentError::InputWidthTooLarge {
            input_bits: function.input_bits(),
            maximum: MAX_BDD_EXPERIMENT_INPUT_BITS,
        });
    }
    if limits.max_result_nodes == 0 || limits.max_apply_pairs == 0 {
        return Err(BddExperimentError::ZeroResourceLimit);
    }

    let Ok(variable_count) = u16::try_from(function.input_bits()) else {
        return Err(BddExperimentError::InputWidthTooLarge {
            input_bits: function.input_bits(),
            maximum: MAX_BDD_EXPERIMENT_INPUT_BITS,
        });
    };
    let variables = BddVariableSet::new_anonymous(variable_count);
    let variable_handles = variables.variables();
    let mut result = variables.mk_false();
    let mut satisfying_rows = 0_usize;
    for (assignment, expected) in function.truth_table().iter().copied().enumerate() {
        if expected == 0 {
            continue;
        }
        satisfying_rows += 1;
        let mut term = variables.mk_true();
        for (index, variable) in variable_handles.iter().copied().enumerate() {
            let value = ((assignment >> index) & 1) != 0;
            let literal = variables.mk_literal(variable, value);
            term = bounded_binary_op(&term, &literal, op_function::and, limits)?;
        }
        result = bounded_binary_op(&result, &term, op_function::or, limits)?;
    }

    let Ok(input_bits) = usize::try_from(function.input_bits()) else {
        return Err(BddExperimentError::InputWidthTooLarge {
            input_bits: function.input_bits(),
            maximum: MAX_BDD_EXPERIMENT_INPUT_BITS,
        });
    };
    for (assignment, expected) in function.truth_table().iter().copied().enumerate() {
        let valuation = BddValuation::new(
            (0..input_bits)
                .map(|index| ((assignment >> index) & 1) != 0)
                .collect(),
        );
        if result.eval_in(&valuation) != (expected != 0) {
            return Err(BddExperimentError::SemanticMismatch { assignment });
        }
    }

    Ok(BddExperimentReport {
        input_bits: function.input_bits(),
        exhaustive_rows: function.truth_table().len(),
        satisfying_rows,
        final_nodes: result.size(),
        result_node_limit: limits.max_result_nodes,
        apply_pair_limit: limits.max_apply_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn function(bits: u32, f: impl Fn(u64) -> bool) -> BooleanFunction {
        BooleanFunction::from_fn(bits, f).expect("test function is inside exact bounds")
    }

    #[test]
    fn bounded_backend_matches_exact_oracle_for_representative_functions() {
        let cases = [
            function(3, |x| (x & 1 != 0) ^ (x & 2 != 0)),
            function(3, |x| (x & 1 != 0) && (x & 2 == 0) && (x & 4 != 0)),
            function(3, |_| true),
            function(3, |_| false),
        ];
        for case in &cases {
            let report = run_bounded_bdd_experiment(case, BddExperimentLimits::default())
                .expect("small exact case fits the bounded experiment");
            assert_eq!(report.exhaustive_rows, 8);
            assert!(report.final_nodes <= report.result_node_limit);
        }
    }
    #[test]
    fn result_node_limit_is_a_non_result() {
        let xor = function(3, |x| (x & 1 != 0) ^ (x & 2 != 0));
        assert_eq!(
            run_bounded_bdd_experiment(
                &xor,
                BddExperimentLimits {
                    max_result_nodes: 1,
                    max_apply_pairs: usize::MAX,
                },
            ),
            Err(BddExperimentError::ResultNodeLimit { limit: 1 })
        );
    }

    #[test]
    fn apply_pair_limit_is_checked_before_backend_dispatch() {
        let one_row = function(3, |x| x == 0);
        assert!(matches!(
            run_bounded_bdd_experiment(
                &one_row,
                BddExperimentLimits {
                    max_result_nodes: 512,
                    max_apply_pairs: 1,
                },
            ),
            Err(BddExperimentError::ApplyPairLimit { limit: 1, .. })
        ));
    }
    #[test]
    fn width_and_zero_limits_fail_closed() {
        let too_wide = function(MAX_BDD_EXPERIMENT_INPUT_BITS + 1, |_| false);
        assert_eq!(
            run_bounded_bdd_experiment(&too_wide, BddExperimentLimits::default()),
            Err(BddExperimentError::InputWidthTooLarge {
                input_bits: MAX_BDD_EXPERIMENT_INPUT_BITS + 1,
                maximum: MAX_BDD_EXPERIMENT_INPUT_BITS,
            })
        );

        let small = function(2, |_| false);
        assert_eq!(
            run_bounded_bdd_experiment(
                &small,
                BddExperimentLimits {
                    max_result_nodes: 0,
                    max_apply_pairs: 1,
                },
            ),
            Err(BddExperimentError::ZeroResourceLimit)
        );
    }
}
