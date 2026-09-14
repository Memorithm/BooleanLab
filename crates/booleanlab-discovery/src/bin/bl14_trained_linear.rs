//! BL-14.1.2 synthetic trained-model development pilot, not final confirmation.
//! The numerical model remains intact; masks only control retained coefficients.

use std::cmp::Ordering;
use std::error::Error;

use booleanlab_core::{
    ExactMask, deterministic_random_keys, deterministic_random_mask,
    mask_from_descending_u64_scores, structured_nm_mask_from_u64_scores,
};
use booleanlab_discovery::BooleanFunction;
use booleanlab_discovery::sparsity_exhaustive_search::propose_exhaustive_rules;
use booleanlab_discovery::sparsity_function_mask::materialize_boolean_function_mask;

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const WIDTH: usize = 8;
const KEEP: usize = 4;
const DATA_SEED: u64 = 0xB114_0012;
const SCALES: [f64; WIDTH] = [1.0, 2.0, 1.0, 0.5, 2.0, 0.25, 1.0, 0.5];
const TEACHER: [f64; WIDTH] = [3.0, -1.0, 2.0, -4.0, 0.5, 1.0, -2.0, 1.5];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Partition {
    Train,
    Search,
    Validation,
}

#[derive(Clone, Debug)]
struct Example {
    id: usize,
    input: [f64; WIDTH],
    target: f64,
}

#[derive(Clone, Debug)]
struct Batch {
    partition: Partition,
    examples: Vec<Example>,
}

#[derive(Clone, Debug)]
struct Model {
    weights: [f64; WIDTH],
}

#[derive(Clone, Debug, PartialEq)]
struct Metrics {
    task_mse: f64,
    reconstruction_mse: f64,
    examples: u64,
    multiplications: u64,
    mask_tests: u64,
}

#[derive(Debug)]
struct ScreenedRule {
    id: String,
    code: u64,
    function: BooleanFunction,
    mask: ExactMask,
    search: Metrics,
}

// No public mutation or refitting path: validation borrows this frozen object.
#[derive(Debug)]
struct FrozenStudy {
    model: Model,
    predicate_rows: Vec<Vec<bool>>,
    baselines: Vec<(String, ExactMask)>,
    screened: Vec<ScreenedRule>,
    selected: Vec<usize>,
}

fn finite(value: f64) -> Result<f64> {
    if !value.is_finite() {
        return Err("non-finite pilot input, arithmetic or metric".into());
    }
    Ok(value)
}

fn increment(value: &mut u64, by: u64) -> Result<()> {
    *value = value.checked_add(by).ok_or("operation counter overflow")?;
    Ok(())
}

fn denominator(count: usize) -> Result<f64> {
    if count == 0 {
        return Err("empty batch".into());
    }
    Ok(f64::from(u32::try_from(count)?))
}

fn dense_dot(weights: &[f64; WIDTH], input: &[f64; WIDTH]) -> Result<f64> {
    weights.iter().zip(input).try_fold(0.0, |sum, (&w, &x)| {
        finite(sum + finite(finite(w)? * finite(x)?)?)
    })
}

fn generate(partition: Partition) -> Result<Batch> {
    let (first, end) = match partition {
        Partition::Train => (0, 128),
        Partition::Search => (128, 192),
        Partition::Validation => (192, 256),
    };
    let keys = deterministic_random_keys(256 * WIDTH, DATA_SEED)?;
    let mut examples = Vec::with_capacity(end - first);
    for id in first..end {
        let mut input = [0.0; WIDTH];
        for (column, value) in input.iter_mut().enumerate() {
            let integer = u32::try_from(keys[id * WIDTH + column] % 17)?;
            *value = (f64::from(integer) - 8.0) / 8.0 * SCALES[column];
        }
        examples.push(Example {
            id,
            target: dense_dot(&TEACHER, &input)?,
            input,
        });
    }
    Ok(Batch {
        partition,
        examples,
    })
}

fn require_partition(batch: &Batch, expected: Partition) -> Result<()> {
    if batch.partition != expected {
        return Err("wrong development partition".into());
    }
    denominator(batch.examples.len())?;
    Ok(())
}

fn fit(train: &Batch) -> Result<Model> {
    require_partition(train, Partition::Train)?;
    let count = denominator(train.examples.len())?;
    let mut model = Model {
        weights: [0.0; WIDTH],
    };
    for _ in 0..256 {
        let mut gradient = [0.0; WIDTH];
        for example in &train.examples {
            let error = finite(
                dense_dot(&model.weights, &example.input)? - finite(example.target)?,
            )?;
            for (derivative, &input) in gradient.iter_mut().zip(&example.input) {
                *derivative = finite(*derivative + finite(2.0 * error * input)?)?;
            }
        }
        for (weight, derivative) in model.weights.iter_mut().zip(gradient) {
            *weight = finite(*weight - 0.125 * derivative / count)?;
        }
    }
    Ok(model)
}

// Preserve exact order of nonnegative finite binary64 magnitudes, including
// subnormals. No lossy integer quantization; both signed zeros get key zero.
fn magnitude_keys(values: &[f64]) -> Result<Vec<u64>> {
    if values.is_empty() {
        return Err("empty magnitude input".into());
    }
    values
        .iter()
        .map(|&value| Ok(finite(value)?.abs().to_bits()))
        .collect()
}

fn training_energy(train: &Batch) -> Result<[f64; WIDTH]> {
    require_partition(train, Partition::Train)?;
    let mut energy = [0.0; WIDTH];
    for example in &train.examples {
        for (sum, &input) in energy.iter_mut().zip(&example.input) {
            *sum = finite(*sum + finite(input * input)?)?;
        }
    }
    let count = denominator(train.examples.len())?;
    for value in &mut energy {
        *value = finite(*value / count)?;
    }
    Ok(energy)
}

fn masked_dot(
    weights: &[f64; WIDTH],
    input: &[f64; WIDTH],
    mask: &ExactMask,
) -> Result<(f64, u64, u64)> {
    if mask.as_slice().len() != WIDTH {
        return Err("mask width mismatch".into());
    }
    // Invalid observations are rejected even for an all-drop mask.
    for &value in weights.iter().chain(input.iter()) {
        finite(value)?;
    }
    let mut sum = 0.0;
    let mut multiplications = 0;
    let mut tests = 0;
    for (index, &keep) in mask.as_slice().iter().enumerate() {
        increment(&mut tests, 1)?;
        if keep {
            increment(&mut multiplications, 1)?;
            sum = finite(sum + finite(weights[index] * input[index])?)?;
        }
    }
    Ok((sum, multiplications, tests))
}

fn score(model: &Model, batch: &Batch, mask: Option<&ExactMask>) -> Result<Metrics> {
    let count = denominator(batch.examples.len())?;
    let mut result = Metrics {
        task_mse: 0.0,
        reconstruction_mse: 0.0,
        examples: 0,
        multiplications: 0,
        mask_tests: 0,
    };
    for example in &batch.examples {
        let reference = dense_dot(&model.weights, &example.input)?;
        let (actual, multiplications, tests) = match mask {
            Some(mask) => masked_dot(&model.weights, &example.input, mask)?,
            None => (reference, u64::try_from(WIDTH)?, 0),
        };
        let error = finite(actual - finite(example.target)?)?;
        let reconstruction = finite(actual - reference)?;
        result.task_mse = finite(result.task_mse + finite(error * error)?)?;
        result.reconstruction_mse = finite(
            result.reconstruction_mse + finite(reconstruction * reconstruction)?,
        )?;
        increment(&mut result.examples, 1)?;
        increment(&mut result.multiplications, multiplications)?;
        increment(&mut result.mask_tests, tests)?;
    }
    result.task_mse = finite(result.task_mse / count)?;
    result.reconstruction_mse = finite(result.reconstruction_mse / count)?;
    Ok(result)
}

fn fixed_baselines(model: &Model, energy: &[f64; WIDTH]) -> Result<Vec<(String, ExactMask)>> {
    let keys = magnitude_keys(&model.weights)?;
    let saliency: Vec<f64> = model
        .weights
        .iter()
        .zip(energy)
        .map(|(&weight, &input_energy)| finite(weight * weight * input_energy))
        .collect::<Result<_>>()?;
    let mut controls = vec![
        (
            "magnitude".to_owned(),
            mask_from_descending_u64_scores(&keys, KEEP)?,
        ),
        (
            "magnitude_2_4".to_owned(),
            structured_nm_mask_from_u64_scores(&keys, 2, 4)?,
        ),
        (
            "activation_energy".to_owned(),
            mask_from_descending_u64_scores(&magnitude_keys(&saliency)?, KEEP)?,
        ),
    ];
    for seed in 0..4 {
        controls.push((
            format!("random_{seed}"),
            deterministic_random_mask(WIDTH, KEEP, seed)?,
        ));
    }
    Ok(controls)
}

fn freeze(model: Model, train: &Batch, search: &Batch) -> Result<FrozenStudy> {
    require_partition(train, Partition::Train)?;
    require_partition(search, Partition::Search)?;
    if train.examples.iter().any(|left| {
        search.examples.iter().any(|right| left.id == right.id)
    }) {
        return Err("TRAIN/SEARCH instance overlap".into());
    }
    let energy = training_energy(train)?;
    let baselines = fixed_baselines(&model, &energy)?;
    let energetic = mask_from_descending_u64_scores(&magnitude_keys(&energy)?, KEEP)?;
    let predicate_rows: Vec<Vec<bool>> = (0..WIDTH)
        .map(|index| {
            vec![
                baselines[0].1.as_slice()[index],
                model.weights[index] < 0.0,
                energetic.as_slice()[index],
            ]
        })
        .collect();
    let rows: Vec<&[bool]> = predicate_rows.iter().map(Vec::as_slice).collect();
    let mut screened = Vec::new();
    for proposal in propose_exhaustive_rules(3)? {
        let mask = materialize_boolean_function_mask(&proposal.function, &rows)?;
        if mask.cardinality().retained() == KEEP {
            screened.push(ScreenedRule {
                search: score(&model, search, Some(&mask))?,
                id: proposal.proposal_id,
                code: proposal.truth_table_code,
                function: proposal.function,
                mask,
            });
        }
    }
    let best = screened
        .iter()
        .map(|rule| rule.search.task_mse)
        .min_by(f64::total_cmp)
        .ok_or("no exact-density Boolean rule")?;
    let selected = screened
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            (rule.search.task_mse.total_cmp(&best) == Ordering::Equal).then_some(index)
        })
        .collect();
    Ok(FrozenStudy {
        model,
        predicate_rows,
        baselines,
        screened,
        selected,
    })
}

impl FrozenStudy {
    fn validate(&self, validation: &Batch) -> Result<Vec<(String, usize, Metrics)>> {
        require_partition(validation, Partition::Validation)?;
        if validation.examples.iter().any(|example| example.id < 192 || example.id >= 256) {
            return Err("validation instance outside declared partition".into());
        }
        let mut rows = vec![(
            "dense".to_owned(),
            WIDTH,
            score(&self.model, validation, None)?,
        )];
        for (id, mask) in &self.baselines {
            rows.push((id.clone(), KEEP, score(&self.model, validation, Some(mask))?));
        }
        let predicates: Vec<&[bool]> = self.predicate_rows.iter().map(Vec::as_slice).collect();
        for &index in &self.selected {
            let rule = &self.screened[index];
            let rematerialized = materialize_boolean_function_mask(&rule.function, &predicates)?;
            if rematerialized != rule.mask {
                return Err("frozen rule/mask semantics changed".into());
            }
            rows.push((
                rule.id.clone(),
                KEEP,
                score(&self.model, validation, Some(&rule.mask))?,
            ));
        }
        Ok(rows)
    }
}

fn emit(stage: &str, id: &str, retained: usize, metrics: &Metrics) {
    println!(
        "{stage}\t{id}\t{retained}\t{}\t{:.17e}\t{:.17e}\t{}\t{}",
        metrics.examples,
        metrics.task_mse,
        metrics.reconstruction_mse,
        metrics.multiplications,
        metrics.mask_tests,
    );
}

fn main() -> Result<()> {
    if std::env::args_os().len() != 1 {
        return Err("fixed development pilot accepts no arguments".into());
    }
    let train = generate(Partition::Train)?;
    let search = generate(Partition::Search)?;
    let model = fit(&train)?;
    println!("# schema=bl14.trained-linear.v1; evidence=NUMERICAL_DEVELOPMENT");
    println!("# fixed synthetic regression; 8 coefficients; no final holdout or hardware claim");
    println!("# inference counters exclude training/search, scoring and the dense comparison oracle");
    println!("# allocation, metadata, ranking, predicates and memory traffic are not measured");
    println!("stage\tpolicy\tretained\texamples\ttask_mse\treconstruction_mse\tmuls\tmask_tests");
    let zero = Model {
        weights: [0.0; WIDTH],
    };
    emit("TRAIN", "before_fit", WIDTH, &score(&zero, &train, None)?);
    emit("TRAIN", "after_fit", WIDTH, &score(&model, &train, None)?);
    let frozen = freeze(model, &train, &search)?;
    let weight_bits: Vec<u64> = frozen.model.weights.iter().map(|w| w.to_bits()).collect();
    println!("# learned_weight_bits={weight_bits:?}");
    println!("# resolved_predicates={:?}", frozen.predicate_rows);
    emit("SEARCH", "dense", WIDTH, &score(&frozen.model, &search, None)?);
    for (id, mask) in &frozen.baselines {
        emit("SEARCH", id, KEEP, &score(&frozen.model, &search, Some(mask))?);
    }
    for rule in &frozen.screened {
        emit("SEARCH", &rule.id, KEEP, &rule.search);
    }
    let codes: Vec<u64> = frozen.selected.iter().map(|&i| frozen.screened[i].code).collect();
    println!("# frozen_selected_codes={codes:?}; no validation-based re-ranking");
    let validation = generate(Partition::Validation)?;
    for (id, retained, metrics) in frozen.validate(&validation)? {
        emit("VALIDATION", &id, retained, &metrics);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn partitions_have_disjoint_ids_and_inputs() {
        let mut ids = BTreeSet::new();
        let mut inputs = BTreeSet::new();
        for partition in [Partition::Train, Partition::Search, Partition::Validation] {
            for example in generate(partition).unwrap().examples {
                assert!(ids.insert(example.id));
                assert!(inputs.insert(example.input.map(f64::to_bits)));
            }
        }
        assert_eq!(ids.len(), 256);
    }

    #[test]
    fn actual_training_reduces_loss_and_uses_training_targets() {
        let mut train = generate(Partition::Train).unwrap();
        let zero = Model {
            weights: [0.0; WIDTH],
        };
        let before = score(&zero, &train, None).unwrap().task_mse;
        let learned = fit(&train).unwrap();
        assert!(score(&learned, &train, None).unwrap().task_mse < before / 100.0);
        for example in &mut train.examples {
            example.target = -example.target;
        }
        let reversed = fit(&train).unwrap();
        for (left, right) in learned.weights.iter().zip(reversed.weights) {
            assert!((left + right).abs() < 1e-12);
        }
        assert!(fit(&generate(Partition::Validation).unwrap()).is_err());
    }

    #[test]
    fn selection_preserves_exact_density_and_all_search_ties() {
        let train = generate(Partition::Train).unwrap();
        let search = generate(Partition::Search).unwrap();
        let frozen = freeze(fit(&train).unwrap(), &train, &search).unwrap();
        assert_eq!(frozen.baselines.len(), 7);
        assert!(!frozen.selected.is_empty());
        for (_, mask) in &frozen.baselines {
            assert_eq!(mask.cardinality().retained(), KEEP);
        }
        let best = frozen.screened[frozen.selected[0]].search.task_mse;
        for (index, rule) in frozen.screened.iter().enumerate() {
            assert_eq!(rule.mask.cardinality().retained(), KEEP);
            assert_eq!(rule.search.multiplications, 256);
            assert_eq!(rule.search.mask_tests, 512);
            assert_eq!(
                frozen.selected.contains(&index),
                rule.search.task_mse.total_cmp(&best) == Ordering::Equal
            );
        }
    }

    #[test]
    fn validation_cannot_change_model_rule_or_search_selection() {
        let train = generate(Partition::Train).unwrap();
        let search = generate(Partition::Search).unwrap();
        let frozen = freeze(fit(&train).unwrap(), &train, &search).unwrap();
        let before = format!("{frozen:?}");
        let mut validation = generate(Partition::Validation).unwrap();
        let first = frozen.validate(&validation).unwrap();
        assert_eq!(first, frozen.validate(&validation).unwrap());
        for example in &mut validation.examples {
            example.target += 100.0;
        }
        let changed = frozen.validate(&validation).unwrap();
        assert!(changed[0].2.task_mse > first[0].2.task_mse);
        assert_eq!(before, format!("{frozen:?}"));
        assert!(frozen.validate(&search).is_err());
        validation.examples[0].id = 0;
        assert!(frozen.validate(&validation).is_err());
    }

    #[test]
    fn dropped_work_is_not_multiplied_and_invalid_input_fails_closed() {
        let dropped = ExactMask::from_retained_indices(WIDTH, &[]).unwrap();
        let retained = ExactMask::from_retained_indices(WIDTH, &[0]).unwrap();
        let weights = [f64::MAX; WIDTH];
        let input = [2.0; WIDTH];
        let (result, muls, checks) = masked_dot(&weights, &input, &dropped).unwrap();
        assert_eq!(result.to_bits(), 0.0f64.to_bits());
        assert_eq!((muls, checks), (0, 8));
        assert!(masked_dot(&weights, &input, &retained).is_err());
        assert!(masked_dot(&weights, &[f64::NAN; WIDTH], &dropped).is_err());
        let wrong = ExactMask::from_retained_indices(2, &[]).unwrap();
        assert!(masked_dot(&weights, &input, &wrong).is_err());
    }

    #[test]
    fn finite_magnitude_bridge_preserves_zeros_subnormals_and_exact_order() {
        let values = [-0.0, 0.0, -f64::from_bits(1), 1.0, -f64::MAX];
        let keys = magnitude_keys(&values).unwrap();
        assert_eq!(keys[0], keys[1]);
        assert!(keys[1..].windows(2).all(|pair| pair[0] < pair[1]));
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(magnitude_keys(&[bad]).is_err());
        }
        assert!(magnitude_keys(&[]).is_err());
    }

    #[test]
    fn empty_or_nonfinite_training_batches_are_rejected() {
        let empty = Batch {
            partition: Partition::Train,
            examples: Vec::new(),
        };
        assert!(fit(&empty).is_err());
        let mut train = generate(Partition::Train).unwrap();
        train.examples[0].target = f64::NAN;
        assert!(fit(&train).is_err());
    }
}
