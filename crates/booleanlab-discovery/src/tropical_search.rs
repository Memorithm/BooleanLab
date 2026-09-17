//! Preregistered exhaustive BL-13.4.1 search over a frozen four-input
//! max-plus grammar.
//!
//! This module implements only the bounded Development search frozen in
//! `experiments/BL-13.4.1-TROPICAL-BOUNDED-SEARCH.md`.  It is not a novelty,
//! equivalence, matched-cost, hardware, or performance screen.

use std::collections::{BTreeMap, BTreeSet};

use scirust_modalg::boolean::{mobius_transform, walsh_hadamard};

use crate::tropical::{
    TropicalComparison, TropicalError, TropicalMonomial, TropicalPolynomial,
    bl13_4_control_predicates,
};
use crate::{BooleanFunction, ExactMetrics};

pub const BL13_4_1_INPUT_BITS: u32 = 4;
pub const BL13_4_1_MAX_REPORTED_WITNESSES: usize = 16;
const ROWS: usize = 1 << BL13_4_1_INPUT_BITS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TropicalSearchSource {
    pub left_polynomial: usize,
    pub right_polynomial: usize,
    pub comparison: TropicalComparison,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TropicalSearchWitness {
    pub source: TropicalSearchSource,
    pub truth_table: Vec<u8>,
    pub fingerprint: u64,
    pub anf_coefficients: Vec<u8>,
    pub walsh_spectrum: Vec<i64>,
    pub metrics: ExactMetrics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TropicalSearchSummary {
    pub syntactic_terms: usize,
    pub polynomials: usize,
    pub predicates_evaluated: usize,
    pub unique_truth_tables: usize,
    pub duplicate_predicates: usize,
    pub constant_functions: usize,
    pub nonconstant_functions: usize,
    pub balanced_functions: usize,
    pub max_nonlinearity: u64,
    pub max_algebraic_degree: u32,
    pub h1_witness_count: usize,
    pub control_present: [bool; 4],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TropicalSearchReport {
    pub summary: TropicalSearchSummary,
    pub witnesses: Vec<TropicalSearchWitness>,
}

#[derive(Clone)]
struct EvaluatedPolynomial {
    values: [i64; ROWS],
}

fn frozen_terms() -> Vec<TropicalMonomial> {
    let masks = (0_u64..(1_u64 << BL13_4_1_INPUT_BITS))
        .filter(|mask| mask.count_ones() <= 2)
        .collect::<Vec<_>>();
    let mut terms = Vec::with_capacity(33);
    for constant in [-1_i64, 0, 1] {
        for &variables in &masks {
            terms.push(TropicalMonomial {
                constant,
                variables,
            });
        }
    }
    terms
}

fn frozen_polynomials(
    terms: &[TropicalMonomial],
) -> Result<Vec<EvaluatedPolynomial>, TropicalError> {
    let mut polynomials = Vec::new();
    for left in 0..terms.len() {
        for right in (left + 1)..terms.len() {
            let polynomial =
                TropicalPolynomial::new(BL13_4_1_INPUT_BITS, vec![terms[left], terms[right]])?;
            let mut values = [0_i64; ROWS];
            for (input, value) in values.iter_mut().enumerate() {
                *value = polynomial.evaluate(input as u64)?;
            }
            polynomials.push(EvaluatedPolynomial { values });
        }
    }
    Ok(polynomials)
}

fn table_for_pair(
    left: &EvaluatedPolynomial,
    right: &EvaluatedPolynomial,
    comparison: TropicalComparison,
) -> Vec<u8> {
    left.values
        .iter()
        .zip(&right.values)
        .map(|(&lhs, &rhs)| {
            u8::from(match comparison {
                TropicalComparison::Greater => lhs > rhs,
                TropicalComparison::GreaterOrEqual => lhs >= rhs,
            })
        })
        .collect()
}

fn exact_witness(
    truth_table: Vec<u8>,
    source: TropicalSearchSource,
) -> Result<TropicalSearchWitness, TropicalError> {
    let function = BooleanFunction::new(BL13_4_1_INPUT_BITS, truth_table.clone())
        .map_err(TropicalError::Function)?;
    let metrics = function.exact_metrics();
    let mut anf_coefficients = truth_table.clone();
    mobius_transform(&mut anf_coefficients, BL13_4_1_INPUT_BITS);
    let walsh_spectrum = walsh_hadamard(&truth_table, BL13_4_1_INPUT_BITS);
    Ok(TropicalSearchWitness {
        source,
        truth_table,
        fingerprint: function.stable_fingerprint(),
        anf_coefficients,
        walsh_spectrum,
        metrics,
    })
}

/// Execute the exact preregistered BL-13.4.1 Development search.
///
/// The search enumerates all frozen syntax before inspecting Boolean metrics,
/// deduplicates by exact truth-table bytes, excludes the four BL-13.4.0 control
/// truth tables from the H1 gate, and retains at most 16 deterministic witness
/// records for reporting.
///
/// # Errors
///
/// Returns [`TropicalError`] if the frozen polynomial grammar, exact Boolean
/// materialization, or the retained BL-13.4.0 control family violates its
/// bounded fail-closed contract.
pub fn bl13_4_1_bounded_search() -> Result<TropicalSearchReport, TropicalError> {
    let terms = frozen_terms();
    let polynomials = frozen_polynomials(&terms)?;
    let comparisons = [
        TropicalComparison::Greater,
        TropicalComparison::GreaterOrEqual,
    ];

    let mut first_source_by_table = BTreeMap::<Vec<u8>, TropicalSearchSource>::new();
    let mut predicates_evaluated = 0_usize;
    for left in 0..polynomials.len() {
        for right in (left + 1)..polynomials.len() {
            for comparison in comparisons {
                predicates_evaluated += 1;
                let table = table_for_pair(&polynomials[left], &polynomials[right], comparison);
                first_source_by_table
                    .entry(table)
                    .or_insert(TropicalSearchSource {
                        left_polynomial: left,
                        right_polynomial: right,
                        comparison,
                    });
            }
        }
    }

    let controls = bl13_4_control_predicates()?
        .into_iter()
        .map(|predicate| {
            predicate
                .boolean_function()
                .map(|function| function.truth_table().to_vec())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let control_tables = controls.iter().cloned().collect::<BTreeSet<_>>();
    let control_present =
        std::array::from_fn(|index| first_source_by_table.contains_key(&controls[index]));

    let mut constant_functions = 0_usize;
    let mut balanced_functions = 0_usize;
    let mut max_nonlinearity = 0_u64;
    let mut max_algebraic_degree = 0_u32;
    let mut witnesses = Vec::new();

    for (truth_table, source) in &first_source_by_table {
        let function = BooleanFunction::new(BL13_4_1_INPUT_BITS, truth_table.clone())
            .map_err(TropicalError::Function)?;
        let metrics = function.exact_metrics();
        if truth_table.iter().all(|value| *value == truth_table[0]) {
            constant_functions += 1;
        }
        if metrics.balanced {
            balanced_functions += 1;
        }
        max_nonlinearity = max_nonlinearity.max(metrics.nonlinearity);
        max_algebraic_degree = max_algebraic_degree.max(metrics.algebraic_degree);

        if !control_tables.contains(truth_table) && metrics.balanced && metrics.nonlinearity >= 4 {
            witnesses.push(exact_witness(truth_table.clone(), *source)?);
        }
    }

    let h1_witness_count = witnesses.len();
    witnesses.sort_by(|left, right| {
        right
            .metrics
            .nonlinearity
            .cmp(&left.metrics.nonlinearity)
            .then_with(|| {
                right
                    .metrics
                    .correlation_immunity
                    .cmp(&left.metrics.correlation_immunity)
            })
            .then_with(|| {
                right
                    .metrics
                    .algebraic_degree
                    .cmp(&left.metrics.algebraic_degree)
            })
            .then_with(|| left.truth_table.cmp(&right.truth_table))
    });
    witnesses.truncate(BL13_4_1_MAX_REPORTED_WITNESSES);

    let unique_truth_tables = first_source_by_table.len();
    Ok(TropicalSearchReport {
        summary: TropicalSearchSummary {
            syntactic_terms: terms.len(),
            polynomials: polynomials.len(),
            predicates_evaluated,
            unique_truth_tables,
            duplicate_predicates: predicates_evaluated - unique_truth_tables,
            constant_functions,
            nonconstant_functions: unique_truth_tables - constant_functions,
            balanced_functions,
            max_nonlinearity,
            max_algebraic_degree,
            h1_witness_count,
            control_present,
        },
        witnesses,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_grammar_has_preregistered_syntactic_cardinality() {
        let terms = frozen_terms();
        assert_eq!(terms.len(), 33);
        let polynomials = frozen_polynomials(&terms).unwrap();
        assert_eq!(polynomials.len(), 528);
        let expected_predicates = polynomials.len() * (polynomials.len() - 1);
        assert_eq!(expected_predicates, 278_256);
    }

    #[test]
    fn bounded_search_is_exact_and_witnesses_obey_the_frozen_gate() {
        let report = bl13_4_1_bounded_search().unwrap();
        assert_eq!(report.summary.syntactic_terms, 33);
        assert_eq!(report.summary.polynomials, 528);
        assert_eq!(report.summary.predicates_evaluated, 278_256);
        assert_eq!(
            report.summary.unique_truth_tables + report.summary.duplicate_predicates,
            report.summary.predicates_evaluated
        );
        assert_eq!(
            report.summary.constant_functions + report.summary.nonconstant_functions,
            report.summary.unique_truth_tables
        );
        assert!(report.witnesses.len() <= BL13_4_1_MAX_REPORTED_WITNESSES);
        for witness in &report.witnesses {
            assert!(witness.metrics.balanced);
            assert!(witness.metrics.nonlinearity >= 4);
            assert_eq!(witness.truth_table.len(), ROWS);
            assert_eq!(witness.anf_coefficients.len(), ROWS);
            assert_eq!(witness.walsh_spectrum.len(), ROWS);
        }
    }
}
