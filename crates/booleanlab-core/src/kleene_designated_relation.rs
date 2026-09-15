//! Exact bounded relations between designated Strong-Kleene truth sets.
//!
//! This module builds on [`crate::kleene_designated_entails`] and classifies
//! how the rows designated `True` by two exact [`crate::KleeneSemanticKey`]
//! artifacts relate. The classification intentionally ignores the distinction
//! between `False` and `Unknown` on non-designated rows; it is therefore not a
//! semantic-equivalence test. Exact semantic identity remains ordinary key
//! equality.
//!
//! The result is suitable for bounded BL-BE3 subsumption experiments only. It
//! is not a production authorization rule, a rule-removal safety proof, or a
//! performance claim.

use crate::{
    KleeneEntailment, KleeneEntailmentError, KleeneEntailmentWitness, KleeneSemanticKey,
    KleeneValue, kleene_designated_entails,
};

/// First canonical row on which the original relation operands differ in
/// designated-`True` membership.
///
/// Unlike [`KleeneEntailmentWitness`], these fields are always oriented to the
/// original arguments passed to [`kleene_designated_relation`], even when the
/// witness was discovered by the reverse entailment check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KleeneDesignatedDifferenceWitness {
    /// Canonical base-3 assignment index.
    pub assignment_index: usize,
    /// Original antecedent output on the witness row.
    pub antecedent: KleeneValue,
    /// Original consequent output on the witness row.
    pub consequent: KleeneValue,
}

/// Exact relation between the rows designated `True` by two semantic keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KleeneDesignatedRelation {
    /// Both keys designate exactly the same rows as `True`.
    ///
    /// This does **not** imply full Strong-Kleene semantic equality because
    /// the keys may still differ between `False` and `Unknown` elsewhere.
    SameDesignatedTrueSet {
        /// Exhaustive domain size.
        rows: usize,
        /// Number of rows designated `True` by both keys.
        designated_true_rows: usize,
    },
    /// The antecedent designates a strict subset of the consequent's `True` rows.
    AntecedentMoreRestrictive {
        /// Exhaustive domain size.
        rows: usize,
        /// First canonical row where the consequent is `True` but the antecedent is not.
        consequent_only_witness: KleeneDesignatedDifferenceWitness,
    },
    /// The consequent designates a strict subset of the antecedent's `True` rows.
    ConsequentMoreRestrictive {
        /// Exhaustive domain size.
        rows: usize,
        /// First canonical row where the antecedent is `True` but the consequent is not.
        antecedent_only_witness: KleeneDesignatedDifferenceWitness,
    },
    /// Each key designates at least one `True` row that the other does not.
    Incomparable {
        /// Exhaustive domain size.
        rows: usize,
        /// First canonical row where the antecedent is `True` but the consequent is not.
        antecedent_only_witness: KleeneDesignatedDifferenceWitness,
        /// First canonical row where the consequent is `True` but the antecedent is not.
        consequent_only_witness: KleeneDesignatedDifferenceWitness,
    },
}

/// Classifies the exact designated-`True` set relation between two keys.
///
/// Both inputs are revalidated by [`kleene_designated_entails`]. The function
/// performs entailment in both directions and combines the exact results. A
/// structural validation error or domain mismatch is returned unchanged.
///
/// `SameDesignatedTrueSet` means only equality of designated `True` rows. Use
/// [`KleeneSemanticKey`] equality when exact three-valued semantic identity is
/// required.
///
/// # Errors
///
/// Returns [`KleeneEntailmentError`] when either key is malformed or when the
/// declared input domains differ.
pub fn kleene_designated_relation(
    antecedent: &KleeneSemanticKey,
    consequent: &KleeneSemanticKey,
) -> Result<KleeneDesignatedRelation, KleeneEntailmentError> {
    let forward = kleene_designated_entails(antecedent, consequent)?;
    let reverse = kleene_designated_entails(consequent, antecedent)?;

    match (forward, reverse) {
        (
            KleeneEntailment::Entails {
                rows,
                antecedent_true_rows,
            },
            KleeneEntailment::Entails {
                rows: reverse_rows,
                antecedent_true_rows: reverse_true_rows,
            },
        ) => {
            debug_assert_eq!(rows, reverse_rows);
            debug_assert_eq!(antecedent_true_rows, reverse_true_rows);
            Ok(KleeneDesignatedRelation::SameDesignatedTrueSet {
                rows,
                designated_true_rows: antecedent_true_rows,
            })
        }
        (
            KleeneEntailment::Entails { rows, .. },
            KleeneEntailment::DoesNotEntail {
                rows: reverse_rows,
                witness,
            },
        ) => {
            debug_assert_eq!(rows, reverse_rows);
            Ok(KleeneDesignatedRelation::AntecedentMoreRestrictive {
                rows,
                consequent_only_witness: reverse_witness(witness),
            })
        }
        (
            KleeneEntailment::DoesNotEntail { rows, witness },
            KleeneEntailment::Entails {
                rows: reverse_rows, ..
            },
        ) => {
            debug_assert_eq!(rows, reverse_rows);
            Ok(KleeneDesignatedRelation::ConsequentMoreRestrictive {
                rows,
                antecedent_only_witness: forward_witness(witness),
            })
        }
        (
            KleeneEntailment::DoesNotEntail {
                rows,
                witness: antecedent_only_witness,
            },
            KleeneEntailment::DoesNotEntail {
                rows: reverse_rows,
                witness: consequent_only_witness,
            },
        ) => {
            debug_assert_eq!(rows, reverse_rows);
            Ok(KleeneDesignatedRelation::Incomparable {
                rows,
                antecedent_only_witness: forward_witness(antecedent_only_witness),
                consequent_only_witness: reverse_witness(consequent_only_witness),
            })
        }
    }
}

fn forward_witness(witness: KleeneEntailmentWitness) -> KleeneDesignatedDifferenceWitness {
    KleeneDesignatedDifferenceWitness {
        assignment_index: witness.assignment_index,
        antecedent: witness.antecedent,
        consequent: witness.consequent,
    }
}

fn reverse_witness(witness: KleeneEntailmentWitness) -> KleeneDesignatedDifferenceWitness {
    KleeneDesignatedDifferenceWitness {
        assignment_index: witness.assignment_index,
        antecedent: witness.consequent,
        consequent: witness.antecedent,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{KleeneInstruction, kleene_semantic_key};

    fn checked_pow3(exponent: usize) -> usize {
        let mut value = 1usize;
        for _ in 0..exponent {
            value = value.checked_mul(3).expect("small test domain must fit");
        }
        value
    }

    fn key(program: &[KleeneInstruction], input_arity: usize) -> KleeneSemanticKey {
        let rows = checked_pow3(input_arity);
        let work = rows
            .checked_mul(program.len())
            .expect("small test work must fit");
        kleene_semantic_key(program, input_arity, rows, work)
            .expect("test program must produce an exact key")
    }

    #[test]
    fn same_designated_set_does_not_claim_semantic_equality() {
        let always_false = key(&[KleeneInstruction::Constant(KleeneValue::False)], 1);
        let always_unknown = key(&[KleeneInstruction::Constant(KleeneValue::Unknown)], 1);

        assert_ne!(always_false, always_unknown);
        assert_eq!(
            kleene_designated_relation(&always_false, &always_unknown),
            Ok(KleeneDesignatedRelation::SameDesignatedTrueSet {
                rows: 3,
                designated_true_rows: 0,
            })
        );
    }

    #[test]
    fn conjunction_is_more_restrictive_than_one_operand_with_oriented_witness() {
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let x_and_y = key(
            &[
                KleeneInstruction::Input(0),
                KleeneInstruction::Input(1),
                KleeneInstruction::And,
            ],
            2,
        );

        let relation = kleene_designated_relation(&x_and_y, &x)
            .expect("valid keys on one domain must compare");
        match relation {
            KleeneDesignatedRelation::AntecedentMoreRestrictive {
                rows: 9,
                consequent_only_witness,
            } => {
                assert_ne!(consequent_only_witness.antecedent, KleeneValue::True);
                assert_eq!(consequent_only_witness.consequent, KleeneValue::True);
            }
            other => panic!("unexpected designated relation: {other:?}"),
        }
    }

    #[test]
    fn reverse_direction_reports_consequent_more_restrictive() {
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let x_and_y = key(
            &[
                KleeneInstruction::Input(0),
                KleeneInstruction::Input(1),
                KleeneInstruction::And,
            ],
            2,
        );

        let relation = kleene_designated_relation(&x, &x_and_y)
            .expect("valid keys on one domain must compare");
        match relation {
            KleeneDesignatedRelation::ConsequentMoreRestrictive {
                rows: 9,
                antecedent_only_witness,
            } => {
                assert_eq!(antecedent_only_witness.antecedent, KleeneValue::True);
                assert_ne!(antecedent_only_witness.consequent, KleeneValue::True);
            }
            other => panic!("unexpected designated relation: {other:?}"),
        }
    }

    #[test]
    fn independent_inputs_are_incomparable_with_both_witnesses_oriented() {
        let x = key(&[KleeneInstruction::Input(0)], 2);
        let y = key(&[KleeneInstruction::Input(1)], 2);

        let relation =
            kleene_designated_relation(&x, &y).expect("valid keys on one domain must compare");
        match relation {
            KleeneDesignatedRelation::Incomparable {
                rows: 9,
                antecedent_only_witness,
                consequent_only_witness,
            } => {
                assert_eq!(antecedent_only_witness.antecedent, KleeneValue::True);
                assert_ne!(antecedent_only_witness.consequent, KleeneValue::True);
                assert_ne!(consequent_only_witness.antecedent, KleeneValue::True);
                assert_eq!(consequent_only_witness.consequent, KleeneValue::True);
            }
            other => panic!("unexpected designated relation: {other:?}"),
        }
    }

    #[test]
    fn malformed_or_mismatched_keys_propagate_exact_errors() {
        let zero = key(&[KleeneInstruction::Constant(KleeneValue::True)], 0);
        let one = key(&[KleeneInstruction::Constant(KleeneValue::True)], 1);

        assert_eq!(
            kleene_designated_relation(&zero, &one),
            Err(KleeneEntailmentError::DomainMismatch {
                antecedent_input_arity: 0,
                consequent_input_arity: 1,
            })
        );
    }
}
