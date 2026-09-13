//! BL-14.5 bounded exhaustive Boolean-rule proposal search.
//!
//! This module enumerates the complete scalar Boolean function space for a
//! deliberately small predicate arity. It is a SEARCH-phase proposal generator:
//! it does not inspect task quality, select a Pareto frontier, consume HOLDOUT
//! evidence, or make a sparsity/performance claim.

use std::fmt;

use crate::{BooleanFunction, FunctionError};

/// Largest predicate arity admitted by the exact exhaustive BL-14.5 enumerator.
///
/// Four inputs already contain `2^(2^4) = 65_536` scalar Boolean functions. The
/// cap is intentionally explicit so an accidental request cannot trigger an
/// explosive allocation at five inputs (`2^32` functions).
pub const MAX_EXHAUSTIVE_SPARSITY_RULE_BITS: u32 = 4;

/// One exact function proposed by bounded exhaustive enumeration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExhaustiveRuleProposal {
    /// Stable deterministic identifier. Identity is still the exact truth table,
    /// not this string.
    pub proposal_id: String,
    /// Complete scalar Boolean truth table.
    pub function: BooleanFunction,
    /// Integer whose bit `row` is the exact truth-table output for that row.
    pub truth_table_code: u64,
    /// Exact number of truth-table rows that evaluate to one.
    pub output_ones: u32,
}

/// Fail-closed errors for bounded exhaustive BL-14.5 proposal generation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExhaustiveRuleSearchError {
    ZeroInputs,
    InputWidthTooLarge { requested: u32, maximum: u32 },
    PopulationTooLarge { function_count: u64 },
    Function(FunctionError),
}

/// Enumerate every scalar Boolean function for the declared predicate arity.
///
/// Truth-table row zero is stored in the least-significant bit of
/// [`ExhaustiveRuleProposal::truth_table_code`]. Enumeration order is therefore
/// exactly integer-code order from zero through `2^(2^n)-1`.
///
/// This function is deliberately limited to
/// [`MAX_EXHAUSTIVE_SPARSITY_RULE_BITS`]. Candidate evaluation, Pareto
/// screening, SEARCH freezing and HOLDOUT validation remain separate BL-14.5
/// stages.
///
/// # Errors
///
/// Returns [`ExhaustiveRuleSearchError::ZeroInputs`] for zero predicates,
/// [`ExhaustiveRuleSearchError::InputWidthTooLarge`] above the declared bounded
/// exhaustive-search cap, [`ExhaustiveRuleSearchError::PopulationTooLarge`] if
/// the exact population cannot be represented by the host allocation index, or
/// propagates an exact [`FunctionError`] if function construction fails
/// unexpectedly.
pub fn propose_exhaustive_rules(
    input_bits: u32,
) -> Result<Vec<ExhaustiveRuleProposal>, ExhaustiveRuleSearchError> {
    if input_bits == 0 {
        return Err(ExhaustiveRuleSearchError::ZeroInputs);
    }
    if input_bits > MAX_EXHAUSTIVE_SPARSITY_RULE_BITS {
        return Err(ExhaustiveRuleSearchError::InputWidthTooLarge {
            requested: input_bits,
            maximum: MAX_EXHAUSTIVE_SPARSITY_RULE_BITS,
        });
    }

    let rows = 1_u32 << input_bits;
    let function_count = 1_u64 << rows;
    let capacity = usize::try_from(function_count)
        .map_err(|_| ExhaustiveRuleSearchError::PopulationTooLarge { function_count })?;
    let mut proposals = Vec::with_capacity(capacity);

    for truth_table_code in 0..function_count {
        let truth_table = (0..rows)
            .map(|row| u8::from(((truth_table_code >> row) & 1) != 0))
            .collect::<Vec<_>>();
        let output_ones = truth_table_code.count_ones();
        let function = BooleanFunction::new(input_bits, truth_table)
            .map_err(ExhaustiveRuleSearchError::Function)?;
        proposals.push(ExhaustiveRuleProposal {
            proposal_id: format!("bl14-exhaustive-n{input_bits}-{truth_table_code:016x}"),
            function,
            truth_table_code,
            output_ones,
        });
    }

    Ok(proposals)
}

impl fmt::Display for ExhaustiveRuleSearchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ExhaustiveRuleSearchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_and_unbounded_predicate_arity() {
        assert_eq!(
            propose_exhaustive_rules(0),
            Err(ExhaustiveRuleSearchError::ZeroInputs)
        );
        assert_eq!(
            propose_exhaustive_rules(MAX_EXHAUSTIVE_SPARSITY_RULE_BITS + 1),
            Err(ExhaustiveRuleSearchError::InputWidthTooLarge {
                requested: MAX_EXHAUSTIVE_SPARSITY_RULE_BITS + 1,
                maximum: MAX_EXHAUSTIVE_SPARSITY_RULE_BITS,
            })
        );
    }

    #[test]
    fn one_bit_function_space_is_complete_and_exactly_ordered() {
        let proposals = propose_exhaustive_rules(1).unwrap();
        assert_eq!(proposals.len(), 4);
        assert_eq!(proposals[0].function.truth_table(), &[0, 0]);
        assert_eq!(proposals[1].function.truth_table(), &[1, 0]);
        assert_eq!(proposals[2].function.truth_table(), &[0, 1]);
        assert_eq!(proposals[3].function.truth_table(), &[1, 1]);
        assert_eq!(
            proposals
                .iter()
                .map(|proposal| proposal.truth_table_code)
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn proposal_identity_and_output_weight_are_deterministic() {
        let first = propose_exhaustive_rules(2).unwrap();
        let second = propose_exhaustive_rules(2).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 16);

        let xor = &first[0b0110];
        assert_eq!(xor.function.truth_table(), &[0, 1, 1, 0]);
        assert_eq!(xor.output_ones, 2);
        assert_eq!(xor.truth_table_code, 0b0110);
        assert_eq!(xor.proposal_id, "bl14-exhaustive-n2-0000000000000006");
    }

    #[test]
    fn four_bit_bound_enumerates_the_declared_complete_population() {
        let proposals = propose_exhaustive_rules(4).unwrap();
        assert_eq!(proposals.len(), 65_536);
        assert_eq!(proposals.first().unwrap().output_ones, 0);
        assert_eq!(proposals.last().unwrap().output_ones, 16);
        assert!(
            proposals
                .first()
                .unwrap()
                .function
                .truth_table()
                .iter()
                .all(|&x| x == 0)
        );
        assert!(
            proposals
                .last()
                .unwrap()
                .function
                .truth_table()
                .iter()
                .all(|&x| x == 1)
        );
    }
}
