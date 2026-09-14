//! BL-14 exact numerical integration calibration, not a trained-model result.
//!
//! A fixed integer linear layer is preserved. Existing masks decide which
//! multiplications execute; rejected coefficients are not multiplied by zero.
//! Reference outputs belong to the evaluator, never to predicate extraction.

use std::error::Error;

use booleanlab_core::{
    ExactMask, deterministic_random_mask, mask_from_descending_u64_scores,
    structured_nm_mask_from_u64_scores,
};
use booleanlab_discovery::sparsity_exhaustive_search::propose_exhaustive_rules;
use booleanlab_discovery::sparsity_function_mask::materialize_boolean_function_mask;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const WIDTH: usize = 4;
const RETAINED: usize = 8;
const WEIGHTS: [i64; 16] = [8, -1, 2, -5, 3, -7, 6, -4, -6, 2, -8, 1, -3, 5, 4, -7];

#[derive(Debug, PartialEq, Eq)]
struct LinearRun {
    outputs: Vec<i128>,
    multiplications: u64,
    mask_tests: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Evidence {
    squared_error_sum: u128,
    output_count: u64,
    multiplications: u64,
    mask_tests: u64,
}

fn increment(value: &mut u64, by: u64) -> Result<()> {
    *value = value.checked_add(by).ok_or("operation counter overflow")?;
    Ok(())
}

fn validate_shape(weights: &[i64], width: usize, input: &[i64]) -> Result<()> {
    if width == 0 || weights.is_empty() || !weights.len().is_multiple_of(width) {
        return Err("empty or ragged linear operator".into());
    }
    if input.len() != width {
        return Err("input width does not match the linear operator".into());
    }
    Ok(())
}

// Independent dense oracle: no mask and no candidate execution path.
fn dense_linear(weights: &[i64], width: usize, input: &[i64]) -> Result<Vec<i128>> {
    validate_shape(weights, width, input)?;
    weights
        .chunks_exact(width)
        .map(|row| {
            row.iter().zip(input).try_fold(0i128, |sum, (&w, &x)| {
                // Every i64 * i64 product fits in i128; accumulation may not.
                sum.checked_add(i128::from(w) * i128::from(x))
                    .ok_or_else(|| "dense accumulation overflow".into())
            })
        })
        .collect()
}

fn sparse_linear(
    weights: &[i64],
    width: usize,
    input: &[i64],
    mask: &ExactMask,
) -> Result<LinearRun> {
    validate_shape(weights, width, input)?;
    if mask.as_slice().len() != weights.len() {
        return Err("mask size does not match the linear operator".into());
    }
    let mut run = LinearRun {
        outputs: Vec::with_capacity(weights.len() / width),
        multiplications: 0,
        mask_tests: 0,
    };
    for row_start in (0..weights.len()).step_by(width) {
        let mut sum = 0i128;
        for (column, &x) in input.iter().enumerate() {
            let index = row_start + column;
            increment(&mut run.mask_tests, 1)?;
            if mask.as_slice()[index] {
                increment(&mut run.multiplications, 1)?;
                sum = sum
                    .checked_add(i128::from(weights[index]) * i128::from(x))
                    .ok_or("sparse accumulation overflow")?;
            }
        }
        run.outputs.push(sum);
    }
    Ok(run)
}

fn measure(mask: &ExactMask, inputs: &[Vec<i64>], reference: &[Vec<i128>]) -> Result<Evidence> {
    if inputs.is_empty() || inputs.len() != reference.len() {
        return Err("empty or unmatched calibration batch".into());
    }
    let mut evidence = Evidence {
        squared_error_sum: 0,
        output_count: 0,
        multiplications: 0,
        mask_tests: 0,
    };
    for (input, expected) in inputs.iter().zip(reference) {
        let run = sparse_linear(&WEIGHTS, WIDTH, input, mask)?;
        if run.outputs.len() != expected.len() {
            return Err("reference output shape drift".into());
        }
        increment(&mut evidence.multiplications, run.multiplications)?;
        increment(&mut evidence.mask_tests, run.mask_tests)?;
        for (&actual, &target) in run.outputs.iter().zip(expected) {
            let difference = actual
                .checked_sub(target)
                .ok_or("output difference overflow")?;
            let magnitude = difference.unsigned_abs();
            let squared = magnitude
                .checked_mul(magnitude)
                .ok_or("squared error overflow")?;
            evidence.squared_error_sum = evidence
                .squared_error_sum
                .checked_add(squared)
                .ok_or("error sum overflow")?;
            increment(&mut evidence.output_count, 1)?;
        }
    }
    Ok(evidence)
}

// All 16 sign vectors are explicit development/calibration inputs. No HOLDOUT.
fn calibration_inputs() -> Vec<Vec<i64>> {
    (0..16usize)
        .map(|address| {
            (0..WIDTH)
                .map(|bit| {
                    if address & (1usize << bit) == 0 {
                        -1
                    } else {
                        1
                    }
                })
                .collect()
        })
        .collect()
}

// Fixed observation schema: magnitude >= 5, negative weight, odd row.
// Threshold and predicate order are constants, not fitted to output errors.
fn predicate_rows() -> Vec<Vec<bool>> {
    WEIGHTS
        .iter()
        .enumerate()
        .map(|(index, &weight)| {
            vec![
                weight.unsigned_abs() >= 5,
                weight < 0,
                (index / WIDTH) % 2 == 1,
            ]
        })
        .collect()
}

fn emit(name: &str, mask: &ExactMask, evidence: &Evidence) {
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}",
        mask.cardinality().retained(),
        mask.cardinality().total(),
        evidence.squared_error_sum,
        evidence.output_count,
        evidence.multiplications,
        evidence.mask_tests,
    );
}

fn main() -> Result<()> {
    if std::env::args_os().len() != 1 {
        return Err("this fixed numerical calibration accepts no arguments".into());
    }
    let inputs = calibration_inputs();
    let reference: Vec<Vec<i128>> = inputs
        .iter()
        .map(|input| dense_linear(&WEIGHTS, WIDTH, input))
        .collect::<Result<_>>()?;
    let scores: Vec<u64> = WEIGHTS.iter().map(|weight| weight.unsigned_abs()).collect();
    println!("# schema=bl14.linear-calibration.v1; evidence=EXACT_NUMERICAL_CALIBRATION");
    println!(
        "# Fixed untrained 4x4 integer layer; 16 sign inputs; no HOLDOUT or model-quality claim."
    );
    println!(
        "# All sparse controls retain exactly 8/16 coefficients. Dense is a separate 16/16 reference."
    );
    println!("# Counters are reference operations, not hardware instructions or elapsed time.");
    println!(
        "# Mask construction/search, allocations and predicate extraction are excluded from counters."
    );
    println!("# Boolean masks are static; 16 predicate rows evaluated once per proposed rule.");
    println!(
        "policy\tretained\ttotal\tsquared_error_sum\toutput_count\tmultiplications\tmask_tests"
    );
    println!("dense\t16\t16\t0\t64\t256\t0");
    let mut baselines = vec![
        (
            "magnitude".to_owned(),
            mask_from_descending_u64_scores(&scores, RETAINED)?,
        ),
        (
            "magnitude_2_4".to_owned(),
            structured_nm_mask_from_u64_scores(&scores, 2, 4)?,
        ),
    ];
    for seed in 0..4u64 {
        baselines.push((
            format!("random_{seed}"),
            deterministic_random_mask(16, RETAINED, seed)?,
        ));
    }
    for (name, mask) in &baselines {
        if mask.cardinality().retained() != RETAINED {
            return Err("baseline density mismatch".into());
        }
        emit(name, mask, &measure(mask, &inputs, &reference)?);
    }
    let owned = predicate_rows();
    let rows: Vec<&[bool]> = owned.iter().map(Vec::as_slice).collect();
    let proposals = propose_exhaustive_rules(3)?;
    let mut eligible = 0u64;
    let mut best_error = u128::MAX;
    let mut best_codes = Vec::new();
    for proposal in &proposals {
        let mask = materialize_boolean_function_mask(&proposal.function, &rows)?;
        if mask.cardinality().retained() != RETAINED {
            continue;
        }
        increment(&mut eligible, 1)?;
        let evidence = measure(&mask, &inputs, &reference)?;
        emit(&proposal.proposal_id, &mask, &evidence);
        if evidence.squared_error_sum < best_error {
            best_error = evidence.squared_error_sum;
            best_codes.clear();
        }
        if evidence.squared_error_sum == best_error {
            best_codes.push(proposal.truth_table_code);
        }
    }
    if eligible == 0 {
        return Err("no exact-density Boolean proposal".into());
    }
    println!(
        "# searched={}; eligible={eligible}; best_error={best_error}; all_best_codes={best_codes:?}",
        proposals.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_small_mask_matches_an_independent_masked_weight_oracle() {
        let weights = [2, -3, 5, 7];
        let input = [11, -13];
        for code in 0..16usize {
            let retained: Vec<usize> = (0..4).filter(|bit| code & (1usize << bit) != 0).collect();
            let mask = ExactMask::from_retained_indices(4, &retained).unwrap();
            let modified: Vec<i64> = weights
                .iter()
                .enumerate()
                .map(|(i, &w)| if code & (1usize << i) != 0 { w } else { 0 })
                .collect();
            let run = sparse_linear(&weights, 2, &input, &mask).unwrap();
            assert_eq!(run.outputs, dense_linear(&modified, 2, &input).unwrap());
            assert_eq!(run.multiplications, u64::try_from(retained.len()).unwrap());
            assert_eq!(run.mask_tests, 4);
        }
    }

    #[test]
    fn rejected_products_are_not_eagerly_accumulated() {
        let weights = [i64::MIN; 3];
        let input = [i64::MIN; 3];
        assert!(dense_linear(&weights, 3, &input).is_err());
        let drop = ExactMask::from_retained_indices(3, &[]).unwrap();
        let run = sparse_linear(&weights, 3, &input, &drop).unwrap();
        assert_eq!(run.outputs, vec![0]);
        assert_eq!(run.multiplications, 0);
        let keep_one = ExactMask::from_retained_indices(3, &[0]).unwrap();
        let one = sparse_linear(&weights, 3, &input, &keep_one).unwrap();
        assert_eq!(one.outputs, vec![1i128 << 126]);
        assert_eq!(one.multiplications, 1);
    }

    #[test]
    fn all_keep_reproduces_dense_and_all_drop_has_zero_products() {
        let inputs = calibration_inputs();
        let reference: Vec<Vec<i128>> = inputs
            .iter()
            .map(|input| dense_linear(&WEIGHTS, WIDTH, input).unwrap())
            .collect();
        let keep = ExactMask::from_retained_indices(16, &(0..16).collect::<Vec<_>>()).unwrap();
        let kept = measure(&keep, &inputs, &reference).unwrap();
        assert_eq!(kept.squared_error_sum, 0);
        assert_eq!(kept.multiplications, 256);
        assert_eq!(kept.mask_tests, 256);
        let drop = ExactMask::from_retained_indices(16, &[]).unwrap();
        let dropped = measure(&drop, &inputs, &reference).unwrap();
        assert_eq!(dropped.multiplications, 0);
        assert_eq!(dropped.mask_tests, 256);
    }

    #[test]
    fn exact_density_boolean_search_matches_magnitude_without_claiming_superiority() {
        let inputs = calibration_inputs();
        let reference: Vec<Vec<i128>> = inputs
            .iter()
            .map(|input| dense_linear(&WEIGHTS, WIDTH, input).unwrap())
            .collect();
        let owned = predicate_rows();
        let rows: Vec<&[bool]> = owned.iter().map(Vec::as_slice).collect();
        let mut evidence = Vec::new();
        for proposal in propose_exhaustive_rules(3).unwrap() {
            let mask = materialize_boolean_function_mask(&proposal.function, &rows).unwrap();
            if mask.cardinality().retained() == RETAINED {
                let measured = measure(&mask, &inputs, &reference).unwrap();
                // Independent sign-cube identity: cross terms cancel exactly.
                let oracle: u128 = WEIGHTS
                    .iter()
                    .zip(mask.as_slice())
                    .filter(|(_, keep)| !**keep)
                    .map(|(&w, _)| u128::from(w.unsigned_abs()).pow(2))
                    .sum::<u128>()
                    * 16;
                assert_eq!(measured.squared_error_sum, oracle);
                assert_eq!(measured.multiplications, 128);
                assert_eq!(measured.mask_tests, 256);
                evidence.push((measured.squared_error_sum, proposal.truth_table_code));
            }
        }
        assert_eq!(evidence.len(), 34);
        let minimum = evidence.iter().map(|row| row.0).min().unwrap();
        let winners: Vec<u64> = evidence
            .iter()
            .filter(|row| row.0 == minimum)
            .map(|row| row.1)
            .collect();
        assert_eq!(minimum, 960);
        assert_eq!(winners, vec![170]);
        let scores: Vec<u64> = WEIGHTS.iter().map(|w| w.unsigned_abs()).collect();
        let magnitude = mask_from_descending_u64_scores(&scores, RETAINED).unwrap();
        let structured = structured_nm_mask_from_u64_scores(&scores, 2, 4).unwrap();
        assert_eq!(magnitude, structured);
        assert_eq!(
            measure(&magnitude, &inputs, &reference)
                .unwrap()
                .squared_error_sum,
            minimum
        );
    }

    #[test]
    fn malformed_shapes_and_reference_batches_fail_closed() {
        let mask = ExactMask::from_retained_indices(4, &[0, 1]).unwrap();
        assert!(sparse_linear(&[1; 4], 0, &[], &mask).is_err());
        assert!(sparse_linear(&[1; 3], 2, &[1; 2], &mask).is_err());
        assert!(sparse_linear(&[1; 4], 2, &[1], &mask).is_err());
        assert!(sparse_linear(&[1; 2], 2, &[1; 2], &mask).is_err());
        assert!(measure(&mask, &[], &[]).is_err());
    }
}
