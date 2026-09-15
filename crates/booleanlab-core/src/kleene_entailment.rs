//! Exact designated-value entailment for bounded Strong-Kleene semantics.
//!
//! This module starts BL-BE3 with a small-domain relation that can be checked
//! directly from collision-free [`crate::KleeneSemanticKey`] artifacts. The
//! relation uses `True` as the designated antecedent value: an antecedent
//! entails a consequent exactly when every row on which the antecedent is
//! `True` also makes the consequent `True`.
//!
//! `Unknown` is never collapsed into `False`. An `Unknown` consequent on a row
//! where the antecedent is `True` is retained as a distinct counterexample.
//! This is an exact bounded research relation, not a production authorization
//! rule and not a performance claim.

use crate::{KLEENE_SEMANTIC_KEY_SCHEMA_VERSION, KleeneSemanticKey, KleeneValue};

/// Identifies which key failed structural validation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneEntailmentSide {
    /// The antecedent key.
    Antecedent,
    /// The consequent key.
    Consequent,
}

/// Failure while validating or comparing exact semantic keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneEntailmentError {
    /// A key uses an unsupported encoding schema.
    UnsupportedSchema {
        side: KleeneEntailmentSide,
        found: u16,
    },
    /// `3^input_arity` cannot be represented as `usize`.
    DomainOverflow {
        side: KleeneEntailmentSide,
        input_arity: usize,
    },
    /// A public key does not encode the complete declared domain.
    InconsistentShape {
        side: KleeneEntailmentSide,
        input_arity: usize,
        declared_rows: usize,
        expected_rows: usize,
        packed_len: usize,
        expected_packed_len: usize,
    },
    /// A row uses the reserved two-bit encoding `0b11`.
    InvalidValueEncoding {
        side: KleeneEntailmentSide,
        row: usize,
    },
    /// Unused high bits in the final packed byte are non-zero.
    NonCanonicalPadding {
        side: KleeneEntailmentSide,
        byte: u8,
        allowed_mask: u8,
    },
    /// Keys describe different declared input domains.
    DomainMismatch {
        antecedent_input_arity: usize,
        consequent_input_arity: usize,
    },
}

/// First exact row showing that designated-value entailment does not hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneEntailmentWitness {
    /// Canonical base-3 assignment index.
    pub assignment_index: usize,
    /// Antecedent output on the witness row. This is always `True`.
    pub antecedent: KleeneValue,
    /// Consequent output on the witness row. `Unknown` and `False` remain distinct.
    pub consequent: KleeneValue,
}

/// Result of exact designated-value entailment over one exhaustive domain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneEntailment {
    /// Every row with antecedent `True` also has consequent `True`.
    Entails {
        /// Exhaustively checked rows.
        rows: usize,
        /// Number of rows on which the antecedent is designated `True`.
        antecedent_true_rows: usize,
    },
    /// At least one designated antecedent row has a non-`True` consequent.
    DoesNotEntail {
        /// Exhaustive domain size.
        rows: usize,
        /// First violating row in canonical assignment order.
        witness: KleeneEntailmentWitness,
    },
}

/// Checks exact designated-value entailment between two semantic keys.
///
/// The inputs must be canonical keys for the same complete `3^n` domain. Both
/// public key structures are fully validated before comparison, including
/// schema version, row count, packed length, reserved encodings and unused
/// padding bits.
///
/// This relation intentionally differs from evaluating the connective
/// `!antecedent OR consequent`: Strong-Kleene `Unknown -> Unknown` under that
/// connective is `Unknown`, while designated-value entailment remains reflexive
/// because only rows with antecedent `True` impose a requirement. This makes the
/// relation suitable for exact guard-subsumption experiments without erasing
/// three-valued observations.
///
/// # Errors
///
/// Returns a structural validation error for either key, or
/// [`KleeneEntailmentError::DomainMismatch`] when the declared input arities
/// differ.
pub fn kleene_designated_entails(
    antecedent: &KleeneSemanticKey,
    consequent: &KleeneSemanticKey,
) -> Result<KleeneEntailment, KleeneEntailmentError> {
    validate_key(antecedent, KleeneEntailmentSide::Antecedent)?;
    validate_key(consequent, KleeneEntailmentSide::Consequent)?;

    if antecedent.input_arity != consequent.input_arity {
        return Err(KleeneEntailmentError::DomainMismatch {
            antecedent_input_arity: antecedent.input_arity,
            consequent_input_arity: consequent.input_arity,
        });
    }

    let mut antecedent_true_rows = 0usize;
    for row in 0..antecedent.rows {
        let antecedent_value = decode_row(antecedent, row);
        if antecedent_value != KleeneValue::True {
            continue;
        }

        antecedent_true_rows += 1;
        let consequent_value = decode_row(consequent, row);
        if consequent_value != KleeneValue::True {
            return Ok(KleeneEntailment::DoesNotEntail {
                rows: antecedent.rows,
                witness: KleeneEntailmentWitness {
                    assignment_index: row,
                    antecedent: antecedent_value,
                    consequent: consequent_value,
                },
            });
        }
    }

    Ok(KleeneEntailment::Entails {
        rows: antecedent.rows,
        antecedent_true_rows,
    })
}

fn validate_key(
    key: &KleeneSemanticKey,
    side: KleeneEntailmentSide,
) -> Result<(), KleeneEntailmentError> {
    if key.schema_version != KLEENE_SEMANTIC_KEY_SCHEMA_VERSION {
        return Err(KleeneEntailmentError::UnsupportedSchema {
            side,
            found: key.schema_version,
        });
    }

    let expected_rows =
        checked_pow3(key.input_arity).ok_or(KleeneEntailmentError::DomainOverflow {
            side,
            input_arity: key.input_arity,
        })?;
    let expected_packed_len = expected_rows.div_ceil(4);
    if key.rows != expected_rows || key.packed_outputs.len() != expected_packed_len {
        return Err(KleeneEntailmentError::InconsistentShape {
            side,
            input_arity: key.input_arity,
            declared_rows: key.rows,
            expected_rows,
            packed_len: key.packed_outputs.len(),
            expected_packed_len,
        });
    }

    for row in 0..expected_rows {
        if encoded_row(key, row) == 0b11 {
            return Err(KleeneEntailmentError::InvalidValueEncoding { side, row });
        }
    }

    let used_values = expected_rows % 4;
    if used_values != 0 {
        let allowed_mask = match used_values {
            1 => 0b0000_0011,
            2 => 0b0000_1111,
            3 => 0b0011_1111,
            _ => unreachable!("a partial packed byte contains one to three values"),
        };
        let byte = *key
            .packed_outputs
            .last()
            .expect("a complete Strong-Kleene domain contains at least one row");
        if byte & !allowed_mask != 0 {
            return Err(KleeneEntailmentError::NonCanonicalPadding {
                side,
                byte,
                allowed_mask,
            });
        }
    }

    Ok(())
}

fn encoded_row(key: &KleeneSemanticKey, row: usize) -> u8 {
    let byte = key.packed_outputs[row / 4];
    (byte >> ((row % 4) * 2)) & 0b11
}

fn decode_row(key: &KleeneSemanticKey, row: usize) -> KleeneValue {
    match encoded_row(key, row) {
        0b00 => KleeneValue::False,
        0b01 => KleeneValue::Unknown,
        0b10 => KleeneValue::True,
        _ => unreachable!("validated semantic keys never contain the reserved encoding"),
    }
}

fn checked_pow3(exponent: usize) -> Option<usize> {
    let mut value = 1usize;
    for _ in 0..exponent {
        value = value.checked_mul(3)?;
    }
    Some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KleeneInstruction, kleene_semantic_key};

    fn key(program: &[KleeneInstruction], input_arity: usize) -> KleeneSemanticKey {
        let rows = checked_pow3(input_arity).expect("small test domain must fit");
        let work = rows
            .checked_mul(program.len())
            .expect("small test work must fit");
        kleene_semantic_key(program, input_arity, rows, work)
            .expect("test program must produce an exact key")
    }

    #[test]
    fn entailment_is_reflexive_even_when_unknown_is_observed() {
        let identity = key(&[KleeneInstruction::Input(0)], 1);

        assert_eq!(
            kleene_designated_entails(&identity, &identity),
            Ok(KleeneEntailment::Entails {
                rows: 3,
                antecedent_true_rows: 1,
            })
        );
    }

    #[test]
    fn antecedent_without_true_rows_entails_vacuously() {
        let always_false = key(&[KleeneInstruction::Constant(KleeneValue::False)], 1);
        let always_unknown = key(&[KleeneInstruction::Constant(KleeneValue::Unknown)], 1);

        assert_eq!(
            kleene_designated_entails(&always_false, &always_unknown),
            Ok(KleeneEntailment::Entails {
                rows: 3,
                antecedent_true_rows: 0,
            })
        );
    }

    #[test]
    fn unknown_consequent_is_a_distinct_counterexample() {
        let always_true = key(&[KleeneInstruction::Constant(KleeneValue::True)], 0);
        let always_unknown = key(&[KleeneInstruction::Constant(KleeneValue::Unknown)], 0);

        assert_eq!(
            kleene_designated_entails(&always_true, &always_unknown),
            Ok(KleeneEntailment::DoesNotEntail {
                rows: 1,
                witness: KleeneEntailmentWitness {
                    assignment_index: 0,
                    antecedent: KleeneValue::True,
                    consequent: KleeneValue::Unknown,
                },
            })
        );
    }

    #[test]
    fn conjunction_entails_each_operand_but_not_conversely() {
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let x_and_y = key(
            &[
                KleeneInstruction::Input(0),
                KleeneInstruction::Input(1),
                KleeneInstruction::And,
            ],
            2,
        );

        assert_eq!(
            kleene_designated_entails(&x_and_y, &x),
            Ok(KleeneEntailment::Entails {
                rows: 9,
                antecedent_true_rows: 1,
            })
        );
        assert_eq!(
            kleene_designated_entails(&x, &x_and_y),
            Ok(KleeneEntailment::DoesNotEntail {
                rows: 9,
                witness: KleeneEntailmentWitness {
                    assignment_index: 2,
                    antecedent: KleeneValue::True,
                    consequent: KleeneValue::False,
                },
            })
        );
    }

    #[test]
    fn rejects_reserved_value_encoding_before_comparison() {
        let invalid = KleeneSemanticKey {
            schema_version: KLEENE_SEMANTIC_KEY_SCHEMA_VERSION,
            input_arity: 0,
            rows: 1,
            packed_outputs: vec![0b11],
        };
        let valid = key(&[KleeneInstruction::Constant(KleeneValue::True)], 0);

        assert_eq!(
            kleene_designated_entails(&invalid, &valid),
            Err(KleeneEntailmentError::InvalidValueEncoding {
                side: KleeneEntailmentSide::Antecedent,
                row: 0,
            })
        );
    }

    #[test]
    fn rejects_noncanonical_padding_before_comparison() {
        let invalid = KleeneSemanticKey {
            schema_version: KLEENE_SEMANTIC_KEY_SCHEMA_VERSION,
            input_arity: 1,
            rows: 3,
            packed_outputs: vec![0b1100_0000],
        };
        let valid = key(&[KleeneInstruction::Constant(KleeneValue::False)], 1);

        assert_eq!(
            kleene_designated_entails(&invalid, &valid),
            Err(KleeneEntailmentError::NonCanonicalPadding {
                side: KleeneEntailmentSide::Antecedent,
                byte: 0b1100_0000,
                allowed_mask: 0b0011_1111,
            })
        );
    }

    #[test]
    fn rejects_different_declared_domains() {
        let zero = key(&[KleeneInstruction::Constant(KleeneValue::True)], 0);
        let one = key(&[KleeneInstruction::Constant(KleeneValue::True)], 1);

        assert_eq!(
            kleene_designated_entails(&zero, &one),
            Err(KleeneEntailmentError::DomainMismatch {
                antecedent_input_arity: 0,
                consequent_input_arity: 1,
            })
        );
    }
}
