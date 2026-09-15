//! BL-14.5 bounded Boolean sparsity-rule synthesis.
//!
//! This module provides a deliberately small, exact baseline for discrete rule
//! synthesis. It searches conjunctions of declared Boolean predicates and
//! returns the least-complex exact rule under a deterministic tie-break. The
//! routine operates only on the rows supplied by the caller: it does not open a
//! holdout, infer predicate semantics, measure task quality, or claim a runtime
//! benefit.

use core::fmt;

/// Maximum predicate width accepted by the exhaustive BL-14.5 baseline.
pub const MAX_SYNTHESIS_PREDICATES: usize = 12;

/// One labelled row supplied to the bounded rule synthesizer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SynthesisRow {
    predicates: Vec<bool>,
    active: bool,
}

impl SynthesisRow {
    /// Construct one labelled predicate row.
    #[must_use]
    pub fn new(predicates: Vec<bool>, active: bool) -> Self {
        Self { predicates, active }
    }

    /// Predicate values in declared input order.
    #[must_use]
    pub fn predicates(&self) -> &[bool] {
        &self.predicates
    }

    /// Expected rule output for this row.
    #[must_use]
    pub const fn active(&self) -> bool {
        self.active
    }
}

/// One literal in a synthesized conjunction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SparsityLiteral {
    predicate_index: usize,
    required_value: bool,
}

impl SparsityLiteral {
    /// Predicate index used by this literal.
    #[must_use]
    pub const fn predicate_index(self) -> usize {
        self.predicate_index
    }

    /// Boolean value required by this literal.
    #[must_use]
    pub const fn required_value(self) -> bool {
        self.required_value
    }
}

/// Exact conjunction synthesized from a bounded labelled table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConjunctiveSparsityRule {
    predicate_count: usize,
    literals: Vec<SparsityLiteral>,
}

impl ConjunctiveSparsityRule {
    /// Number of predicates expected by [`Self::evaluate`].
    #[must_use]
    pub const fn predicate_count(&self) -> usize {
        self.predicate_count
    }

    /// Literals in increasing predicate-index order.
    #[must_use]
    pub fn literals(&self) -> &[SparsityLiteral] {
        &self.literals
    }

    /// Evaluate the conjunction on one predicate vector.
    ///
    /// The empty conjunction evaluates to `true`.
    ///
    /// # Errors
    ///
    /// Returns [`RuleSynthesisError::InputWidthMismatch`] when `predicates`
    /// does not have the same width as the table used during synthesis.
    pub fn evaluate(&self, predicates: &[bool]) -> Result<bool, RuleSynthesisError> {
        if predicates.len() != self.predicate_count {
            return Err(RuleSynthesisError::InputWidthMismatch {
                expected: self.predicate_count,
                actual: predicates.len(),
            });
        }

        Ok(self
            .literals
            .iter()
            .all(|literal| predicates[literal.predicate_index] == literal.required_value))
    }
}

/// Fail-closed errors for BL-14.5 bounded rule synthesis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleSynthesisError {
    /// No labelled rows were supplied.
    EmptyRows,
    /// The declared predicate width exceeds the exhaustive-search bound.
    TooManyPredicates { actual: usize, maximum: usize },
    /// A labelled row does not match the first row's predicate width.
    RowWidthMismatch {
        row: usize,
        expected: usize,
        actual: usize,
    },
    /// The caller requested more literals than available predicates.
    LiteralBudgetOutOfRange { budget: usize, predicates: usize },
    /// A synthesized rule was evaluated with the wrong predicate width.
    InputWidthMismatch { expected: usize, actual: usize },
}

impl fmt::Display for RuleSynthesisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyRows => write!(f, "BL-14.5 synthesis requires at least one labelled row"),
            Self::TooManyPredicates { actual, maximum } => write!(
                f,
                "BL-14.5 synthesis predicate width {actual} exceeds exhaustive bound {maximum}"
            ),
            Self::RowWidthMismatch {
                row,
                expected,
                actual,
            } => write!(
                f,
                "BL-14.5 row {row} has predicate width {actual}, expected {expected}"
            ),
            Self::LiteralBudgetOutOfRange { budget, predicates } => write!(
                f,
                "BL-14.5 literal budget {budget} exceeds predicate width {predicates}"
            ),
            Self::InputWidthMismatch { expected, actual } => write!(
                f,
                "BL-14.5 rule expects predicate width {expected}, received {actual}"
            ),
        }
    }
}

impl std::error::Error for RuleSynthesisError {}

fn ternary_search_space(width: usize) -> usize {
    let mut size = 1usize;
    for _ in 0..width {
        size *= 3;
    }
    size
}

fn decode_rule(mut code: usize, predicate_count: usize) -> Vec<SparsityLiteral> {
    let mut literals = Vec::new();
    for predicate_index in 0..predicate_count {
        match code % 3 {
            0 => {}
            1 => literals.push(SparsityLiteral {
                predicate_index,
                required_value: false,
            }),
            2 => literals.push(SparsityLiteral {
                predicate_index,
                required_value: true,
            }),
            _ => unreachable!("remainder modulo three is always in 0..=2"),
        }
        code /= 3;
    }
    literals
}

fn matches_rows(literals: &[SparsityLiteral], rows: &[SynthesisRow]) -> bool {
    rows.iter().all(|row| {
        let predicted = literals
            .iter()
            .all(|literal| row.predicates[literal.predicate_index] == literal.required_value);
        predicted == row.active
    })
}

/// Synthesize the least-complex exact conjunctive rule within a literal budget.
///
/// Each predicate has three search states: absent, require `false`, or require
/// `true`. The full bounded space is enumerated exactly. Candidate complexity is
/// the number of retained literals. Ties are resolved by the first ternary code
/// encountered, which is deterministic and independent of hash/map ordering.
///
/// `Ok(None)` is a valid negative result: no conjunction within `max_literals`
/// reproduces every supplied label exactly. The caller should retain such a
/// result rather than weakening the labels or opening a protected holdout.
///
/// # Errors
///
/// Returns an error for an empty table, inconsistent row widths, predicate
/// width above [`MAX_SYNTHESIS_PREDICATES`], or a literal budget greater than
/// the predicate width.
pub fn synthesize_exact_conjunction(
    rows: &[SynthesisRow],
    max_literals: usize,
) -> Result<Option<ConjunctiveSparsityRule>, RuleSynthesisError> {
    let Some(first) = rows.first() else {
        return Err(RuleSynthesisError::EmptyRows);
    };
    let predicate_count = first.predicates.len();
    if predicate_count > MAX_SYNTHESIS_PREDICATES {
        return Err(RuleSynthesisError::TooManyPredicates {
            actual: predicate_count,
            maximum: MAX_SYNTHESIS_PREDICATES,
        });
    }
    if max_literals > predicate_count {
        return Err(RuleSynthesisError::LiteralBudgetOutOfRange {
            budget: max_literals,
            predicates: predicate_count,
        });
    }
    for (row_index, row) in rows.iter().enumerate().skip(1) {
        if row.predicates.len() != predicate_count {
            return Err(RuleSynthesisError::RowWidthMismatch {
                row: row_index,
                expected: predicate_count,
                actual: row.predicates.len(),
            });
        }
    }

    let mut best: Option<Vec<SparsityLiteral>> = None;
    for code in 0..ternary_search_space(predicate_count) {
        let literals = decode_rule(code, predicate_count);
        if literals.len() > max_literals
            || best
                .as_ref()
                .is_some_and(|current| literals.len() >= current.len())
        {
            continue;
        }
        if matches_rows(&literals, rows) {
            best = Some(literals);
        }
    }

    Ok(best.map(|literals| ConjunctiveSparsityRule {
        predicate_count,
        literals,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exhaustive_two_input_rows(label: fn(bool, bool) -> bool) -> Vec<SynthesisRow> {
        [false, true]
            .into_iter()
            .flat_map(|left| {
                [false, true].into_iter().map(move |right| {
                    SynthesisRow::new(vec![left, right], label(left, right))
                })
            })
            .collect()
    }

    #[test]
    fn synthesizes_positive_literal_exactly() {
        let rows = exhaustive_two_input_rows(|left, _| left);
        let rule = synthesize_exact_conjunction(&rows, 2).unwrap().unwrap();
        assert_eq!(rule.literals().len(), 1);
        assert_eq!(rule.literals()[0].predicate_index(), 0);
        assert!(rule.literals()[0].required_value());
        for row in &rows {
            assert_eq!(rule.evaluate(row.predicates()).unwrap(), row.active());
        }
    }

    #[test]
    fn synthesizes_negative_literal_exactly() {
        let rows = exhaustive_two_input_rows(|left, _| !left);
        let rule = synthesize_exact_conjunction(&rows, 2).unwrap().unwrap();
        assert_eq!(rule.literals().len(), 1);
        assert_eq!(rule.literals()[0].predicate_index(), 0);
        assert!(!rule.literals()[0].required_value());
    }

    #[test]
    fn minimizes_literal_count_before_tie_breaking() {
        let rows = exhaustive_two_input_rows(|left, right| left && right);
        let rule = synthesize_exact_conjunction(&rows, 2).unwrap().unwrap();
        assert_eq!(rule.literals().len(), 2);
        assert!(rule.evaluate(&[true, true]).unwrap());
        assert!(!rule.evaluate(&[true, false]).unwrap());
    }

    #[test]
    fn deterministic_tie_prefers_lowest_ternary_code() {
        let rows = vec![
            SynthesisRow::new(vec![false, false], false),
            SynthesisRow::new(vec![true, true], true),
        ];
        let rule = synthesize_exact_conjunction(&rows, 1).unwrap().unwrap();
        assert_eq!(rule.literals().len(), 1);
        assert_eq!(rule.literals()[0].predicate_index(), 0);
        assert!(rule.literals()[0].required_value());
    }

    #[test]
    fn reports_no_exact_conjunction_for_xor() {
        let rows = exhaustive_two_input_rows(|left, right| left ^ right);
        assert_eq!(synthesize_exact_conjunction(&rows, 2).unwrap(), None);
    }

    #[test]
    fn zero_literal_budget_can_fit_only_constant_true() {
        let true_rows = vec![
            SynthesisRow::new(vec![false], true),
            SynthesisRow::new(vec![true], true),
        ];
        let rule = synthesize_exact_conjunction(&true_rows, 0)
            .unwrap()
            .unwrap();
        assert!(rule.literals().is_empty());

        let mixed_rows = vec![
            SynthesisRow::new(vec![false], false),
            SynthesisRow::new(vec![true], true),
        ];
        assert_eq!(synthesize_exact_conjunction(&mixed_rows, 0).unwrap(), None);
    }

    #[test]
    fn rejects_malformed_inputs_and_wrong_evaluation_width() {
        assert_eq!(
            synthesize_exact_conjunction(&[], 0),
            Err(RuleSynthesisError::EmptyRows)
        );

        let mismatched = vec![
            SynthesisRow::new(vec![false, true], false),
            SynthesisRow::new(vec![true], true),
        ];
        assert_eq!(
            synthesize_exact_conjunction(&mismatched, 1),
            Err(RuleSynthesisError::RowWidthMismatch {
                row: 1,
                expected: 2,
                actual: 1,
            })
        );

        let rows = exhaustive_two_input_rows(|left, _| left);
        assert_eq!(
            synthesize_exact_conjunction(&rows, 3),
            Err(RuleSynthesisError::LiteralBudgetOutOfRange {
                budget: 3,
                predicates: 2,
            })
        );
        let rule = synthesize_exact_conjunction(&rows, 1).unwrap().unwrap();
        assert_eq!(
            rule.evaluate(&[true]),
            Err(RuleSynthesisError::InputWidthMismatch {
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn enforces_exhaustive_width_bound() {
        let row = SynthesisRow::new(vec![false; MAX_SYNTHESIS_PREDICATES + 1], true);
        assert_eq!(
            synthesize_exact_conjunction(&[row], 0),
            Err(RuleSynthesisError::TooManyPredicates {
                actual: MAX_SYNTHESIS_PREDICATES + 1,
                maximum: MAX_SYNTHESIS_PREDICATES,
            })
        );
    }
}
