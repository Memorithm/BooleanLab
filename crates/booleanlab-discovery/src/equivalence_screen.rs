//! BL-13.2.1 bounded equivalence screens.
//!
//! Exact equality and output complementation can establish an equivalence match.
//! The affine-invariant screen added here is deliberately one-way: incompatible
//! exact invariants can exclude affine-input equivalence (with optional output
//! complement), while compatible invariants remain inconclusive and must never be
//! promoted to an affine-equivalence or novelty claim.

use crate::BooleanFunction;
use crate::baseline::{BaselineRecord, BaselineSummary};

/// The strongest equivalence relation actually demonstrated by the exact screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenedEquivalence {
    /// Same complete truth table.
    Exact,
    /// Equal only after `BooleanLab`'s declared output-complement canonicalisation.
    OutputComplement,
}

/// A reference-corpus match with the exact index that established it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EquivalenceMatch {
    pub reference_index: usize,
    pub relation: ScreenedEquivalence,
}

/// Exact invariants preserved by an invertible affine transformation of the input.
///
/// `canonical_weight` also tolerates an optional global output complement. The
/// sorted absolute Walsh spectrum discards the coefficient permutation/sign
/// changes induced by affine input changes and translations. Equality of this
/// signature is necessary, but not sufficient, for the declared relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AffineInvariantSignature {
    pub input_bits: u32,
    pub algebraic_degree: u32,
    pub canonical_weight: usize,
    pub walsh_abs_spectrum: Vec<u64>,
}

/// Outcome of the necessary-condition affine-invariant screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AffineInvariantScreen {
    /// No reference has the same necessary affine invariants.
    ///
    /// This excludes affine-input equivalence, with optional output complement,
    /// only against the supplied bounded reference corpus.
    Excluded,
    /// One or more references share all screened invariants.
    ///
    /// This is intentionally inconclusive: the listed functions are candidates
    /// for a stronger exact affine-equivalence test, not demonstrated matches.
    Inconclusive {
        compatible_reference_indices: Vec<usize>,
    },
}

/// Screens one scalar Boolean function against a bounded reference corpus.
///
/// Exact equality is checked first. If no exact match exists, the function is
/// canonicalised only under `f ~ f XOR 1`, matching the equivalence relation
/// already declared by `BooleanLab`. Functions with different input widths are
/// incomparable here and are skipped rather than coerced.
///
/// A `None` result means only "not found under these two declared relations in
/// this supplied corpus". It is not evidence of novelty.
#[must_use]
pub fn screen_exact_or_complement(
    candidate: &BooleanFunction,
    reference: &[BooleanFunction],
) -> Option<EquivalenceMatch> {
    if let Some(reference_index) = reference.iter().position(|known| known == candidate) {
        return Some(EquivalenceMatch {
            reference_index,
            relation: ScreenedEquivalence::Exact,
        });
    }

    let canonical = candidate.canonical_under_complement();
    reference
        .iter()
        .enumerate()
        .find_map(|(reference_index, known)| {
            if known.input_bits() != candidate.input_bits() {
                return None;
            }
            (known.canonical_under_complement() == canonical).then_some(EquivalenceMatch {
                reference_index,
                relation: ScreenedEquivalence::OutputComplement,
            })
        })
}

/// Screens one candidate directly against retained Boolean-only baseline records.
///
/// The returned index addresses `reference` itself, so callers can recover the
/// matched record's exact metrics, gate count, depth and imbalance without a
/// parallel lookup table. The equivalence relation is unchanged: only exact
/// truth-table equality and the declared output-complement canonicalisation are
/// considered.
///
/// A `None` result still means only "not found in this supplied bounded baseline
/// under the declared relations". It is not evidence of novelty or of broader
/// affine, EA, CCZ, permutation, circuit or algebraic non-equivalence.
#[must_use]
pub fn screen_baseline_records(
    candidate: &BooleanFunction,
    reference: &[BaselineRecord],
) -> Option<EquivalenceMatch> {
    if let Some(reference_index) = reference
        .iter()
        .position(|record| &record.function == candidate)
    {
        return Some(EquivalenceMatch {
            reference_index,
            relation: ScreenedEquivalence::Exact,
        });
    }

    let canonical = candidate.canonical_under_complement();
    reference
        .iter()
        .enumerate()
        .find_map(|(reference_index, record)| {
            let known = &record.function;
            if known.input_bits() != candidate.input_bits() {
                return None;
            }
            (known.canonical_under_complement() == canonical).then_some(EquivalenceMatch {
                reference_index,
                relation: ScreenedEquivalence::OutputComplement,
            })
        })
}

/// Screens a candidate against the complete exact-deduplicated population retained
/// by a Boolean-only baseline campaign.
///
/// This is the BL-13.2.1 bridge between the frozen BL-13.1.2 search result and the
/// equivalence screen. It deliberately uses [`BaselineSummary::population`], not
/// the Pareto projection, so a candidate cannot appear absent merely because an
/// equivalent reference function was dominated on cost/metric objectives.
///
/// A `None` result remains bounded to the supplied baseline population and the two
/// declared relations. It is not a novelty verdict.
#[must_use]
pub fn screen_full_baseline(
    candidate: &BooleanFunction,
    baseline: &BaselineSummary,
) -> Option<EquivalenceMatch> {
    screen_baseline_records(candidate, &baseline.population)
}

/// Screens a candidate set without collapsing individual provenance.
///
/// The output order exactly matches `candidates`; each entry is independently
/// `Some(match)` or `None`.
#[must_use]
pub fn screen_candidate_set(
    candidates: &[BooleanFunction],
    reference: &[BooleanFunction],
) -> Vec<Option<EquivalenceMatch>> {
    candidates
        .iter()
        .map(|candidate| screen_exact_or_complement(candidate, reference))
        .collect()
}

/// Computes the exact necessary-condition signature used by the affine screen.
///
/// The relation screened is `g(x) = f(Ax + b) XOR c`, where `A` is invertible,
/// `b` is an input translation and `c` is an optional global output complement.
/// Algebraic degree, canonical Hamming weight and the multiset of absolute Walsh
/// coefficients are invariant under that relation.
#[must_use]
pub fn affine_invariant_signature(function: &BooleanFunction) -> AffineInvariantSignature {
    let table = function.truth_table();
    let rows = table.len();
    let weight = table.iter().map(|&value| usize::from(value)).sum::<usize>();
    let canonical_weight = weight.min(rows - weight);

    let mut walsh = table
        .iter()
        .map(|&value| if value == 0 { 1_i64 } else { -1_i64 })
        .collect::<Vec<_>>();
    let mut stride = 1;
    while stride < rows {
        let step = stride * 2;
        for base in (0..rows).step_by(step) {
            for offset in 0..stride {
                let left = walsh[base + offset];
                let right = walsh[base + offset + stride];
                walsh[base + offset] = left + right;
                walsh[base + offset + stride] = left - right;
            }
        }
        stride = step;
    }
    let mut walsh_abs_spectrum = walsh.into_iter().map(i64::unsigned_abs).collect::<Vec<_>>();
    walsh_abs_spectrum.sort_unstable();

    AffineInvariantSignature {
        input_bits: function.input_bits(),
        algebraic_degree: function.exact_metrics().algebraic_degree,
        canonical_weight,
        walsh_abs_spectrum,
    }
}

/// Applies a necessary-condition screen for affine-input equivalence.
///
/// `Excluded` is a valid bounded negative result: no function in `reference` can
/// be related to `candidate` by an invertible affine input transformation plus an
/// optional global output complement because at least one exact invariant differs.
/// `Inconclusive` is not an equivalence match. It only identifies references that
/// survive this filter and require a stronger exact test.
#[must_use]
pub fn screen_affine_invariants(
    candidate: &BooleanFunction,
    reference: &[BooleanFunction],
) -> AffineInvariantScreen {
    let candidate_signature = affine_invariant_signature(candidate);
    let compatible_reference_indices = reference
        .iter()
        .enumerate()
        .filter_map(|(index, known)| {
            (affine_invariant_signature(known) == candidate_signature).then_some(index)
        })
        .collect::<Vec<_>>();

    if compatible_reference_indices.is_empty() {
        AffineInvariantScreen::Excluded
    } else {
        AffineInvariantScreen::Inconclusive {
            compatible_reference_indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::{BaselineConfig, run_boolean_baseline};

    fn function(bits: &[u8]) -> BooleanFunction {
        BooleanFunction::new(2, bits.to_vec()).unwrap()
    }

    fn baseline_record(function: BooleanFunction, gate_count: usize) -> BaselineRecord {
        BaselineRecord {
            metrics: function.exact_metrics(),
            function,
            gate_count,
            depth: gate_count,
            imbalance: 0,
        }
    }

    #[test]
    fn exact_match_has_precedence() {
        let candidate = function(&[0, 1, 1, 0]);
        let reference = vec![function(&[1, 0, 0, 1]), candidate.clone()];
        assert_eq!(
            screen_exact_or_complement(&candidate, &reference),
            Some(EquivalenceMatch {
                reference_index: 1,
                relation: ScreenedEquivalence::Exact,
            })
        );
    }

    #[test]
    fn complement_match_is_reported_but_not_promoted_to_exact() {
        let candidate = function(&[0, 0, 1, 1]);
        let reference = vec![function(&[1, 1, 0, 0])];
        assert_eq!(
            screen_exact_or_complement(&candidate, &reference),
            Some(EquivalenceMatch {
                reference_index: 0,
                relation: ScreenedEquivalence::OutputComplement,
            })
        );
    }

    #[test]
    fn baseline_record_screen_preserves_cost_lookup_index() {
        let candidate = function(&[0, 1, 1, 0]);
        let reference = vec![
            baseline_record(function(&[0, 0, 0, 1]), 3),
            baseline_record(candidate.clone(), 7),
        ];
        let matched = screen_baseline_records(&candidate, &reference).unwrap();

        assert_eq!(matched.reference_index, 1);
        assert_eq!(matched.relation, ScreenedEquivalence::Exact);
        assert_eq!(reference[matched.reference_index].gate_count, 7);
    }

    #[test]
    fn baseline_record_screen_keeps_declared_relation_narrow() {
        let candidate = function(&[0, 0, 1, 1]);
        let reference = vec![baseline_record(function(&[1, 1, 0, 0]), 4)];

        assert_eq!(
            screen_baseline_records(&candidate, &reference),
            Some(EquivalenceMatch {
                reference_index: 0,
                relation: ScreenedEquivalence::OutputComplement,
            })
        );
    }

    #[test]
    fn full_baseline_screen_uses_non_pareto_population_records() {
        let summary = run_boolean_baseline(BaselineConfig {
            input_bits: 2,
            candidates: 64,
            min_gates: 2,
            max_gates: 6,
            seed: 11,
        })
        .unwrap();
        let non_pareto = summary
            .population
            .iter()
            .enumerate()
            .find(|(_, record)| !summary.pareto_front.contains(record))
            .map(|(index, record)| (index, record.function.clone()))
            .expect("bounded fixture should retain at least one dominated function");

        assert_eq!(
            screen_full_baseline(&non_pareto.1, &summary),
            Some(EquivalenceMatch {
                reference_index: non_pareto.0,
                relation: ScreenedEquivalence::Exact,
            })
        );
    }

    #[test]
    fn absence_is_not_fabricated_into_an_equivalence() {
        let candidate = function(&[0, 0, 0, 1]);
        let reference = vec![function(&[0, 1, 1, 0])];
        assert_eq!(screen_exact_or_complement(&candidate, &reference), None);
    }

    #[test]
    fn differing_widths_are_not_compared() {
        let candidate = function(&[0, 0, 0, 1]);
        let wider = BooleanFunction::new(3, vec![0, 0, 0, 0, 0, 0, 0, 1]).unwrap();
        assert_eq!(screen_exact_or_complement(&candidate, &[wider]), None);
    }

    #[test]
    fn candidate_set_preserves_input_order() {
        let exact = function(&[0, 1, 1, 0]);
        let absent = function(&[0, 0, 0, 1]);
        let reference = vec![exact.clone()];
        let screened = screen_candidate_set(&[absent, exact], &reference);
        assert_eq!(screened[0], None);
        assert_eq!(
            screened[1],
            Some(EquivalenceMatch {
                reference_index: 0,
                relation: ScreenedEquivalence::Exact,
            })
        );
    }

    #[test]
    fn affine_signature_is_output_complement_invariant() {
        let original = function(&[0, 0, 0, 1]);
        let complement = function(&[1, 1, 1, 0]);
        assert_eq!(
            affine_invariant_signature(&original),
            affine_invariant_signature(&complement)
        );
    }

    #[test]
    fn affine_signature_survives_input_permutation() {
        let left = BooleanFunction::from_fn(3, |x| {
            let x0 = x & 1;
            let x1 = (x >> 1) & 1;
            let x2 = (x >> 2) & 1;
            (x0 & x1) ^ x2 == 1
        })
        .unwrap();
        let permuted = BooleanFunction::from_fn(3, |x| {
            let x0 = x & 1;
            let x1 = (x >> 1) & 1;
            let x2 = (x >> 2) & 1;
            (x2 & x0) ^ x1 == 1
        })
        .unwrap();

        assert_eq!(
            affine_invariant_signature(&left),
            affine_invariant_signature(&permuted)
        );
    }

    #[test]
    fn differing_affine_invariants_exclude_equivalence() {
        let affine = function(&[0, 1, 1, 0]);
        let nonlinear = function(&[0, 0, 0, 1]);
        assert_eq!(
            screen_affine_invariants(&nonlinear, &[affine]),
            AffineInvariantScreen::Excluded
        );
    }

    #[test]
    fn compatible_affine_invariants_remain_inconclusive() {
        let x0 = function(&[0, 1, 0, 1]);
        let x1 = function(&[0, 0, 1, 1]);
        assert_eq!(
            screen_affine_invariants(&x0, &[x1]),
            AffineInvariantScreen::Inconclusive {
                compatible_reference_indices: vec![0],
            }
        );
    }
}
