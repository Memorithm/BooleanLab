//! BL-14.2.2 numerical development: fixed `ReLU` features, trained readout.
//! Masks skip entire units before projection. This is not full MLP training.

#[path = "support/bl14_dynamic_routing.rs"]
mod dynamic_routing;
#[path = "support/bl14_matched_search.rs"]
mod matched_search;
#[path = "support/bl14_relational.rs"]
#[allow(dead_code)]
mod relational;

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;

use booleanlab_core::{
    ExactMask, deterministic_random_keys, deterministic_random_mask,
    mask_from_descending_u64_scores, structured_nm_mask_from_u64_scores,
};
use booleanlab_discovery::BooleanFunction;
use booleanlab_discovery::sparsity_exhaustive_search::propose_exhaustive_rules;
use booleanlab_discovery::sparsity_function_mask::materialize_boolean_function_mask;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[allow(dead_code)]
pub(crate) fn run_relational_entry() -> Result<()> {
    relational::run()
}

const INPUTS: usize = 4;
const UNITS: usize = 8;
const KEEP: usize = 4;
const PROJECTIONS: [[f64; INPUTS]; UNITS] = [
    [0.5, 0.5, 0.5, 0.5],
    [-0.5, 0.5, 0.5, 0.5],
    [0.5, -0.5, 0.5, 0.5],
    [0.5, 0.5, -0.5, 0.5],
    [0.5, 0.5, 0.5, -0.5],
    [-0.5, -0.5, 0.5, 0.5],
    [0.5, -0.5, -0.5, 0.5],
    [-0.5, 0.5, -0.5, 0.5],
];
const TEACHER: [f64; UNITS] = [2.0, -1.0, 0.5, 1.5, -2.0, 0.25, 1.0, -0.5];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Regime {
    Isotropic,
    UnequalScale,
    Correlated,
}

impl Regime {
    fn name(self) -> &'static str {
        match self {
            Self::Isotropic => "isotropic",
            Self::UnequalScale => "unequal_scale",
            Self::Correlated => "correlated",
        }
    }

    fn transform(self, raw: [f64; INPUTS]) -> [f64; INPUTS] {
        match self {
            Self::Isotropic => raw,
            Self::UnequalScale => [raw[0], 2.0 * raw[1], 0.5 * raw[2], 0.25 * raw[3]],
            Self::Correlated => [
                raw[0],
                0.75 * raw[0] + 0.25 * raw[1],
                raw[2],
                0.75 * raw[2] + 0.25 * raw[3],
            ],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Trial {
    regime: Regime,
    seed: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Split {
    Train,
    Search,
    Validation,
}

impl Split {
    fn ids(self) -> std::ops::Range<usize> {
        match self {
            Self::Train => 0..128,
            Self::Search => 128..192,
            Self::Validation => 192..256,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Sample {
    id: usize,
    input: [f64; INPUTS],
    target: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Batch {
    trial: Trial,
    split: Split,
    samples: Vec<Sample>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Work {
    multiplications: u64,
    relus: u64,
    mask_tests: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct Metrics {
    task_mse: f64,
    reconstruction_mse: f64,
    work: Work,
}

#[derive(Clone, Debug)]
struct Choice {
    id: String,
    mask: ExactMask,
    codes: Vec<u64>,
    functions: Vec<BooleanFunction>,
    search: Metrics,
}

#[derive(Debug)]
struct MaskFamily {
    mask: ExactMask,
    codes: Vec<u64>,
    functions: Vec<BooleanFunction>,
}

#[derive(Debug)]
struct FrozenTrial {
    trial: Trial,
    weights: [f64; UNITS],
    predicates: Vec<Vec<bool>>,
    train_before: f64,
    train_after: f64,
    dense_search: Metrics,
    baselines: Vec<Choice>,
    boolean: Vec<Choice>,
    selected: Vec<usize>,
}

fn finite(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err("non-finite data, arithmetic or metric".into())
    }
}

fn count(value: usize) -> Result<f64> {
    if value == 0 {
        return Err("empty batch".into());
    }
    Ok(f64::from(u32::try_from(value)?))
}

fn increment(total: &mut u64, value: u64) -> Result<()> {
    *total = total
        .checked_add(value)
        .ok_or("reference counter overflow")?;
    Ok(())
}

fn feature(unit: usize, input: &[f64; INPUTS]) -> Result<f64> {
    let sum = PROJECTIONS[unit]
        .iter()
        .zip(input)
        .try_fold(0.125, |sum, (&weight, &value)| {
            finite(sum + finite(weight * finite(value)?)?)
        })?;
    Ok(sum.max(0.0))
}

fn features(input: &[f64; INPUTS]) -> Result<[f64; UNITS]> {
    let mut output = [0.0; UNITS];
    for (unit, value) in output.iter_mut().enumerate() {
        *value = feature(unit, input)?;
    }
    Ok(output)
}

fn dot(weights: &[f64; UNITS], values: &[f64; UNITS]) -> Result<f64> {
    weights
        .iter()
        .zip(values)
        .try_fold(0.0, |sum, (&weight, &value)| {
            finite(sum + finite(finite(weight)? * finite(value)?)?)
        })
}

fn trials() -> Vec<Trial> {
    [Regime::Isotropic, Regime::UnequalScale, Regime::Correlated]
        .into_iter()
        .flat_map(|regime| (0..4).map(move |seed| Trial { regime, seed }))
        .collect()
}

fn generate(trial: Trial, split: Split) -> Result<Batch> {
    if trial.seed >= 4 {
        return Err("undeclared trial seed".into());
    }
    let keys = deterministic_random_keys(256 * INPUTS, 0xB114_0022 + trial.seed)?;
    let mut samples = Vec::new();
    for id in split.ids() {
        let mut raw = [0.0; INPUTS];
        for (column, value) in raw.iter_mut().enumerate() {
            let integer = u32::try_from(keys[id * INPUTS + column] % 65)?;
            *value = (f64::from(integer) - 32.0) / 32.0;
        }
        let input = trial.regime.transform(raw);
        samples.push(Sample {
            id,
            target: dot(&TEACHER, &features(&input)?)?,
            input,
        });
    }
    Ok(Batch {
        trial,
        split,
        samples,
    })
}

fn require_batch(batch: &Batch, trial: Trial, split: Split) -> Result<()> {
    if batch.trial != trial || batch.split != split || trial.seed >= 4 {
        return Err("wrong trial or split".into());
    }
    let ids = split.ids();
    if batch.samples.len() != ids.len() {
        return Err("missing or extra samples".into());
    }
    let mut seen = BTreeSet::new();
    for sample in &batch.samples {
        if !ids.contains(&sample.id) || !seen.insert(sample.id) {
            return Err("duplicate or foreign instance identity".into());
        }
        finite(sample.target)?;
        for &value in &sample.input {
            finite(value)?;
        }
    }
    Ok(())
}

fn fit(train: &Batch) -> Result<([f64; UNITS], [f64; UNITS])> {
    require_batch(train, train.trial, Split::Train)?;
    let rows: Vec<[f64; UNITS]> = train
        .samples
        .iter()
        .map(|sample| features(&sample.input))
        .collect::<Result<_>>()?;
    let divisor = count(rows.len())?;
    let mut energy = [0.0; UNITS];
    for row in &rows {
        for (sum, &value) in energy.iter_mut().zip(row) {
            *sum = finite(*sum + finite(value * value)?)?;
        }
    }
    for value in &mut energy {
        *value = finite(*value / divisor)?;
    }
    let mut weights = [0.0; UNITS];
    for _ in 0..512 {
        let mut gradient = [0.0; UNITS];
        for (sample, row) in train.samples.iter().zip(&rows) {
            let error = finite(dot(&weights, row)? - sample.target)?;
            for (derivative, &value) in gradient.iter_mut().zip(row) {
                *derivative = finite(*derivative + finite(2.0 * error * value)?)?;
            }
        }
        for (weight, derivative) in weights.iter_mut().zip(gradient) {
            *weight = finite(*weight - 0.0625 * derivative / divisor)?;
        }
    }
    Ok((weights, energy))
}

fn predict(
    weights: &[f64; UNITS],
    input: &[f64; INPUTS],
    mask: Option<&ExactMask>,
) -> Result<(f64, Work)> {
    if mask.is_some_and(|mask| mask.as_slice().len() != UNITS) {
        return Err("mask width mismatch".into());
    }
    for &value in weights.iter().chain(input.iter()) {
        finite(value)?;
    }
    let mut work = Work::default();
    let mut output = 0.0;
    for (unit, &weight) in weights.iter().enumerate() {
        if let Some(mask) = mask {
            increment(&mut work.mask_tests, 1)?;
            if !mask.as_slice()[unit] {
                continue;
            }
        }
        // Admission precedes the projection, ReLU and readout contribution.
        let value = feature(unit, input)?;
        output = finite(output + finite(weight * value)?)?;
        increment(&mut work.multiplications, u64::try_from(INPUTS + 1)?)?;
        increment(&mut work.relus, 1)?;
    }
    Ok((output, work))
}

fn score(weights: &[f64; UNITS], batch: &Batch, mask: Option<&ExactMask>) -> Result<Metrics> {
    let divisor = count(batch.samples.len())?;
    let mut metrics = Metrics {
        task_mse: 0.0,
        reconstruction_mse: 0.0,
        work: Work::default(),
    };
    for sample in &batch.samples {
        // Evaluator-only dense oracle, explicitly excluded from inference counters.
        let reference = dot(weights, &features(&sample.input)?)?;
        let (actual, work) = predict(weights, &sample.input, mask)?;
        let task_error = finite(actual - finite(sample.target)?)?;
        let reconstruction = finite(actual - reference)?;
        metrics.task_mse = finite(metrics.task_mse + finite(task_error * task_error)?)?;
        metrics.reconstruction_mse =
            finite(metrics.reconstruction_mse + finite(reconstruction * reconstruction)?)?;
        increment(&mut metrics.work.multiplications, work.multiplications)?;
        increment(&mut metrics.work.relus, work.relus)?;
        increment(&mut metrics.work.mask_tests, work.mask_tests)?;
    }
    metrics.task_mse = finite(metrics.task_mse / divisor)?;
    metrics.reconstruction_mse = finite(metrics.reconstruction_mse / divisor)?;
    Ok(metrics)
}

fn keys(values: &[f64; UNITS]) -> Result<Vec<u64>> {
    values
        .iter()
        .map(|&value| Ok(finite(value)?.abs().to_bits()))
        .collect()
}

fn mask_code(mask: &ExactMask) -> u16 {
    mask.as_slice()
        .iter()
        .enumerate()
        .fold(0, |code, (bit, &keep)| code | (u16::from(keep) << bit))
}

fn baseline_masks(
    weights: &[f64; UNITS],
    energy: &[f64; UNITS],
) -> Result<Vec<(String, ExactMask)>> {
    let magnitude = keys(weights)?;
    let mut saliency = [0.0; UNITS];
    for (unit, value) in saliency.iter_mut().enumerate() {
        *value = finite(weights[unit] * weights[unit] * energy[unit])?;
    }
    let mut masks = vec![
        (
            "magnitude".to_owned(),
            mask_from_descending_u64_scores(&magnitude, KEEP)?,
        ),
        (
            "unit_2_4".to_owned(),
            structured_nm_mask_from_u64_scores(&magnitude, 2, 4)?,
        ),
        (
            "activation_energy".to_owned(),
            mask_from_descending_u64_scores(&keys(&saliency)?, KEEP)?,
        ),
    ];
    for seed in 0..4 {
        masks.push((
            format!("random_{seed}"),
            deterministic_random_mask(UNITS, KEEP, seed)?,
        ));
    }
    Ok(masks)
}

fn group_rules(predicates: &[Vec<bool>]) -> Result<Vec<MaskFamily>> {
    let rows: Vec<&[bool]> = predicates.iter().map(Vec::as_slice).collect();
    let mut groups: BTreeMap<u16, MaskFamily> = BTreeMap::new();
    for rule in propose_exhaustive_rules(3)? {
        let mask = materialize_boolean_function_mask(&rule.function, &rows)?;
        if mask.cardinality().retained() != KEEP {
            continue;
        }
        let group = groups
            .entry(mask_code(&mask))
            .or_insert_with(|| MaskFamily {
                mask,
                codes: Vec::new(),
                functions: Vec::new(),
            });
        group.codes.push(rule.truth_table_code);
        group.functions.push(rule.function);
    }
    Ok(groups.into_values().collect())
}

fn prepare(trial: Trial) -> Result<FrozenTrial> {
    let train = generate(trial, Split::Train)?;
    let search = generate(trial, Split::Search)?;
    require_batch(&search, trial, Split::Search)?;
    let (weights, energy) = fit(&train)?;
    let masks = baseline_masks(&weights, &energy)?;
    let energetic = mask_from_descending_u64_scores(&keys(&energy)?, KEEP)?;
    let predicates: Vec<Vec<bool>> = (0..UNITS)
        .map(|unit| {
            vec![
                masks[0].1.as_slice()[unit],
                weights[unit] < 0.0,
                energetic.as_slice()[unit],
            ]
        })
        .collect();
    let mut baselines = Vec::new();
    for (id, mask) in masks {
        baselines.push(Choice {
            id,
            search: score(&weights, &search, Some(&mask))?,
            mask,
            codes: Vec::new(),
            functions: Vec::new(),
        });
    }
    let mut boolean = Vec::new();
    for family in group_rules(&predicates)? {
        boolean.push(Choice {
            id: format!("boolean-mask-{:02x}", mask_code(&family.mask)),
            search: score(&weights, &search, Some(&family.mask))?,
            mask: family.mask,
            codes: family.codes,
            functions: family.functions,
        });
    }
    let best = boolean
        .iter()
        .map(|choice| choice.search.task_mse)
        .min_by(f64::total_cmp)
        .ok_or("no eligible Boolean topology")?;
    let selected = boolean
        .iter()
        .enumerate()
        .filter_map(|(index, choice)| {
            (choice.search.task_mse.total_cmp(&best) == Ordering::Equal).then_some(index)
        })
        .collect();
    Ok(FrozenTrial {
        trial,
        train_before: score(&[0.0; UNITS], &train, None)?.task_mse,
        train_after: score(&weights, &train, None)?.task_mse,
        dense_search: score(&weights, &search, None)?,
        weights,
        predicates,
        baselines,
        boolean,
        selected,
    })
}

impl FrozenTrial {
    fn validate(&self, batch: &Batch) -> Result<Vec<(String, Metrics)>> {
        require_batch(batch, self.trial, Split::Validation)?;
        let mut output = vec![("dense".to_owned(), score(&self.weights, batch, None)?)];
        let predicates: Vec<&[bool]> = self.predicates.iter().map(Vec::as_slice).collect();
        for choice in self
            .baselines
            .iter()
            .chain(self.selected.iter().map(|&i| &self.boolean[i]))
        {
            if choice.mask.cardinality().retained() != KEEP {
                return Err("frozen density changed".into());
            }
            for function in &choice.functions {
                if materialize_boolean_function_mask(function, &predicates)? != choice.mask {
                    return Err("frozen Boolean semantics changed".into());
                }
            }
            output.push((
                choice.id.clone(),
                score(&self.weights, batch, Some(&choice.mask))?,
            ));
        }
        Ok(output)
    }
}

fn emit(trial: Trial, stage: &str, id: &str, metrics: &Metrics) {
    println!(
        "{}\t{}\t{stage}\t{id}\t{:.17e}\t{:.17e}\t{}\t{}\t{}",
        trial.regime.name(),
        trial.seed,
        metrics.task_mse,
        metrics.reconstruction_mse,
        metrics.work.multiplications,
        metrics.work.relus,
        metrics.work.mask_tests,
    );
}

fn main() -> Result<()> {
    let mut arguments = std::env::args_os().skip(1);
    match (arguments.next(), arguments.next()) {
        (None, None) => {}
        (Some(mode), None) if mode == "--matched-search-v1" => return matched_search::run(),
        (Some(mode), None) if mode == "--dynamic-routing-v1" => return dynamic_routing::run(),
        _ => {
            return Err(
                "expected no arguments, --matched-search-v1, or --dynamic-routing-v1 only".into(),
            );
        }
    }
    // Freeze ALL twelve trials before constructing ANY validation batch.
    let frozen: Vec<FrozenTrial> = trials().into_iter().map(prepare).collect::<Result<_>>()?;
    println!("# schema=bl14.structured-relu.v1; phase=NUMERICAL_DEVELOPMENT; trials=12");
    println!("# fixed ReLU features; trained readout; all sparse controls keep 4/8 units");
    println!(
        "# counts exclude training/search, statistics, predicates, checks, scoring and dense oracle"
    );
    println!("# no hardware timing, memory traffic, energy or final confirmation");
    println!("regime\tseed\tstage\tpolicy\ttask_mse\treconstruction_mse\tmuls\trelus\tmask_tests");
    for study in &frozen {
        let trial = study.trial;
        let bits = study.weights.map(f64::to_bits);
        let eligible: usize = study.boolean.iter().map(|choice| choice.codes.len()).sum();
        println!(
            "# trial={}:{}; train_before={:.17e}; train_after={:.17e}",
            trial.regime.name(),
            trial.seed,
            study.train_before,
            study.train_after
        );
        println!("# weight_bits={bits:?}; predicates={:?}", study.predicates);
        println!(
            "# eligible_rules={eligible}; unique_masks={}; selected_masks={:?}",
            study.boolean.len(),
            study.selected
        );
        emit(trial, "SEARCH", "dense", &study.dense_search);
        for choice in study.baselines.iter().chain(&study.boolean) {
            println!(
                "# policy={}; mask={}; truth_table_codes={:?}",
                choice.id,
                mask_code(&choice.mask),
                choice.codes
            );
            emit(trial, "SEARCH", &choice.id, &choice.search);
        }
    }
    println!("# ALL_TRIALS_FROZEN_BEFORE_VALIDATION");
    for study in &frozen {
        let batch = generate(study.trial, Split::Validation)?;
        for (id, metrics) in study.validate(&batch)? {
            emit(study.trial, "VALIDATION", &id, &metrics);
        }
    }
    println!("# COMPLETE_TRIALS=12");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twelve_trials_have_exact_disjoint_instance_partitions() {
        let design = trials();
        assert_eq!(design.len(), 12);
        assert_eq!(design.iter().collect::<BTreeSet<_>>().len(), 12);
        for trial in design {
            let mut seen = BTreeSet::new();
            for split in [Split::Train, Split::Search, Split::Validation] {
                let batch = generate(trial, split).unwrap();
                assert_eq!(batch, generate(trial, split).unwrap());
                require_batch(&batch, trial, split).unwrap();
                for sample in batch.samples {
                    assert!(seen.insert(sample.id));
                }
            }
            assert_eq!(seen.len(), 256);
        }
    }

    #[test]
    fn malformed_or_foreign_batches_fail_closed() {
        let trial = trials()[0];
        for split in [Split::Train, Split::Search, Split::Validation] {
            let original = generate(trial, split).unwrap();
            let mut changed = original.clone();
            changed.samples[0].id = changed.samples[1].id;
            assert!(require_batch(&changed, trial, split).is_err());
            changed = original.clone();
            changed.samples.pop();
            assert!(require_batch(&changed, trial, split).is_err());
            changed = original.clone();
            changed.trial.seed = 1;
            assert!(require_batch(&changed, trial, split).is_err());
            changed = original;
            changed.samples[0].target = f64::NAN;
            assert!(require_batch(&changed, trial, split).is_err());
        }
    }

    #[test]
    fn training_uses_only_train_and_reduces_its_error() {
        let trial = trials()[0];
        let train = generate(trial, Split::Train).unwrap();
        let before = score(&[0.0; UNITS], &train, None).unwrap().task_mse;
        let (learned, _) = fit(&train).unwrap();
        assert!(score(&learned, &train, None).unwrap().task_mse < before);
        let mut reversed = train;
        for sample in &mut reversed.samples {
            sample.target = -sample.target;
        }
        let (negative, _) = fit(&reversed).unwrap();
        for (left, right) in learned.iter().zip(negative) {
            assert!((left + right).abs() < 1e-12);
        }
        assert!(fit(&generate(trial, Split::Validation).unwrap()).is_err());
    }

    #[test]
    fn rule_families_preserve_codes_density_and_every_search_tie() {
        let study = prepare(trials()[0]).unwrap();
        let mut masks = BTreeSet::new();
        let mut codes = BTreeSet::new();
        let best = study.boolean[study.selected[0]].search.task_mse;
        for (index, choice) in study.boolean.iter().enumerate() {
            assert!(masks.insert(mask_code(&choice.mask)));
            assert_eq!(choice.mask.cardinality().retained(), KEEP);
            assert_eq!(choice.codes.len(), choice.functions.len());
            for &code in &choice.codes {
                assert!(codes.insert(code));
            }
            assert_eq!(
                study.selected.contains(&index),
                choice.search.task_mse.total_cmp(&best) == Ordering::Equal
            );
        }
        for choice in &study.baselines {
            assert_eq!(choice.mask.cardinality().retained(), KEEP);
            assert_eq!(
                choice.search.work,
                Work {
                    multiplications: 1280,
                    relus: 256,
                    mask_tests: 512,
                }
            );
        }
        assert_eq!(study.baselines.len(), 7);
    }

    #[test]
    fn validation_cannot_refit_or_select_new_topologies() {
        let study = prepare(trials()[0]).unwrap();
        let snapshot = format!("{study:?}");
        let mut batch = generate(study.trial, Split::Validation).unwrap();
        let first = study.validate(&batch).unwrap();
        assert_eq!(first, study.validate(&batch).unwrap());
        for sample in &mut batch.samples {
            sample.target += 100.0;
        }
        assert!(study.validate(&batch).unwrap()[0].1.task_mse > first[0].1.task_mse);
        assert_eq!(snapshot, format!("{study:?}"));
        batch.trial.regime = Regime::Correlated;
        assert!(study.validate(&batch).is_err());
    }

    #[test]
    fn admission_skips_projection_relu_and_readout_before_overflow() {
        let drop = ExactMask::from_retained_indices(UNITS, &[]).unwrap();
        let one = ExactMask::from_retained_indices(UNITS, &[0]).unwrap();
        let (output, work) = predict(&[1.0; UNITS], &[f64::MAX; INPUTS], Some(&drop)).unwrap();
        assert_eq!(output.to_bits(), 0.0f64.to_bits());
        assert_eq!(
            work,
            Work {
                multiplications: 0,
                relus: 0,
                mask_tests: 8
            }
        );
        assert!(predict(&[1.0; UNITS], &[f64::MAX; INPUTS], Some(&one)).is_err());
        assert!(predict(&[f64::NAN; UNITS], &[0.0; INPUTS], Some(&drop)).is_err());
        let wrong = ExactMask::from_retained_indices(2, &[]).unwrap();
        assert!(predict(&[1.0; UNITS], &[0.0; INPUTS], Some(&wrong)).is_err());
    }

    #[test]
    fn all_small_masks_match_an_independent_dense_feature_oracle() {
        let input = [0.25, -0.5, 1.0, -0.25];
        let values = features(&input).unwrap();
        for code in 0..256u16 {
            let kept: Vec<usize> = (0..UNITS).filter(|bit| code & (1u16 << bit) != 0).collect();
            let mask = ExactMask::from_retained_indices(UNITS, &kept).unwrap();
            let mut changed = TEACHER;
            for (unit, weight) in changed.iter_mut().enumerate() {
                if !mask.as_slice()[unit] {
                    *weight = 0.0;
                }
            }
            let (actual, work) = predict(&TEACHER, &input, Some(&mask)).unwrap();
            assert!((actual - dot(&changed, &values).unwrap()).abs() < 1e-12);
            assert_eq!(work.multiplications, u64::try_from(kept.len() * 5).unwrap());
            assert_eq!(work.relus, u64::try_from(kept.len()).unwrap());
        }
    }
}