//! Exact SEARCH-only comparison of finite Boolean-rule search methods.
//!
//! Full-population mask fitting answers a different question from finding the
//! first exact rule. A short-circuit linear search therefore supplies the
//! matched first-solution baseline. Counts below are logical checks, not CPU
//! instructions, elapsed time, or evidence of model-quality improvement.

use std::error::Error;

use booleanlab_discovery::sparsity_cegis_search::{
    CegisSearchError, CegisSearchResult, search_cegis,
};
use booleanlab_discovery::sparsity_exhaustive_fit::{
    best_mismatch_indices, evaluate_exhaustive_mask_fit,
};
use booleanlab_discovery::sparsity_exhaustive_search::{
    ExhaustiveRuleProposal, propose_exhaustive_rules,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq, Eq)]
struct LinearSearch {
    selected_index: Option<usize>,
    candidate_checks: u64,
    row_checks: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Comparison {
    candidate_count: usize,
    example_count: usize,
    minimum_mismatches: u64,
    best_codes: Vec<u64>,
    exhaustive_mask_decisions: u64,
    linear: LinearSearch,
    cegis: Option<CegisSearchResult>,
}

fn checked_increment(value: &mut u64) -> Result<()> {
    *value = value.checked_add(1).ok_or("logical check count overflow")?;
    Ok(())
}

// Inputs have already passed the shared exhaustive materialization validator.
// This oracle deliberately does not reuse the CEGIS evaluator.
fn linear_first_exact(
    candidates: &[ExhaustiveRuleProposal],
    rows: &[&[bool]],
    target: &[bool],
) -> Result<LinearSearch> {
    let mut result = LinearSearch {
        selected_index: None,
        candidate_checks: 0,
        row_checks: 0,
    };
    for (index, candidate) in candidates.iter().enumerate() {
        checked_increment(&mut result.candidate_checks)?;
        let mut exact = true;
        for (predicates, expected) in rows.iter().zip(target) {
            checked_increment(&mut result.row_checks)?;
            let mut address = 0usize;
            for (bit, value) in predicates.iter().enumerate() {
                if *value {
                    address |= 1usize << bit;
                }
            }
            if (candidate.function.truth_table()[address] != 0) != *expected {
                exact = false;
                break;
            }
        }
        if exact {
            result.selected_index = Some(index);
            break;
        }
    }
    Ok(result)
}

fn compare(input_bits: u32, rows: &[&[bool]], target: &[bool]) -> Result<Comparison> {
    // The full fit retains ALL best-fit ties, including nonzero-error optima.
    // Its API generates its own population. The second population below is
    // shared by linear and CEGIS search; both generation passes are disclosed.
    let fit = evaluate_exhaustive_mask_fit(input_bits, rows, target)?;
    let best = best_mismatch_indices(&fit)?;
    let candidates = propose_exhaustive_rules(input_bits)?;
    if fit.len() != candidates.len() {
        return Err("candidate population drift".into());
    }
    let linear = linear_first_exact(&candidates, rows, target)?;
    let cegis = match search_cegis(&candidates, rows, target) {
        Ok(result) => Some(result),
        Err(CegisSearchError::NoConsistentCandidate) => None,
        Err(error) => return Err(error.into()),
    };
    let first_exact = fit.iter().position(|row| row.mismatches == 0);
    if linear.selected_index != first_exact
        || cegis.as_ref().map(|row| row.candidate_index) != first_exact
    {
        return Err("search methods disagree on first exact solution".into());
    }
    if let Some(result) = &cegis {
        let proposal = &candidates[result.candidate_index];
        if proposal.proposal_id != result.candidate_id
            || proposal.truth_table_code != fit[result.candidate_index].truth_table_code
        {
            return Err("selected rule identity drift".into());
        }
    }
    let exhaustive_mask_decisions = u64::try_from(candidates.len())?
        .checked_mul(u64::try_from(rows.len())?)
        .ok_or("full-population logical check count overflow")?;
    Ok(Comparison {
        candidate_count: candidates.len(),
        example_count: rows.len(),
        minimum_mismatches: fit[best[0]].mismatches,
        best_codes: best
            .iter()
            .map(|&index| fit[index].truth_table_code)
            .collect(),
        exhaustive_mask_decisions,
        linear,
        cegis,
    })
}

fn emit(name: &str, result: &Comparison) -> Result<()> {
    let (status, synthesis, verification, total) = match &result.cegis {
        Some(row) => (
            "EXACT_ON_SUPPLIED_ROWS",
            row.synthesis_row_checks.to_string(),
            row.verification_row_checks.to_string(),
            row.synthesis_row_checks
                .checked_add(row.verification_row_checks)
                .ok_or("CEGIS total logical checks overflow")?
                .to_string(),
        ),
        // The existing CEGIS error API does not return failed-run work counts.
        // Unavailable counters must not silently become measured zeroes.
        None => (
            "NO_EXACT_SOLUTION",
            "NA".to_owned(),
            "NA".to_owned(),
            "NA".to_owned(),
        ),
    };
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{synthesis}\t{verification}\t{total}\t{status}",
        result.candidate_count,
        result.example_count,
        result.minimum_mismatches,
        result.best_codes.len(),
        result.exhaustive_mask_decisions,
        result.linear.row_checks,
    );
    Ok(())
}

fn main() -> Result<()> {
    if std::env::args_os().len() != 1 {
        return Err("this fixed calibration accepts no arguments".into());
    }
    println!("# schema=bl14.search-comparison.v1; phase=SEARCH; evidence=EXACT_CALIBRATION");
    println!("# Counts are logical rule/row checks, NOT hardware time or model quality.");
    println!(
        "# population_generation_passes=2; generation, allocation and metadata costs excluded"
    );
    println!("# CEGIS first-solution counts are comparable to linear first-solution counts.");
    println!("# Full fitting additionally reports ALL optimal ties; missing counters are NA.");
    println!(
        "case\tcandidates\trows\tmin_errors\tbest_ties\tfull_fit_rows\tlinear_rows\tcegis_synthesis_rows\tcegis_verifier_rows\tcegis_total_rows\tstatus"
    );
    let rows: [&[bool]; 4] = [
        &[false, false],
        &[true, false],
        &[false, true],
        &[true, true],
    ];
    for (name, target) in [
        ("xor", [false, true, true, false]),
        ("and", [false, false, false, true]),
        ("all_keep", [true; 4]),
    ] {
        emit(name, &compare(2, &rows, &target)?)?;
    }
    emit(
        "partial_xor",
        &compare(2, &rows[..3], &[false, true, true])?,
    )?;
    let repeated: [&[bool]; 2] = [&[false, false], &[false, false]];
    emit(
        "contradictory_labels",
        &compare(2, &repeated, &[false, true])?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_rows(bits: u32) -> Vec<Vec<bool>> {
        (0..(1usize << bits))
            .map(|row| (0..bits).map(|bit| row & (1usize << bit) != 0).collect())
            .collect()
    }

    #[test]
    fn xor_counts_include_verification_and_expose_negative_matched_comparison() {
        let owned = full_rows(2);
        let rows: Vec<&[bool]> = owned.iter().map(Vec::as_slice).collect();
        let result = compare(2, &rows, &[false, true, true, false]).unwrap();
        assert_eq!(result.best_codes, vec![6]);
        assert_eq!(result.exhaustive_mask_decisions, 64);
        assert_eq!(result.linear.candidate_checks, 7);
        assert_eq!(result.linear.row_checks, 14);
        let cegis = result.cegis.unwrap();
        assert_eq!(cegis.counterexample_rows, vec![1, 2]);
        assert_eq!(cegis.synthesis_candidate_checks, 11);
        assert_eq!(cegis.synthesis_row_checks, 13);
        assert_eq!(cegis.verification_row_checks, 9);
        assert!(cegis.synthesis_row_checks + cegis.verification_row_checks > 14);
    }

    #[test]
    fn and_control_preserves_the_opposite_logical_check_outcome() {
        let owned = full_rows(2);
        let rows: Vec<&[bool]> = owned.iter().map(Vec::as_slice).collect();
        let result = compare(2, &rows, &[false, false, false, true]).unwrap();
        assert_eq!(result.linear.row_checks, 19);
        let cegis = result.cegis.unwrap();
        assert_eq!(
            cegis.synthesis_row_checks + cegis.verification_row_checks,
            17
        );
    }

    #[test]
    fn every_three_bit_target_agrees_with_independent_full_population_oracle() {
        let owned = full_rows(3);
        let rows: Vec<&[bool]> = owned.iter().map(Vec::as_slice).collect();
        for code in 0..256u64 {
            let target: Vec<bool> = (0..8).map(|row| code & (1u64 << row) != 0).collect();
            let result = compare(3, &rows, &target).unwrap();
            assert_eq!(result.minimum_mismatches, 0);
            assert_eq!(result.best_codes, vec![code]);
            assert!(result.cegis.is_some());
        }
    }

    #[test]
    fn partial_coverage_retains_both_exact_ties_without_uniqueness_claim() {
        let rows: [&[bool]; 3] = [&[false, false], &[true, false], &[false, true]];
        let result = compare(2, &rows, &[false, true, true]).unwrap();
        assert_eq!(result.best_codes, vec![6, 14]);
        assert_eq!(result.cegis.unwrap().candidate_index, 6);
    }

    #[test]
    fn contradictory_labels_are_not_reported_as_an_exact_solution() {
        let rows: [&[bool]; 2] = [&[false, false], &[false, false]];
        let result = compare(2, &rows, &[false, true]).unwrap();
        assert_eq!(result.minimum_mismatches, 1);
        assert_eq!(result.best_codes.len(), 16);
        assert_eq!(result.linear.selected_index, None);
        assert!(result.cegis.is_none());
    }

    #[test]
    fn malformed_inputs_fail_closed() {
        assert!(compare(2, &[], &[]).is_err());
        assert!(compare(5, &[&[false; 5]], &[false]).is_err());
        assert!(compare(2, &[&[false]], &[false]).is_err());
        assert!(compare(2, &[&[false, false]], &[false, true]).is_err());
    }
}
