use booleanlab_discovery::baseline::{BaselineConfig, run_boolean_baseline};
use booleanlab_discovery::equivalence_screen::{ScreenedEquivalence, screen_full_baseline};
use booleanlab_discovery::sedenion::control_component_functions;

fn relation_name(relation: ScreenedEquivalence) -> &'static str {
    match relation {
        ScreenedEquivalence::Exact => "EXACT",
        ScreenedEquivalence::OutputComplement => "OUTPUT_COMPLEMENT",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let baseline = run_boolean_baseline(BaselineConfig::bl13_reference())?;
    let functions = control_component_functions()?;

    let mut exact_matches = 0_usize;
    let mut complement_matches = 0_usize;
    let mut not_found = 0_usize;

    println!(
        "coordinate\tstatus\treference_index\tgate_count\tdepth\timbalance\tnonlinearity\tdegree"
    );

    for (coordinate, function) in functions.iter().enumerate() {
        match screen_full_baseline(function, &baseline) {
            Some(screened) => {
                let reference = &baseline.population[screened.reference_index];
                match screened.relation {
                    ScreenedEquivalence::Exact => exact_matches += 1,
                    ScreenedEquivalence::OutputComplement => complement_matches += 1,
                }
                println!(
                    "{coordinate}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    relation_name(screened.relation),
                    screened.reference_index,
                    reference.gate_count,
                    reference.depth,
                    reference.imbalance,
                    reference.metrics.nonlinearity,
                    reference.metrics.algebraic_degree,
                );
            }
            None => {
                not_found += 1;
                println!("{coordinate}\tNOT_FOUND_DECLARED_EQUIVALENCE\t-\t-\t-\t-\t-\t-");
            }
        }
    }

    println!(
        "summary\tbaseline_unique={}\texact_matches={}\tcomplement_matches={}\tnot_found={}\tnovelty_claim=false",
        baseline.unique_functions, exact_matches, complement_matches, not_found
    );
    Ok(())
}
