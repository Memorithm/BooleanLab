//! Exact three-valued Strong-Kleene reference semantics.
//!
//! This module is an experimental/differential oracle for BooleanLab. It is
//! intentionally independent from any production runtime so downstream
//! projects can compare their own implementations against small, exhaustive
//! truth tables without linking BooleanLab into an actuation path.

/// One value in Strong-Kleene three-valued logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KleeneValue {
    False,
    Unknown,
    True,
}

impl KleeneValue {
    /// Logical negation.
    #[must_use]
    pub const fn not(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::True => Self::False,
        }
    }

    /// Strong-Kleene conjunction.
    #[must_use]
    pub const fn and(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }

    /// Strong-Kleene disjunction.
    #[must_use]
    pub const fn or(self, rhs: Self) -> Self {
        match (self, rhs) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }

    /// Exclusive-or derived compositionally from the Strong-Kleene core.
    ///
    /// `xor(a, b) = (a OR b) AND NOT(a AND b)`.
    #[must_use]
    pub const fn xor(self, rhs: Self) -> Self {
        self.or(rhs).and(self.and(rhs).not())
    }

    /// Material implication derived compositionally from the Strong-Kleene
    /// core: `a -> b = NOT(a) OR b`.
    #[must_use]
    pub const fn implies(self, rhs: Self) -> Self {
        self.not().or(rhs)
    }
}

/// Every truth value in canonical order.
pub const KLEENE_VALUES: [KleeneValue; 3] = [
    KleeneValue::False,
    KleeneValue::Unknown,
    KleeneValue::True,
];

/// One exhaustive binary truth-table row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BinaryTruthRow {
    pub lhs: KleeneValue,
    pub rhs: KleeneValue,
    pub and: KleeneValue,
    pub or: KleeneValue,
    pub xor: KleeneValue,
    pub implies: KleeneValue,
}

/// Produce the complete 3 x 3 Strong-Kleene binary oracle.
#[must_use]
pub fn exhaustive_binary_truth_table() -> [BinaryTruthRow; 9] {
    let mut rows = [BinaryTruthRow {
        lhs: KleeneValue::False,
        rhs: KleeneValue::False,
        and: KleeneValue::False,
        or: KleeneValue::False,
        xor: KleeneValue::False,
        implies: KleeneValue::True,
    }; 9];
    let mut index = 0;
    let mut lhs_index = 0;
    while lhs_index < KLEENE_VALUES.len() {
        let lhs = KLEENE_VALUES[lhs_index];
        let mut rhs_index = 0;
        while rhs_index < KLEENE_VALUES.len() {
            let rhs = KLEENE_VALUES[rhs_index];
            rows[index] = BinaryTruthRow {
                lhs,
                rhs,
                and: lhs.and(rhs),
                or: lhs.or(rhs),
                xor: lhs.xor(rhs),
                implies: lhs.implies(rhs),
            };
            index += 1;
            rhs_index += 1;
        }
        lhs_index += 1;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negation_is_exact() {
        assert_eq!(KleeneValue::False.not(), KleeneValue::True);
        assert_eq!(KleeneValue::Unknown.not(), KleeneValue::Unknown);
        assert_eq!(KleeneValue::True.not(), KleeneValue::False);
    }

    #[test]
    fn binary_table_is_exhaustive_and_unique() {
        let rows = exhaustive_binary_truth_table();
        assert_eq!(rows.len(), 9);
        for lhs in KLEENE_VALUES {
            for rhs in KLEENE_VALUES {
                assert_eq!(
                    rows.iter()
                        .filter(|row| row.lhs == lhs && row.rhs == rhs)
                        .count(),
                    1
                );
            }
        }
    }

    #[test]
    fn unknown_is_not_false() {
        let u = KleeneValue::Unknown;
        assert_eq!(u.and(KleeneValue::True), KleeneValue::Unknown);
        assert_eq!(u.or(KleeneValue::False), KleeneValue::Unknown);
        assert_eq!(u.xor(KleeneValue::False), KleeneValue::Unknown);
        assert_eq!(u.implies(KleeneValue::False), KleeneValue::Unknown);
    }

    #[test]
    fn decisive_operands_short_circuit_truth_not_evidence() {
        let u = KleeneValue::Unknown;
        assert_eq!(KleeneValue::False.and(u), KleeneValue::False);
        assert_eq!(KleeneValue::True.or(u), KleeneValue::True);
    }

    #[test]
    fn known_inputs_reduce_to_classical_boolean_logic() {
        for lhs in [KleeneValue::False, KleeneValue::True] {
            for rhs in [KleeneValue::False, KleeneValue::True] {
                let lhs_bool = lhs == KleeneValue::True;
                let rhs_bool = rhs == KleeneValue::True;
                assert_eq!(lhs.and(rhs) == KleeneValue::True, lhs_bool && rhs_bool);
                assert_eq!(lhs.or(rhs) == KleeneValue::True, lhs_bool || rhs_bool);
                assert_eq!(lhs.xor(rhs) == KleeneValue::True, lhs_bool ^ rhs_bool);
                assert_eq!(lhs.implies(rhs) == KleeneValue::True, !lhs_bool || rhs_bool);
            }
        }
    }

    #[test]
    fn conjunction_and_disjunction_are_commutative() {
        for lhs in KLEENE_VALUES {
            for rhs in KLEENE_VALUES {
                assert_eq!(lhs.and(rhs), rhs.and(lhs));
                assert_eq!(lhs.or(rhs), rhs.or(lhs));
                assert_eq!(lhs.xor(rhs), rhs.xor(lhs));
            }
        }
    }

    #[test]
    fn de_morgan_holds_exhaustively() {
        for lhs in KLEENE_VALUES {
            for rhs in KLEENE_VALUES {
                assert_eq!(lhs.and(rhs).not(), lhs.not().or(rhs.not()));
                assert_eq!(lhs.or(rhs).not(), lhs.not().and(rhs.not()));
            }
        }
    }
}
