use booleanlab_discovery::tropical::bl13_4_control_predicates;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let predicates = bl13_4_control_predicates()?;
    println!(
        "candidate,input_bits,ones,degree,nonlinearity,balanced,correlation_immunity,fingerprint"
    );
    for (index, predicate) in predicates.iter().enumerate() {
        let function = predicate.boolean_function()?;
        let metrics = function.exact_metrics();
        let ones = function
            .truth_table()
            .iter()
            .map(|&value| usize::from(value))
            .sum::<usize>();
        println!(
            "tropical-{index},{},{ones},{},{},{},{},{:016x}",
            function.input_bits(),
            metrics.algebraic_degree,
            metrics.nonlinearity,
            metrics.balanced,
            metrics.correlation_immunity,
            function.stable_fingerprint(),
        );
    }
    Ok(())
}
