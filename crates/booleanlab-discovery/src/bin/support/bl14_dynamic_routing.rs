//! BL-14.3.1 development pilot for input-conditioned Boolean sparsity.
//!
//! This module deliberately reuses the frozen BL-14.2.2 numerical workload.
//! A single raw-input predicate selects between two exact 4/8 Boolean-derived
//! masks. Selection is fitted on SEARCH only and all trials are frozen before
//! any VALIDATION batch is constructed. Reference operation counters are not
//! elapsed-time, memory-traffic, energy, or hardware-performance evidence.

use std::cmp::Ordering;

use booleanlab_core::{ExactMask, deterministic_random_mask};
use booleanlab_discovery::sparsity_function_mask::materialize_boolean_function_mask;

use super::{
    Batch, FrozenTrial, KEEP, Metrics, Result, Split, Trial, UNITS, Work, count, dot, features,
    finite, generate, increment, mask_code, predict, prepare, require_batch, trials,
};

const NEGATIVE_RANDOM_SEED: u64 = 0xB114_0300;
const NONNEGATIVE_RANDOM_SEED: u64 = 0xB114_0301;

#[derive(Clone, Debug, PartialEq)]
struct DynamicMetrics {
    metrics: Metrics,
    controller_predicate_tests: u64,
    mask_switches: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct ReportRow {
    id: String,
    metrics: Metrics,
    controller_predicate_tests: u64,
    mask_switches: u64,
}

#[derive(Debug)]
struct DynamicStudy {
    base: FrozenTrial,
    static_boolean: usize,
    negative_boolean: usize,
    nonnegative_boolean: usize,
    negative_random: ExactMask,
    nonnegative_random: ExactMask,
    search_candidate_sample_evaluations: usize,
}

fn add_work(total: &mut Work, value: &Work) -> Result<()> {
    increment(&mut total.multiplications, value.multiplications)?;
    increment(&mut total.relus, value.relus)?;
    increment(&mut total.mask_tests, value.mask_tests)?;
    Ok(())
}

fn finalize(mut metrics: Metrics, samples: usize) -> Result<Metrics> {
    let divisor = count(samples)?;
    metrics.task_mse = finite(metrics.task_mse / divisor)?;
    metrics.reconstruction_mse = finite(metrics.reconstruction_mse / divisor)?;
    Ok(metrics)
}

fn score_subset(
    study: &FrozenTrial,
    batch: &Batch,
    mask: &ExactMask,
    nonnegative: bool,
) -> Result<Metrics> {
    require_batch(batch, study.trial, Split::Search)?;
    if mask.cardinality().retained() != KEEP {
        return Err("dynamic search mask does not preserve the declared 4/8 density".into());
    }
    let mut samples = 0usize;
    let mut metrics = Metrics {
        task_mse: 0.0,
        reconstruction_mse: 0.0,
        work: Work::default(),
    };
    for sample in &batch.samples {
        if (sample.input[0] >= 0.0) != nonnegative {
            continue;
        }
        samples = samples
            .checked_add(1)
            .ok_or("partition sample count overflow")?;
        let reference = dot(&study.weights, &features(&sample.input)?)?;
        let (actual, work) = predict(&study.weights, &sample.input, Some(mask))?;
        let task_error = finite(actual - finite(sample.target)?)?;
        let reconstruction = finite(actual - reference)?;
        metrics.task_mse = finite(metrics.task_mse + finite(task_error * task_error)?)?;
        metrics.reconstruction_mse =
            finite(metrics.reconstruction_mse + finite(reconstruction * reconstruction)?)?;
        add_work(&mut metrics.work, &work)?;
    }
    finalize(metrics, samples)
}

fn select_partition(study: &FrozenTrial, search: &Batch, nonnegative: bool) -> Result<usize> {
    let mut best: Option<(usize, Metrics)> = None;
    for (index, choice) in study.boolean.iter().enumerate() {
        let metrics = score_subset(study, search, &choice.mask, nonnegative)?;
        let replace = match &best {
            None => true,
            Some((best_index, best_metrics)) => {
                let quality = metrics.task_mse.total_cmp(&best_metrics.task_mse);
                quality == Ordering::Less
                    || (quality == Ordering::Equal
                        && mask_code(&choice.mask) < mask_code(&study.boolean[*best_index].mask))
            }
        };
        if replace {
            best = Some((index, metrics));
        }
    }
    best.map(|(index, _)| index)
        .ok_or_else(|| "no Boolean topology is eligible for dynamic routing".into())
}

fn validate_boolean_choice(study: &FrozenTrial, index: usize) -> Result<()> {
    let choice = study
        .boolean
        .get(index)
        .ok_or("dynamic Boolean choice index is out of range")?;
    if choice.mask.cardinality().retained() != KEEP
        || choice.codes.is_empty()
        || choice.functions.is_empty()
        || choice.codes.len() != choice.functions.len()
    {
        return Err("dynamic Boolean choice lost its frozen exact representation".into());
    }
    let predicates: Vec<&[bool]> = study.predicates.iter().map(Vec::as_slice).collect();
    for function in &choice.functions {
        if materialize_boolean_function_mask(function, &predicates)? != choice.mask {
            return Err("dynamic Boolean choice no longer materializes its frozen mask".into());
        }
    }
    Ok(())
}

fn score_dynamic(
    study: &FrozenTrial,
    batch: &Batch,
    negative: &ExactMask,
    nonnegative: &ExactMask,
) -> Result<DynamicMetrics> {
    require_batch(batch, study.trial, batch.split)?;
    if negative.cardinality().retained() != KEEP || nonnegative.cardinality().retained() != KEEP {
        return Err("dynamic route must retain exactly 4/8 units on both branches".into());
    }
    let mut metrics = Metrics {
        task_mse: 0.0,
        reconstruction_mse: 0.0,
        work: Work::default(),
    };
    let mut controller_predicate_tests = 0u64;
    let mut mask_switches = 0u64;
    let mut previous_route: Option<bool> = None;
    for sample in &batch.samples {
        // The controller observes a raw input sign before any candidate unit's
        // projection/ReLU is evaluated. One comparison is accounted separately.
        let route_nonnegative = sample.input[0] >= 0.0;
        increment(&mut controller_predicate_tests, 1)?;
        if previous_route.is_some_and(|previous| previous != route_nonnegative) {
            increment(&mut mask_switches, 1)?;
        }
        previous_route = Some(route_nonnegative);
        let mask = if route_nonnegative {
            nonnegative
        } else {
            negative
        };
        let reference = dot(&study.weights, &features(&sample.input)?)?;
        let (actual, work) = predict(&study.weights, &sample.input, Some(mask))?;
        let task_error = finite(actual - finite(sample.target)?)?;
        let reconstruction = finite(actual - reference)?;
        metrics.task_mse = finite(metrics.task_mse + finite(task_error * task_error)?)?;
        metrics.reconstruction_mse =
            finite(metrics.reconstruction_mse + finite(reconstruction * reconstruction)?)?;
        add_work(&mut metrics.work, &work)?;
    }
    Ok(DynamicMetrics {
        metrics: finalize(metrics, batch.samples.len())?,
        controller_predicate_tests,
        mask_switches,
    })
}

fn plain(id: impl Into<String>, metrics: Metrics) -> ReportRow {
    ReportRow {
        id: id.into(),
        metrics,
        controller_predicate_tests: 0,
        mask_switches: 0,
    }
}

fn dynamic(id: impl Into<String>, metrics: DynamicMetrics) -> ReportRow {
    ReportRow {
        id: id.into(),
        metrics: metrics.metrics,
        controller_predicate_tests: metrics.controller_predicate_tests,
        mask_switches: metrics.mask_switches,
    }
}

fn prepare_dynamic(trial: Trial) -> Result<DynamicStudy> {
    let base = prepare(trial)?;
    let search = generate(trial, Split::Search)?;
    require_batch(&search, trial, Split::Search)?;
    let static_boolean = *base
        .selected
        .first()
        .ok_or("no frozen static Boolean winner")?;
    let negative_boolean = select_partition(&base, &search, false)?;
    let nonnegative_boolean = select_partition(&base, &search, true)?;
    for index in [static_boolean, negative_boolean, nonnegative_boolean] {
        validate_boolean_choice(&base, index)?;
    }
    let negative_random = deterministic_random_mask(UNITS, KEEP, NEGATIVE_RANDOM_SEED)?;
    let nonnegative_random = deterministic_random_mask(UNITS, KEEP, NONNEGATIVE_RANDOM_SEED)?;
    let search_candidate_sample_evaluations = base
        .boolean
        .len()
        .checked_mul(search.samples.len())
        .ok_or("dynamic search budget overflow")?;
    Ok(DynamicStudy {
        base,
        static_boolean,
        negative_boolean,
        nonnegative_boolean,
        negative_random,
        nonnegative_random,
        search_candidate_sample_evaluations,
    })
}

impl DynamicStudy {
    fn rows(&self, split: Split) -> Result<Vec<ReportRow>> {
        if !matches!(split, Split::Search | Split::Validation) {
            return Err("dynamic routing may only score SEARCH or non-final VALIDATION".into());
        }
        for index in [
            self.static_boolean,
            self.negative_boolean,
            self.nonnegative_boolean,
        ] {
            validate_boolean_choice(&self.base, index)?;
        }
        let batch = generate(self.base.trial, split)?;
        require_batch(&batch, self.base.trial, split)?;
        let mut rows = Vec::new();
        let dense = if split == Split::Search {
            self.base.dense_search.clone()
        } else {
            super::score(&self.base.weights, &batch, None)?
        };
        rows.push(plain("dense", dense));
        for choice in &self.base.baselines {
            let metrics = if split == Split::Search {
                choice.search.clone()
            } else {
                super::score(&self.base.weights, &batch, Some(&choice.mask))?
            };
            rows.push(plain(choice.id.clone(), metrics));
        }
        let static_choice = &self.base.boolean[self.static_boolean];
        let static_metrics = if split == Split::Search {
            static_choice.search.clone()
        } else {
            super::score(&self.base.weights, &batch, Some(&static_choice.mask))?
        };
        rows.push(plain("boolean_static_best", static_metrics));
        rows.push(dynamic(
            "boolean_dynamic_x0_sign",
            score_dynamic(
                &self.base,
                &batch,
                &self.base.boolean[self.negative_boolean].mask,
                &self.base.boolean[self.nonnegative_boolean].mask,
            )?,
        ));
        rows.push(dynamic(
            "random_dynamic_x0_sign",
            score_dynamic(
                &self.base,
                &batch,
                &self.negative_random,
                &self.nonnegative_random,
            )?,
        ));
        Ok(rows)
    }
}

fn emit(trial: Trial, stage: &str, row: &ReportRow) {
    println!(
        "{}\t{}\t{stage}\t{}\t{:.17e}\t{:.17e}\t{}\t{}\t{}\t{}\t{}",
        trial.regime.name(),
        trial.seed,
        row.id,
        row.metrics.task_mse,
        row.metrics.reconstruction_mse,
        row.metrics.work.multiplications,
        row.metrics.work.relus,
        row.metrics.work.mask_tests,
        row.controller_predicate_tests,
        row.mask_switches,
    );
}

/// Execute the BL-14.3.1 non-final dynamic-routing development pilot.
pub(super) fn run() -> Result<()> {
    // Freeze every fitted model, Boolean candidate family, branch winner and
    // random control before constructing any VALIDATION batch.
    let frozen: Vec<DynamicStudy> = trials()
        .into_iter()
        .map(prepare_dynamic)
        .collect::<Result<_>>()?;
    println!("# schema=bl14.dynamic-routing.v1; phase=NUMERICAL_DEVELOPMENT; trials=12");
    println!("# selector=(input[0] >= 0); every routed mask retains exactly 4/8 units");
    println!("# Boolean branch masks come only from the frozen BL-14.2.2 Boolean topology family");
    println!(
        "# controller_predicate_tests and mask_switches are reference counts, not elapsed time"
    );
    println!("# no hardware timing, memory traffic, energy, final holdout or speedup claim");
    println!(
        "regime\tseed\tstage\tpolicy\ttask_mse\treconstruction_mse\tmuls\trelus\tmask_tests\tcontroller_predicate_tests\tmask_switches"
    );
    for study in &frozen {
        println!(
            "# trial={}:{}; boolean_unique_masks={}; search_candidate_sample_evaluations={}; static_mask={}; negative_mask={}; nonnegative_mask={}; random_negative_mask={}; random_nonnegative_mask={}",
            study.base.trial.regime.name(),
            study.base.trial.seed,
            study.base.boolean.len(),
            study.search_candidate_sample_evaluations,
            mask_code(&study.base.boolean[study.static_boolean].mask),
            mask_code(&study.base.boolean[study.negative_boolean].mask),
            mask_code(&study.base.boolean[study.nonnegative_boolean].mask),
            mask_code(&study.negative_random),
            mask_code(&study.nonnegative_random),
        );
        println!(
            "# negative_truth_table_codes={:?}; nonnegative_truth_table_codes={:?}",
            study.base.boolean[study.negative_boolean].codes,
            study.base.boolean[study.nonnegative_boolean].codes,
        );
        for row in study.rows(Split::Search)? {
            emit(study.base.trial, "SEARCH", &row);
        }
    }
    println!("# ALL_DYNAMIC_CONTROLLERS_FROZEN_BEFORE_VALIDATION");
    for study in &frozen {
        for row in study.rows(Split::Validation)? {
            emit(study.base.trial, "VALIDATION", &row);
        }
    }
    println!("# COMPLETE_TRIALS=12");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_routes_are_frozen_on_search_and_keep_exact_density() {
        for trial in trials() {
            let study = prepare_dynamic(trial).unwrap();
            assert_eq!(
                study.search_candidate_sample_evaluations,
                study.base.boolean.len() * 64
            );
            for index in [
                study.static_boolean,
                study.negative_boolean,
                study.nonnegative_boolean,
            ] {
                assert_eq!(
                    study.base.boolean[index].mask.cardinality().retained(),
                    KEEP
                );
                validate_boolean_choice(&study.base, index).unwrap();
            }
            assert_eq!(study.negative_random.cardinality().retained(), KEEP);
            assert_eq!(study.nonnegative_random.cardinality().retained(), KEEP);
        }
    }

    #[test]
    fn controller_cost_is_separate_and_sparse_work_is_fixed_per_input() {
        let study = prepare_dynamic(trials()[0]).unwrap();
        for split in [Split::Search, Split::Validation] {
            let rows = study.rows(split).unwrap();
            let boolean = rows
                .iter()
                .find(|row| row.id == "boolean_dynamic_x0_sign")
                .unwrap();
            assert_eq!(boolean.controller_predicate_tests, 64);
            assert!(boolean.mask_switches <= 63);
            assert_eq!(
                boolean.metrics.work,
                Work {
                    multiplications: 1280,
                    relus: 256,
                    mask_tests: 512,
                }
            );
            let dense = rows.iter().find(|row| row.id == "dense").unwrap();
            assert_eq!(dense.controller_predicate_tests, 0);
            assert_eq!(dense.metrics.work.multiplications, 2560);
            assert_eq!(dense.metrics.work.relus, 512);
            assert_eq!(dense.metrics.work.mask_tests, 0);
        }
    }

    #[test]
    fn validation_cannot_change_dynamic_selection() {
        let study = prepare_dynamic(trials()[0]).unwrap();
        let frozen = (
            study.static_boolean,
            study.negative_boolean,
            study.nonnegative_boolean,
            mask_code(&study.negative_random),
            mask_code(&study.nonnegative_random),
        );
        assert_eq!(
            study.rows(Split::Validation).unwrap(),
            study.rows(Split::Validation).unwrap()
        );
        assert_eq!(
            frozen,
            (
                study.static_boolean,
                study.negative_boolean,
                study.nonnegative_boolean,
                mask_code(&study.negative_random),
                mask_code(&study.nonnegative_random),
            )
        );
        assert!(study.rows(Split::Train).is_err());
    }
}
