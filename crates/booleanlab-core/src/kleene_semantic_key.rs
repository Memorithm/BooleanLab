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

/// Failure while constructing an exact semantic key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneSemanticKeyError {
    /// Exhaustive analysis could not be completed under the declared bounds.
    Analysis(KleeneAnalysisError),
    /// A caller-supplied analysis does not describe the complete declared domain.
    InconsistentAnalysis {
        input_arity: usize,
        declared_rows: usize,
        output_rows: usize,
        expected_rows: usize,
    },
}

impl KleeneSemanticKey {
    /// Builds a collision-free packed key from an already completed exact analysis.
    ///
    /// This operation performs no semantic evaluation. Because
    /// [`KleeneProgramAnalysis`] has public fields, the declared row count and
    /// output length are revalidated against `3^input_arity` before allocation.
    /// Equality between two successfully constructed keys therefore means
    /// equality of arity, exhaustive row count, and every encoded output.
    ///
    /// # Errors
    ///
    /// Returns [`KleeneSemanticKeyError::Analysis`] when `3^input_arity`
    /// overflows `usize`, or [`KleeneSemanticKeyError::InconsistentAnalysis`]
    /// when either public row field does not match the complete declared domain.
    pub fn from_analysis(
        analysis: &KleeneProgramAnalysis,
    ) -> Result<Self, KleeneSemanticKeyError> {
        let expected_rows = checked_pow3(analysis.input_arity).ok_or(
            KleeneSemanticKeyError::Analysis(KleeneAnalysisError::EnumerationOverflow {
                input_arity: analysis.input_arity,
            }),
        )?;
        if analysis.rows != expected_rows || analysis.outputs.len() != expected_rows {
            return Err(KleeneSemanticKeyError::InconsistentAnalysis {
                input_arity: analysis.input_arity,
                declared_rows: analysis.rows,
                output_rows: analysis.outputs.len(),
                expected_rows,
            });
        }

        let mut packed_outputs = vec![0_u8; expected_rows.div_ceil(4)];
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

        Ok(Self {
            schema_version: KLEENE_SEMANTIC_KEY_SCHEMA_VERSION,
            input_arity: analysis.input_arity,
            rows: expected_rows,
            packed_outputs,
        })
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
/// malformed, overflows, or exceeds either declared budget. The completed
/// analysis is revalidated before packing and can therefore also report
/// [`KleeneSemanticKeyError::InconsistentAnalysis`] if its public invariants are
/// ever violated.
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
    KleeneSemanticKey::from_analysis(&analysis)
}

fn checked_pow3(exponent: usize) -> Option<usize> {
    let mut value = 1_usize;
    for _ in 0..exponent {
        value = value.checked_mul(3)?;
    }
    Some(value)
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
        let false_key =
            kleene_semantic_key(&[KleeneInstruction::Constant(KleeneValue::False)], 0, 1, 1)
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
    fn rejects_short_public_analysis_before_packing() {
        let analysis = KleeneProgramAnalysis {
            input_arity: 1,
            rows: 3,
            outputs: vec![KleeneValue::False; 2],
            always_true: false,
            always_false: false,
            redundant_inputs: Vec::new(),
        };

        assert_eq!(
            KleeneSemanticKey::from_analysis(&analysis),
            Err(KleeneSemanticKeyError::InconsistentAnalysis {
                input_arity: 1,
                declared_rows: 3,
                output_rows: 2,
                expected_rows: 3,
            })
        );
    }

    #[test]
    fn rejects_long_public_analysis_before_packing() {
        let analysis = KleeneProgramAnalysis {
            input_arity: 0,
            rows: 1,
            outputs: vec![KleeneValue::False; 5],
            always_true: false,
            always_false: true,
            redundant_inputs: Vec::new(),
        };

        assert_eq!(
            KleeneSemanticKey::from_analysis(&analysis),
            Err(KleeneSemanticKeyError::InconsistentAnalysis {
                input_arity: 0,
                declared_rows: 1,
                output_rows: 5,
                expected_rows: 1,
            })
        );
    }

    #[test]
    fn rejects_forged_row_count_before_packing() {
        let analysis = KleeneProgramAnalysis {
            input_arity: 1,
            rows: 1,
            outputs: vec![KleeneValue::False; 3],
            always_true: false,
            always_false: true,
            redundant_inputs: Vec::new(),
        };

        assert_eq!(
            KleeneSemanticKey::from_analysis(&analysis),
            Err(KleeneSemanticKeyError::InconsistentAnalysis {
                input_arity: 1,
                declared_rows: 1,
                output_rows: 3,
                expected_rows: 3,
            })
        );
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
