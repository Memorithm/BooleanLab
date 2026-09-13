use std::collections::BTreeMap;

use booleanlab_discovery::BooleanFunction;
use booleanlab_discovery::baseline::{BaselineConfig, BaselineSummary, run_boolean_baseline};
use booleanlab_discovery::equivalence_screen::{
    AffineInvariantScreen, ScreenedEquivalence, screen_affine_invariants, screen_full_baseline,
};
use booleanlab_discovery::sedenion::control_component_functions;

const MAX_EXHAUSTIVE_PERMUTATION_BITS: u32 = 8;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let baseline = run_boolean_baseline(BaselineConfig::bl13_reference())?;
    let functions = control_component_functions()?;
    let (exact_index, complement_index) = build_reference_indexes(&baseline.population);
    let reference_functions = baseline
        .population
        .iter()
        .map(|record| &record.function)
        .cloned()
        .collect::<Vec<_>>();
    let screens = Screens {
        baseline: &baseline,
        reference_functions: &reference_functions,
        exact_index: &exact_index,
        complement_index: &complement_index,
    };
    let mut counts = ScreenCounts::default();

    println!(
        "coordinate\tstatus\treference_index\tpermutation\taffine_screen\taffine_compatible_references"
    );
    for (coordinate, function) in functions.iter().enumerate() {
        screen_coordinate(coordinate, function, &screens, &mut counts)?;
    }

    println!(
        "summary\tbaseline_unique={}\texact_matches={}\tcomplement_matches={}\tinput_permutation_matches={}\tinput_permutation_complement_matches={}\taffine_excluded={}\taffine_inconclusive={}\taffine_compatible_references={}\tnot_found={}\tnovelty_claim=false",
        baseline.unique_functions,
        counts.exact_matches,
        counts.complement_matches,
        counts.permutation_matches,
        counts.permutation_complement_matches,
        counts.affine_excluded,
        counts.affine_inconclusive,
        counts.affine_compatible_references,
        counts.not_found,
    );
    Ok(())
}

#[derive(Default)]
struct ScreenCounts {
    exact_matches: usize,
    complement_matches: usize,
    permutation_matches: usize,
    permutation_complement_matches: usize,
    affine_excluded: usize,
    affine_inconclusive: usize,
    affine_compatible_references: usize,
    not_found: usize,
}

struct Screens<'a> {
    baseline: &'a BaselineSummary,
    reference_functions: &'a [BooleanFunction],
    exact_index: &'a BTreeMap<Vec<u8>, usize>,
    complement_index: &'a BTreeMap<Vec<u8>, usize>,
}

fn screen_coordinate(
    coordinate: usize,
    function: &BooleanFunction,
    screens: &Screens<'_>,
    counts: &mut ScreenCounts,
) -> Result<(), Box<dyn std::error::Error>> {
    if function.input_bits() > MAX_EXHAUSTIVE_PERMUTATION_BITS {
        return Err(format!(
            "input width {} exceeds exhaustive permutation bound {}",
            function.input_bits(),
            MAX_EXHAUSTIVE_PERMUTATION_BITS
        )
        .into());
    }

    if let Some(screened) = screen_full_baseline(function, screens.baseline) {
        let status = match screened.relation {
            ScreenedEquivalence::Exact => {
                counts.exact_matches += 1;
                "EXACT"
            }
            ScreenedEquivalence::OutputComplement => {
                counts.complement_matches += 1;
                "OUTPUT_COMPLEMENT"
            }
        };
        println!(
            "{coordinate}\t{status}\t{}\tidentity\tNOT_APPLICABLE\t0",
            screened.reference_index
        );
        return Ok(());
    }

    match screen_input_permutations(function, screens.exact_index, screens.complement_index) {
        Some(PermutationMatch {
            reference_index,
            permutation,
            complemented: false,
        }) => {
            counts.permutation_matches += 1;
            println!(
                "{coordinate}\tINPUT_PERMUTATION\t{reference_index}\t{}\tNOT_APPLICABLE\t0",
                format_permutation(&permutation)
            );
        }
        Some(PermutationMatch {
            reference_index,
            permutation,
            complemented: true,
        }) => {
            counts.permutation_complement_matches += 1;
            println!(
                "{coordinate}\tINPUT_PERMUTATION_OUTPUT_COMPLEMENT\t{reference_index}\t{}\tNOT_APPLICABLE\t0",
                format_permutation(&permutation)
            );
        }
        None => report_affine_screen(coordinate, function, screens.reference_functions, counts),
    }
    Ok(())
}

fn report_affine_screen(
    coordinate: usize,
    function: &BooleanFunction,
    reference_functions: &[BooleanFunction],
    counts: &mut ScreenCounts,
) {
    counts.not_found += 1;
    match screen_affine_invariants(function, reference_functions) {
        AffineInvariantScreen::Excluded => {
            counts.affine_excluded += 1;
            println!(
                "{coordinate}\tNOT_FOUND_DECLARED_EQUIVALENCE\t-\t-\tAFFINE_INVARIANTS_EXCLUDED\t0"
            );
        }
        AffineInvariantScreen::Inconclusive {
            compatible_reference_indices,
        } => {
            counts.affine_inconclusive += 1;
            counts.affine_compatible_references += compatible_reference_indices.len();
            println!(
                "{coordinate}\tNOT_FOUND_DECLARED_EQUIVALENCE\t-\t-\tAFFINE_INVARIANTS_INCONCLUSIVE\t{}",
                compatible_reference_indices.len()
            );
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct PermutationMatch {
    reference_index: usize,
    permutation: Vec<u32>,
    complemented: bool,
}

fn build_reference_indexes(
    records: &[booleanlab_discovery::baseline::BaselineRecord],
) -> (BTreeMap<Vec<u8>, usize>, BTreeMap<Vec<u8>, usize>) {
    let mut exact = BTreeMap::new();
    let mut complement = BTreeMap::new();
    for (index, record) in records.iter().enumerate() {
        exact
            .entry(record.function.truth_table().to_vec())
            .or_insert(index);
        let complemented = record
            .function
            .truth_table()
            .iter()
            .map(|value| *value ^ 1)
            .collect::<Vec<_>>();
        complement.entry(complemented).or_insert(index);
    }
    (exact, complement)
}

fn screen_input_permutations(
    candidate: &BooleanFunction,
    exact_index: &BTreeMap<Vec<u8>, usize>,
    complement_index: &BTreeMap<Vec<u8>, usize>,
) -> Option<PermutationMatch> {
    let mut permutation: Vec<u32> = (0..candidate.input_bits()).collect();
    while next_permutation(&mut permutation) {
        let table = permuted_truth_table(candidate, &permutation);
        if let Some(&reference_index) = exact_index.get(&table) {
            return Some(PermutationMatch {
                reference_index,
                permutation: permutation.clone(),
                complemented: false,
            });
        }
        if let Some(&reference_index) = complement_index.get(&table) {
            return Some(PermutationMatch {
                reference_index,
                permutation: permutation.clone(),
                complemented: true,
            });
        }
    }
    None
}

fn permuted_truth_table(candidate: &BooleanFunction, permutation: &[u32]) -> Vec<u8> {
    (0..candidate.truth_table().len())
        .map(|row| candidate.truth_table()[remap_row(row, permutation)])
        .collect()
}

fn remap_row(row: usize, permutation: &[u32]) -> usize {
    let mut remapped = 0usize;
    for (target_bit, &source_bit) in permutation.iter().enumerate() {
        if row & (1usize << target_bit) != 0 {
            remapped |= 1usize << source_bit;
        }
    }
    remapped
}

fn next_permutation(values: &mut [u32]) -> bool {
    let Some(pivot) = (0..values.len().saturating_sub(1))
        .rev()
        .find(|&index| values[index] < values[index + 1])
    else {
        return false;
    };
    let Some(successor) = (pivot + 1..values.len())
        .rev()
        .find(|&index| values[pivot] < values[index])
    else {
        return false;
    };
    values.swap(pivot, successor);
    values[pivot + 1..].reverse();
    true
}

fn format_permutation(permutation: &[u32]) -> String {
    permutation
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use booleanlab_discovery::baseline::BaselineRecord;

    fn function(bits: &[u8]) -> BooleanFunction {
        BooleanFunction::new(2, bits.to_vec()).unwrap()
    }

    fn record(function: BooleanFunction) -> BaselineRecord {
        BaselineRecord {
            metrics: function.exact_metrics(),
            function,
            gate_count: 1,
            depth: 1,
            imbalance: 0,
        }
    }

    #[test]
    fn finds_two_input_variable_swap() {
        let x0 = function(&[0, 1, 0, 1]);
        let x1 = function(&[0, 0, 1, 1]);
        let (exact, complement) = build_reference_indexes(&[record(x0)]);
        assert_eq!(
            screen_input_permutations(&x1, &exact, &complement),
            Some(PermutationMatch {
                reference_index: 0,
                permutation: vec![1, 0],
                complemented: false,
            })
        );
    }

    #[test]
    fn finds_variable_swap_with_output_complement() {
        let x0 = function(&[0, 1, 0, 1]);
        let not_x1 = function(&[1, 1, 0, 0]);
        let (exact, complement) = build_reference_indexes(&[record(x0)]);
        assert_eq!(
            screen_input_permutations(&not_x1, &exact, &complement),
            Some(PermutationMatch {
                reference_index: 0,
                permutation: vec![1, 0],
                complemented: true,
            })
        );
    }

    #[test]
    fn permutation_enumeration_excludes_identity() {
        let mut permutation = vec![0, 1, 2];
        let mut seen = Vec::new();
        while next_permutation(&mut permutation) {
            seen.push(permutation.clone());
        }
        assert_eq!(seen.len(), 5);
        assert!(!seen.contains(&vec![0, 1, 2]));
        assert!(seen.contains(&vec![2, 1, 0]));
    }
}
