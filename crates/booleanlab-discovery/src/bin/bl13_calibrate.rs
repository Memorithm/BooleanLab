use booleanlab_discovery::BooleanFunction;

fn report(name: &str, function: &BooleanFunction) {
    let metrics = function.exact_metrics();
    println!(
        "{name}\t{}\t{}\t{}\t{}\t{}\t{}\t{:016x}",
        function.input_bits(),
        metrics.algebraic_degree,
        metrics.nonlinearity,
        metrics.balanced,
        metrics.bent,
        metrics.correlation_immunity,
        function.stable_fingerprint()
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let and_2 = BooleanFunction::new(2, vec![0, 0, 0, 1])?;
    let parity_3 = BooleanFunction::from_fn(3, |x| x.count_ones() % 2 == 1)?;
    let majority_3 = BooleanFunction::from_fn(3, |x| x.count_ones() >= 2)?;

    println!("name\tinputs\tdegree\tnonlinearity\tbalanced\tbent\tcorrelation_immunity\tfingerprint");
    report("and_2", &and_2);
    report("parity_3", &parity_3);
    report("majority_3", &majority_3);

    Ok(())
}
