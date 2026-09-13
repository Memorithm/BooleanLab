use booleanlab_core::BitSignature;
use booleanlab_discovery::attention_frontier::{
    density_false_negative_frontier, sweep_hamming_thresholds,
};

fn signature(bits: &[bool]) -> Result<BitSignature, Box<dyn std::error::Error>> {
    Ok(BitSignature::from_bits(bits)?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let query = signature(&[true, false, true, false, true, false, true, false])?;
    let keys = [
        signature(&[true, false, true, false, true, false, true, false])?,
        signature(&[false, false, true, false, true, false, true, false])?,
        signature(&[false, true, true, false, true, false, true, false])?,
        signature(&[false, true, false, false, true, false, true, false])?,
        signature(&[false, true, false, true, true, false, true, false])?,
        signature(&[false, true, false, true, false, true, false, true])?,
    ];
    let reference = [true, true, true, false, false, false];
    let thresholds = [0_u64, 1, 2, 3, 4, 8];

    let points = sweep_hamming_thresholds(&query, &keys, &reference, &thresholds)?;
    let frontier = density_false_negative_frontier(&points);

    let frontier_thresholds: Vec<u64> = frontier.iter().map(|point| point.threshold()).collect();
    if frontier_thresholds != [0, 1, 2] {
        return Err(format!(
            "unexpected BL-4 calibration frontier: {frontier_thresholds:?}"
        )
        .into());
    }

    println!("BL-4.4.2 deterministic Hamming-frontier calibration");
    println!(
        "threshold\ttotal\tadmitted\treference_positive\ttrue_positive\tfalse_positive\tfalse_negative\tfrontier"
    );

    for point in points {
        let score = point.score();
        let on_frontier = frontier_thresholds.contains(&point.threshold());
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            point.threshold(),
            score.total(),
            score.admitted(),
            score.reference_positive(),
            score.true_positive(),
            score.false_positive(),
            score.false_negative(),
            on_frontier
        );
    }

    Ok(())
}
