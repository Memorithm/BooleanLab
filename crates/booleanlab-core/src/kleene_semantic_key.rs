//! Exact packed semantic identities for bounded Strong-Kleene programs.
//!
//! The key in this module is collision-free for the exhaustively evaluated
//! domain: it stores the declared arity, row count, and every output value in
//! canonical assignment order. It is an exact differential artifact for
//! BL-BE1, not a probabilistic hash and not a production-runtime identity.

use crate::{
    KleeneAnalysisError, KleeneInstruction, KleeneProgramAnalysis, KleeneValue,
    analyze_kleene_program_with_work_budget,
};

/// Schema version for [`KleeneSemanticKey`].
pub const KLEENE_SEMANTIC_KEY_SCHEMA_VERSION: u16 = 1;

/// Exact packed semantic identity of one exhaustively evaluated program.
///
/// Outputs are packed four rows per byte using two bits per Strong-Kleene value:
/// `False = 0b00`, `Unknown = 0b01`, and `True = 0b10`. Unused high bits in the
/// final byte are zero. The declared arity is part of the identity, so constant
/// programs evaluated over different input domains remain distinct artifacts.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KleeneSemanticKey {
    /// Encoding schema version.
    pub schema_version: u16,
    /// Declared number of inputs used to construct the exhaustive domain.
    pub input_arity: usize,
    /// Number of rows encoded in [`Self::packed_outputs`].
    pub rows: usize,
    /// Canonical two-bit packed outputs in base-3 assignment order.
    pub packed_outputs: Vec<u8>,
}

/// Failure while constructing an exact semantic key from a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneSemanticKeyError {
    /// Exhaustive analysis could not be completed under the declared bounds.
    Analysis(KleeneAnalysisError),
}

impl KleeneSemanticKey {
    /// Builds a collision-free packed key from an already completed exact analysis.
    ///
    /// This operation performs no semantic evaluation. Equality between two keys
    /// means equality of arity, exhaustive row count, and every encoded output.
    #[must_use]
    pub fn from_analysis(analysis: &KleeneProgramAnalysis) -> Self {
        let mut packed_outputs = vec![0_u8; analysis.rows.div_ceil(4)];
        for (row, value) in analysis.outputs.iter().copied().enumerate() {
            let encoded = match value {
                KleeneValue::False => 0_u8,
                KleeneValue::Unknown => 1_u8,
                KleeneValue::True => 2_u8,
            };
            let byte = row / 4;
            let shift = (row % 4) * 2;
            packed_outputs[byte] |= encoded << shift;
        }

        Self {
            schema_version: KLEENE_SEMANTIC_KEY_SCHEMA_VERSION,
            input_arity: analysis.input_arity,
            rows: analysis.rows,
            packed_outputs,
        }
    }
}

/// Exhaustively evaluates a program and returns its exact packed semantic key.
///
/// The same row and instruction-evaluation budgets used by
/// [`analyze_kleene_program_with_work_budget`] are enforced before a key is
/// constructed. Budget exhaustion is therefore an explicit non-result rather
/// than an approximate semantic identity.
///
/// # Errors
///
/// Returns [`KleeneSemanticKeyError::Analysis`] when exhaustive analysis is
/// malformed, overflows, or exceeds either declared budget.
pub fn kleene_semantic_key(
    program: &[KleeneInstruction],
    input_arity: usize,
    max_rows: usize,
    max_instruction_evaluations: usize,
) -> Result<KleeneSemanticKey, KleeneSemanticKeyError> {
    let analysis = analyze_kleene_program_with_work_budget(
        program,
        input_arity,
        max_rows,
        max_instruction_evaluations,
    )
    .map_err(KleeneSemanticKeyError::Analysis)?;
    Ok(KleeneSemanticKey::from_analysis(&analysis))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_input_identity_has_stable_two_bit_encoding() {
        let key = kleene_semantic_key(&[KleeneInstruction::Input(0)], 1, 3, 3)
            .expect("one-input identity must fit exact bounds");

        assert_eq!(key.schema_version, 1);
        assert_eq!(key.input_arity, 1);
        assert_eq!(key.rows, 3);
        assert_eq!(key.packed_outputs, vec![0x24]);
    }

    #[test]
    fn semantically_equivalent_programs_share_the_same_key() {
        let direct = [KleeneInstruction::Input(0)];
        let redundant = [
            KleeneInstruction::Input(0),
            KleeneInstruction::Input(1),
            KleeneInstruction::Constant(KleeneValue::False),
            KleeneInstruction::And,
            KleeneInstruction::Or,
        ];

        let direct_key = kleene_semantic_key(&direct, 2, 9, 9).expect("direct program is valid");
        let redundant_key =
            kleene_semantic_key(&redundant, 2, 9, 45).expect("redundant program is valid");

        assert_eq!(direct_key, redundant_key);
    }

    #[test]
    fn unknown_is_not_collapsed_into_false() {
        let unknown = kleene_semantic_key(
            &[KleeneInstruction::Constant(KleeneValue::Unknown)],
            0,
            1,
            1,
        )
        .expect("constant unknown is valid");
        let false_key = kleene_semantic_key(
            &[KleeneInstruction::Constant(KleeneValue::False)],
            0,
            1,
            1,
        )
        .expect("constant false is valid");

        assert_ne!(unknown, false_key);
        assert_eq!(unknown.packed_outputs, vec![0x01]);
        assert_eq!(false_key.packed_outputs, vec![0x00]);
    }

    #[test]
    fn declared_arity_is_part_of_semantic_identity() {
        let constant = [KleeneInstruction::Constant(KleeneValue::True)];
        let arity_zero = kleene_semantic_key(&constant, 0, 1, 1).expect("arity zero is valid");
        let arity_one = kleene_semantic_key(&constant, 1, 3, 3).expect("arity one is valid");

        assert_ne!(arity_zero, arity_one);
    }

    #[test]
    fn analysis_budget_failure_is_preserved_as_a_non_result() {
        let program = [KleeneInstruction::Input(0), KleeneInstruction::Not];
        assert_eq!(
            kleene_semantic_key(&program, 1, 3, 5),
            Err(KleeneSemanticKeyError::Analysis(
                KleeneAnalysisError::WorkLimitExceeded {
                    required_instruction_evaluations: 6,
                    max_instruction_evaluations: 5,
                }
            ))
        );
    }
}
