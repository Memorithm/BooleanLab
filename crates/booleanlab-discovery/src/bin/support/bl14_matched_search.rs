//! BL-14.2.3 exploratory budget audit using the unchanged numerical fixture.
//! Matching unique-mask scoring does not match total time or controller cost.

use super::{
    Batch, Choice, ExactMask, MaskFamily, Metrics, Result, Split, Trial, UNITS, Work,
    deterministic_random_keys, finite, fit, generate, increment, keys, mask_code,
    mask_from_descending_u64_scores, materialize_boolean_function_mask, propose_exhaustive_rules,
    require_batch, score, structured_nm_mask_from_u64_scores, trials,
};
use std::cmp::{Ordering, Reverse};
use std::collections::{BTreeMap, BTreeSet};

const RETAINED: [usize; 3] = [2, 4, 6];

#[derive(Debug)]
struct SearchArm {
    name: &'static str,
    choices: Vec<Choice>,
    selected: Vec<usize>,
    candidate_examples: u64,
    work: Work,
}

#[derive(Debug)]
struct FrozenCell {
    trial: Trial,
    keep: usize,
    weights: [f64; UNITS],
    predicates: Vec<Vec<bool>>,
    dense_search: Metrics,
    heuristics: Vec<Choice>,
    arms: Vec<SearchArm>,
}

#[derive(Debug, PartialEq)]
struct ValidationRow {
    arm: &'static str,
    mask: u16,
    metrics: Metrics,
}

fn from_code(code: u16) -> Result<ExactMask> {
    if code >= 256 {
        return Err("mask code exceeds the eight-unit carrier".into());
    }
    let indices: Vec<usize> = (0..UNITS)
        .filter(|&bit| code & (1u16 << bit) != 0)
        .collect();
    Ok(ExactMask::from_retained_indices(UNITS, &indices)?)
}

fn all_masks(keep: usize) -> Result<Vec<ExactMask>> {
    if keep > UNITS {
        return Err("retained count exceeds units".into());
    }
    (0..256u16)
        .filter(|code| code.count_ones() == u32::try_from(keep).unwrap_or(u32::MAX))
        .map(from_code)
        .collect()
}

fn direct_family(mask: ExactMask) -> MaskFamily {
    MaskFamily {
        mask,
        codes: Vec::new(),
        functions: Vec::new(),
    }
}

// Population construction has no access to SEARCH or VALIDATION data.
fn matched_population(
    keep: usize,
    budget: usize,
    seed: u64,
    anchor: &ExactMask,
) -> Result<Vec<MaskFamily>> {
    let population = all_masks(keep)?;
    if budget == 0 || budget > population.len() {
        return Err("empty or impossible unique-topology budget".into());
    }
    if anchor.as_slice().len() != UNITS || anchor.cardinality().retained() != keep {
        return Err("invalid magnitude anchor".into());
    }
    let ranking = deterministic_random_keys(256, seed)?;
    let mut others: Vec<ExactMask> = population
        .into_iter()
        .filter(|mask| mask != anchor)
        .collect();
    others.sort_unstable_by_key(|mask| {
        let code = mask_code(mask);
        (Reverse(ranking[usize::from(code)]), code)
    });
    others.truncate(budget - 1);
    let mut result = vec![direct_family(anchor.clone())];
    result.extend(others.into_iter().map(direct_family));
    Ok(result)
}

fn boolean_population(predicates: &[Vec<bool>], keep: usize) -> Result<Vec<MaskFamily>> {
    if predicates.len() != UNITS || keep > UNITS {
        return Err("wrong predicate carrier or retained count".into());
    }
    let rows: Vec<&[bool]> = predicates.iter().map(Vec::as_slice).collect();
    let mut groups: BTreeMap<u16, MaskFamily> = BTreeMap::new();
    for rule in propose_exhaustive_rules(3)? {
        let mask = materialize_boolean_function_mask(&rule.function, &rows)?;
        if mask.cardinality().retained() == keep {
            let group = groups
                .entry(mask_code(&mask))
                .or_insert_with(|| direct_family(mask));
            group.codes.push(rule.truth_table_code);
            group.functions.push(rule.function);
        }
    }
    Ok(groups.into_values().collect())
}

fn minimum_indices(choices: &[Choice]) -> Result<Vec<usize>> {
    for choice in choices {
        finite(choice.search.task_mse)?;
    }
    let best = choices
        .iter()
        .map(|choice| choice.search.task_mse)
        .min_by(f64::total_cmp)
        .ok_or("empty search population")?;
    Ok(choices
        .iter()
        .enumerate()
        .filter_map(|(index, choice)| {
            (choice.search.task_mse.total_cmp(&best) == Ordering::Equal).then_some(index)
        })
        .collect())
}

fn search_arm(
    name: &'static str,
    weights: &[f64; UNITS],
    batch: &Batch,
    population: Vec<MaskFamily>,
    keep: usize,
) -> Result<SearchArm> {
    require_batch(batch, batch.trial, Split::Search)?;
    if population.is_empty() {
        return Err("empty search population".into());
    }
    // Validate the entire population before spending any scoring budget.
    let mut seen = BTreeSet::new();
    for family in &population {
        if family.mask.as_slice().len() != UNITS
            || family.mask.cardinality().retained() != keep
            || !seen.insert(mask_code(&family.mask))
        {
            return Err("duplicate, wrong-width or wrong-density search candidate".into());
        }
    }
    let mut work = Work::default();
    let mut candidate_examples = 0;
    let mut choices = Vec::new();
    for family in population {
        let metrics = score(weights, batch, Some(&family.mask))?;
        increment(&mut work.multiplications, metrics.work.multiplications)?;
        increment(&mut work.relus, metrics.work.relus)?;
        increment(&mut work.mask_tests, metrics.work.mask_tests)?;
        increment(&mut candidate_examples, u64::try_from(batch.samples.len())?)?;
        choices.push(Choice {
            id: format!("mask-{:02x}", mask_code(&family.mask)),
            mask: family.mask,
            codes: family.codes,
            functions: family.functions,
            search: metrics,
        });
    }
    let selected = minimum_indices(&choices)?;
    Ok(SearchArm {
        name,
        choices,
        selected,
        candidate_examples,
        work,
    })
}

fn same_metrics(left: &Metrics, right: &Metrics) -> bool {
    left.task_mse.to_bits() == right.task_mse.to_bits()
        && left.reconstruction_mse.to_bits() == right.reconstruction_mse.to_bits()
        && left.work == right.work
}

fn prepare_cell(
    trial: Trial,
    weights: [f64; UNITS],
    energy: &[f64; UNITS],
    search: &Batch,
    keep: usize,
) -> Result<FrozenCell> {
    require_batch(search, trial, Split::Search)?;
    if !RETAINED.contains(&keep) {
        return Err("undeclared retained-unit budget".into());
    }
    let magnitude_keys = keys(&weights)?;
    let magnitude = mask_from_descending_u64_scores(&magnitude_keys, keep)?;
    let energetic = mask_from_descending_u64_scores(&keys(energy)?, keep)?;
    let predicates: Vec<Vec<bool>> = (0..UNITS)
        .map(|index| {
            vec![
                magnitude.as_slice()[index],
                weights[index] < 0.0,
                energetic.as_slice()[index],
            ]
        })
        .collect();
    let population = boolean_population(&predicates, keep)?;
    let budget = population.len();
    let direct_same = population
        .iter()
        .map(|family| Ok(direct_family(from_code(mask_code(&family.mask))?)))
        .collect::<Result<Vec<_>>>()?;
    let direct_matched = matched_population(keep, budget, 0xB114_0023 + trial.seed, &magnitude)?;
    let complete = all_masks(keep)?.into_iter().map(direct_family).collect();
    let arms = vec![
        search_arm("boolean", &weights, search, population, keep)?,
        search_arm("direct_same", &weights, search, direct_same, keep)?,
        search_arm("direct_matched", &weights, search, direct_matched, keep)?,
        search_arm("direct_all_larger_budget", &weights, search, complete, keep)?,
    ];
    for arm in &arms[1..3] {
        if arm.choices.len() != budget
            || arm.candidate_examples != arms[0].candidate_examples
            || arm.work != arms[0].work
        {
            return Err("matched scoring-budget drift".into());
        }
    }
    if arms[0].selected != arms[1].selected {
        return Err("direct encoding changed selected optimum identities".into());
    }
    for (left, right) in arms[0].choices.iter().zip(&arms[1].choices) {
        if left.mask != right.mask || !same_metrics(&left.search, &right.search) {
            return Err("direct encoding changed candidate semantics".into());
        }
    }
    let optimum = arms[3].choices[arms[3].selected[0]].search.task_mse;
    for arm in &arms[..3] {
        if optimum > arm.choices[arm.selected[0]].search.task_mse {
            return Err("complete-population SEARCH oracle is inconsistent".into());
        }
    }
    let mut saliency = [0.0; UNITS];
    for (index, value) in saliency.iter_mut().enumerate() {
        *value = finite(weights[index] * weights[index] * energy[index])?;
    }
    let controls = vec![
        ("magnitude", magnitude),
        (
            "unit_nm",
            structured_nm_mask_from_u64_scores(&magnitude_keys, keep / 2, 4)?,
        ),
        (
            "activation_energy",
            mask_from_descending_u64_scores(&keys(&saliency)?, keep)?,
        ),
    ];
    let mut heuristics = Vec::new();
    for (name, mask) in controls {
        heuristics.push(Choice {
            id: name.to_owned(),
            search: score(&weights, search, Some(&mask))?,
            mask,
            codes: Vec::new(),
            functions: Vec::new(),
        });
    }
    Ok(FrozenCell {
        trial,
        keep,
        weights,
        predicates,
        dense_search: score(&weights, search, None)?,
        heuristics,
        arms,
    })
}

impl FrozenCell {
    fn validate(&self, batch: &Batch) -> Result<Vec<ValidationRow>> {
        require_batch(batch, self.trial, Split::Validation)?;
        let predicates: Vec<&[bool]> = self.predicates.iter().map(Vec::as_slice).collect();
        let mut rows = vec![ValidationRow {
            arm: "dense",
            mask: 255,
            metrics: score(&self.weights, batch, None)?,
        }];
        for choice in &self.heuristics {
            let arm = match choice.id.as_str() {
                "magnitude" => "magnitude",
                "unit_nm" => "unit_nm",
                "activation_energy" => "activation_energy",
                _ => return Err("unknown frozen heuristic".into()),
            };
            rows.push(ValidationRow {
                arm,
                mask: mask_code(&choice.mask),
                metrics: score(&self.weights, batch, Some(&choice.mask))?,
            });
        }
        for arm in &self.arms {
            for &index in &arm.selected {
                let choice = &arm.choices[index];
                if choice.mask.cardinality().retained() != self.keep {
                    return Err("frozen density changed".into());
                }
                for function in &choice.functions {
                    if materialize_boolean_function_mask(function, &predicates)? != choice.mask {
                        return Err("frozen predicate/function/mask binding changed".into());
                    }
                }
                rows.push(ValidationRow {
                    arm: arm.name,
                    mask: mask_code(&choice.mask),
                    metrics: score(&self.weights, batch, Some(&choice.mask))?,
                });
            }
        }
        let boolean: Vec<_> = rows.iter().filter(|row| row.arm == "boolean").collect();
        let direct: Vec<_> = rows.iter().filter(|row| row.arm == "direct_same").collect();
        if boolean.len() != direct.len()
            || boolean.iter().zip(direct).any(|(left, right)| {
                left.mask != right.mask || !same_metrics(&left.metrics, &right.metrics)
            })
        {
            return Err("direct representation failed validation parity".into());
        }
        Ok(rows)
    }
}

fn emit(cell: &FrozenCell, stage: &str, arm: &str, code: u16, selected: bool, metrics: &Metrics) {
    println!(
        "{}\t{}\t{}\t{stage}\t{arm}\t{code}\t{selected}\t{:.17e}\t{:.17e}\t{}\t{}\t{}",
        cell.trial.regime.name(),
        cell.trial.seed,
        cell.keep,
        metrics.task_mse,
        metrics.reconstruction_mse,
        metrics.work.multiplications,
        metrics.work.relus,
        metrics.work.mask_tests,
    );
}

/// Execute the explicitly exploratory audit, without modifying the v1 mode.
///
/// # Errors
/// Returns any fixture, scoring, identity, budget or frozen-binding failure.
pub(super) fn run() -> Result<()> {
    let mut cells = Vec::new();
    for trial in trials() {
        let train = generate(trial, Split::Train)?;
        let search = generate(trial, Split::Search)?;
        let (weights, energy) = fit(&train)?;
        for keep in RETAINED {
            cells.push(prepare_cell(trial, weights, &energy, &search, keep)?);
        }
    }
    if cells.len() != 36 {
        return Err("incomplete matched-budget panel".into());
    }
    println!("# schema=bl14.matched-topology-budget.v1; evidence=EXPLORATORY_REANALYSIS");
    println!("# reuses previously observed BL-14.2.2 data; NOT independent validation");
    println!("# equal budget means unique-mask task scoring only; direct_all has a larger budget");
    println!("# work excludes training, predicates, enumeration, scoring, dense oracle and memory");
    println!(
        "regime\tseed\tkeep\tstage\tarm\tmask\tselected\ttask_mse\treconstruction_mse\tmuls\trelus\tmask_tests"
    );
    for cell in &cells {
        println!(
            "# cell={}:{}:{}; weight_bits={:?}; predicates={:?}",
            cell.trial.regime.name(),
            cell.trial.seed,
            cell.keep,
            cell.weights.map(f64::to_bits),
            cell.predicates
        );
        emit(cell, "SEARCH", "dense", 255, true, &cell.dense_search);
        for choice in &cell.heuristics {
            emit(
                cell,
                "SEARCH",
                &choice.id,
                mask_code(&choice.mask),
                true,
                &choice.search,
            );
        }
        for arm in &cell.arms {
            println!(
                "# budget arm={}; masks={}; candidate_examples={}; muls={}; relus={}; mask_tests={}",
                arm.name,
                arm.choices.len(),
                arm.candidate_examples,
                arm.work.multiplications,
                arm.work.relus,
                arm.work.mask_tests
            );
            for (index, choice) in arm.choices.iter().enumerate() {
                println!(
                    "# arm={}; mask={}; truth_table_codes={:?}",
                    arm.name,
                    mask_code(&choice.mask),
                    choice.codes
                );
                emit(
                    cell,
                    "SEARCH",
                    arm.name,
                    mask_code(&choice.mask),
                    arm.selected.contains(&index),
                    &choice.search,
                );
            }
        }
    }
    println!("# ALL_36_CELLS_FROZEN_BEFORE_VALIDATION");
    for cell in &cells {
        for row in cell.validate(&generate(cell.trial, Split::Validation)?)? {
            emit(cell, "VALIDATION", row.arm, row.mask, true, &row.metrics);
        }
    }
    println!("# COMPLETE_CELLS=36");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exhaustive_masks_have_exact_binomial_cardinality_and_roundtrip() {
        for (keep, expected) in [1, 8, 28, 56, 70, 56, 28, 8, 1].into_iter().enumerate() {
            let masks = all_masks(keep).unwrap();
            assert_eq!(masks.len(), expected);
            assert_eq!(
                masks.iter().map(mask_code).collect::<BTreeSet<_>>().len(),
                expected
            );
            for mask in masks {
                assert_eq!(mask.cardinality().retained(), keep);
                assert_eq!(from_code(mask_code(&mask)).unwrap(), mask);
            }
        }
        assert!(from_code(256).is_err());
        assert!(all_masks(9).is_err());
    }

    #[test]
    fn matched_population_is_unique_anchored_and_label_independent() {
        for keep in RETAINED {
            let all = all_masks(keep).unwrap();
            let anchor = &all[0];
            for budget in 1..=all.len() {
                let population = matched_population(keep, budget, 99, anchor).unwrap();
                let codes: Vec<_> = population.iter().map(|row| mask_code(&row.mask)).collect();
                assert_eq!(codes.len(), budget);
                assert_eq!(codes.iter().collect::<BTreeSet<_>>().len(), budget);
                assert_eq!(population[0].mask, *anchor);
                let repeat = matched_population(keep, budget, 99, anchor).unwrap();
                assert_eq!(
                    codes,
                    repeat
                        .iter()
                        .map(|row| mask_code(&row.mask))
                        .collect::<Vec<_>>()
                );
            }
            assert!(matched_population(keep, 0, 99, anchor).is_err());
            assert!(matched_population(keep, all.len() + 1, 99, anchor).is_err());
        }
    }

    #[test]
    fn every_cell_matches_scoring_budgets_and_preserves_direct_equivalence() {
        for trial in trials() {
            let (weights, energy) = fit(&generate(trial, Split::Train).unwrap()).unwrap();
            let search = generate(trial, Split::Search).unwrap();
            for keep in RETAINED {
                let cell = prepare_cell(trial, weights, &energy, &search, keep).unwrap();
                let k = u64::try_from(cell.arms[0].choices.len()).unwrap();
                for arm in &cell.arms[..3] {
                    assert_eq!(arm.candidate_examples, k * 64);
                    assert_eq!(
                        arm.work.multiplications,
                        k * 64 * u64::try_from(keep * 5).unwrap()
                    );
                    assert_eq!(arm.work.mask_tests, k * 64 * 8);
                    assert_eq!(arm.selected, minimum_indices(&arm.choices).unwrap());
                }
                assert_eq!(cell.arms[0].selected, cell.arms[1].selected);
                cell.validate(&generate(trial, Split::Validation).unwrap())
                    .unwrap();
            }
        }
    }

    #[test]
    fn all_search_ties_survive_instead_of_using_validation() {
        let search = generate(trials()[0], Split::Search).unwrap();
        let population = all_masks(4)
            .unwrap()
            .into_iter()
            .map(direct_family)
            .collect();
        let arm = search_arm("all", &[0.0; UNITS], &search, population, 4).unwrap();
        assert_eq!(arm.selected, (0..70).collect::<Vec<_>>());
    }

    #[test]
    fn malformed_populations_and_nonsearch_batches_fail_closed() {
        let search = generate(trials()[0], Split::Search).unwrap();
        let mask = from_code(15).unwrap();
        let duplicate = vec![direct_family(mask.clone()), direct_family(mask.clone())];
        assert!(search_arm("bad", &[0.0; UNITS], &search, duplicate, 4).is_err());
        assert!(search_arm("bad", &[0.0; UNITS], &search, Vec::new(), 4).is_err());
        assert!(
            search_arm(
                "bad",
                &[0.0; UNITS],
                &search,
                vec![direct_family(mask.clone())],
                2
            )
            .is_err()
        );
        let validation = generate(trials()[0], Split::Validation).unwrap();
        assert!(
            search_arm(
                "bad",
                &[0.0; UNITS],
                &validation,
                vec![direct_family(mask)],
                4
            )
            .is_err()
        );
        assert!(boolean_population(&[], 4).is_err());
    }

    #[test]
    fn validation_only_changes_reported_losses_not_frozen_selections() {
        let trial = trials()[0];
        let (weights, energy) = fit(&generate(trial, Split::Train).unwrap()).unwrap();
        let search = generate(trial, Split::Search).unwrap();
        let cell = prepare_cell(trial, weights, &energy, &search, 4).unwrap();
        let snapshot = format!("{cell:?}");
        let mut validation = generate(trial, Split::Validation).unwrap();
        let first = cell.validate(&validation).unwrap();
        assert_eq!(first, cell.validate(&validation).unwrap());
        for sample in &mut validation.samples {
            sample.target += 100.0;
        }
        assert!(
            cell.validate(&validation).unwrap()[0].metrics.task_mse > first[0].metrics.task_mse
        );
        assert_eq!(snapshot, format!("{cell:?}"));
        assert!(cell.validate(&search).is_err());
        validation.samples[0].id = 0;
        assert!(cell.validate(&validation).is_err());
    }

    #[test]
    fn four_unit_boolean_population_matches_unchanged_v1_fixture() {
        let trial = trials()[0];
        let legacy = super::super::prepare(trial).unwrap();
        let (weights, energy) = fit(&generate(trial, Split::Train).unwrap()).unwrap();
        let cell = prepare_cell(
            trial,
            weights,
            &energy,
            &generate(trial, Split::Search).unwrap(),
            4,
        )
        .unwrap();
        assert_eq!(
            legacy.weights.map(f64::to_bits),
            cell.weights.map(f64::to_bits)
        );
        assert_eq!(legacy.predicates, cell.predicates);
        assert_eq!(legacy.selected, cell.arms[0].selected);
        assert_eq!(legacy.boolean.len(), cell.arms[0].choices.len());
        for (left, right) in legacy.boolean.iter().zip(&cell.arms[0].choices) {
            assert_eq!(left.mask, right.mask);
            assert_eq!(left.codes, right.codes);
            assert!(same_metrics(&left.search, &right.search));
        }
    }
}
