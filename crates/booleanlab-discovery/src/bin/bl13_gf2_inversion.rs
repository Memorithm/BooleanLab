use booleanlab_discovery::DedupIndex;
use booleanlab_discovery::gf2::inversion_component_functions;
use scirust_modalg::gf2::Gf2Field;

fn run_family(label: &str, field: Gf2Field) -> Result<(), Box<dyn std::error::Error>> {
    let functions = inversion_component_functions(field)?;
    let mut exact_index = DedupIndex::new();
    let mut complement_index = DedupIndex::new();

    for (bit, function) in functions.into_iter().enumerate() {
        let metrics = function.exact_metrics();
        let fingerprint = function.stable_fingerprint();
        let canonical = function.canonical_under_complement();
        let _ = exact_index.insert(function);
        let _ = complement_index.insert(canonical);
        println!(
            "{label}\t{bit}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
            metrics.algebraic_degree,
            metrics.nonlinearity,
            metrics.balanced,
            metrics.bent,
            metrics.correlation_immunity,
            fingerprint
        );
    }

    println!(
        "summary\t{label}\texact_unique={}\tcomplement_unique={}",
        exact_index.unique_functions(),
        complement_index.unique_functions()
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("field\tbit\tdegree\tnonlinearity\tbalanced\tbent\tcorrelation_immunity\tfingerprint");
    run_family("primitive8", Gf2Field::primitive8())?;
    run_family("rijndael8", Gf2Field::rijndael8())?;
    Ok(())
}
