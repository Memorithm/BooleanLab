use booleanlab_discovery::baseline::{
    BaselineConfig, run_boolean_baseline, scan_four_variable_reference_space,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reference = scan_four_variable_reference_space();
    println!(
        "bl13.1.1\ttotal={}\tbalanced={}\tbent={}\tresilient={}\tthree_valued_plateaued={}\tbest_nonlinearity={}\tbest_balanced_nonlinearity={}",
        reference.total_functions,
        reference.balanced_functions,
        reference.bent_functions,
        reference.resilient_functions,
        reference.three_valued_plateaued_functions,
        reference.best_nonlinearity,
        reference.best_balanced_nonlinearity
    );

    let summary = run_boolean_baseline(BaselineConfig::bl13_reference())?;
    println!(
        "bl13.1.2\tgenerated={}\tunique={}\tbalanced={}\tbent={}\tbest_nonlinearity={}\tpareto={}",
        summary.generated,
        summary.unique_functions,
        summary.balanced_functions,
        summary.bent_functions,
        summary.best_nonlinearity,
        summary.pareto_front.len()
    );
    println!(
        "pareto_rank\tnonlinearity\timbalance\tcorrelation_immunity\tdegree\tgates\tdepth\tfingerprint"
    );
    for (rank, record) in summary.pareto_front.iter().take(32).enumerate() {
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            rank + 1,
            record.metrics.nonlinearity,
            record.imbalance,
            record.metrics.correlation_immunity,
            record.metrics.algebraic_degree,
            record.gate_count,
            record.depth,
            record.function.stable_fingerprint()
        );
    }
    Ok(())
}
