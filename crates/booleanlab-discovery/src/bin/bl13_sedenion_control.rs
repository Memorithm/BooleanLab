use booleanlab_discovery::DedupIndex;
use booleanlab_discovery::sedenion::control_component_functions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let functions = control_component_functions()?;
    let mut exact_index = DedupIndex::new();
    let mut complement_index = DedupIndex::new();

    println!("coordinate\tdegree\tnonlinearity\tbalanced\tbent\tcorrelation_immunity\tfingerprint");

    for (coordinate, function) in functions.into_iter().enumerate() {
        let metrics = function.exact_metrics();
        let fingerprint = function.stable_fingerprint();
        let canonical = function.canonical_under_complement();
        let _ = exact_index.insert(function);
        let _ = complement_index.insert(canonical);
        println!(
            "{coordinate}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            metrics.algebraic_degree,
            metrics.nonlinearity,
            metrics.balanced,
            metrics.bent,
            metrics.correlation_immunity,
            fingerprint
        );
    }

    println!(
        "summary\texact_unique={}\tcomplement_unique={}",
        exact_index.unique_functions(),
        complement_index.unique_functions()
    );
    Ok(())
}
