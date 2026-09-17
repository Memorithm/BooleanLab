use booleanlab_discovery::tropical::TropicalComparison;
use booleanlab_discovery::tropical_search::bl13_4_1_bounded_search;

fn bits(values: &[u8]) -> String {
    values.iter().map(u8::to_string).collect::<String>()
}

fn integers(values: &[i64]) -> String {
    values
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn comparison_name(comparison: TropicalComparison) -> &'static str {
    match comparison {
        TropicalComparison::Greater => ">",
        TropicalComparison::GreaterOrEqual => ">=",
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let report = bl13_4_1_bounded_search()?;
    let summary = &report.summary;
    println!("experiment=BL-13.4.1");
    println!("configuration=bl13.4.1-frozen-four-input-max-plus-v1");
    println!("enumeration=seedless exhaustive enumeration");
    println!("syntactic_terms={}", summary.syntactic_terms);
    println!("polynomials={}", summary.polynomials);
    println!("predicates_evaluated={}", summary.predicates_evaluated);
    println!("unique_truth_tables={}", summary.unique_truth_tables);
    println!("duplicate_predicates={}", summary.duplicate_predicates);
    println!("constant_functions={}", summary.constant_functions);
    println!("nonconstant_functions={}", summary.nonconstant_functions);
    println!("balanced_functions={}", summary.balanced_functions);
    println!("max_nonlinearity={}", summary.max_nonlinearity);
    println!("max_algebraic_degree={}", summary.max_algebraic_degree);
    println!("h1_witness_count={}", summary.h1_witness_count);
    println!(
        "control_present={}",
        summary
            .control_present
            .iter()
            .map(|present| u8::from(*present).to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    println!(
        "decision={}",
        if summary.h1_witness_count > 0 {
            "H1_OBSERVED"
        } else {
            "H0_RETAINED"
        }
    );
    println!("reported_witnesses={}", report.witnesses.len());
    for (rank, witness) in report.witnesses.iter().enumerate() {
        println!(
            "witness\trank={}\tleft_polynomial={}\tright_polynomial={}\tcomparison={}\tfingerprint={:016x}\tdegree={}\tnonlinearity={}\tbalanced={}\tcorrelation_immunity={}\ttruth_table={}\tanf={}\twalsh={}",
            rank + 1,
            witness.source.left_polynomial,
            witness.source.right_polynomial,
            comparison_name(witness.source.comparison),
            witness.fingerprint,
            witness.metrics.algebraic_degree,
            witness.metrics.nonlinearity,
            witness.metrics.balanced,
            witness.metrics.correlation_immunity,
            bits(&witness.truth_table),
            bits(&witness.anf_coefficients),
            integers(&witness.walsh_spectrum),
        );
    }
    Ok(())
}
