use std::hint::black_box;
use std::time::{Duration, Instant};

use booleanlab_core::{
    CompiledMultiwordKleeneConjunction, KleeneInstruction, KleeneLiteral, KleeneValue,
    evaluate_kleene_program,
};

const PROGRAM: [KleeneInstruction; 4] = [
    KleeneInstruction::Input(0),
    KleeneInstruction::Input(64),
    KleeneInstruction::Not,
    KleeneInstruction::And,
];

fn assignment(iteration: usize) -> Vec<KleeneValue> {
    let values = [KleeneValue::False, KleeneValue::Unknown, KleeneValue::True];
    let mut inputs = vec![KleeneValue::True; 65];
    inputs[0] = values[iteration % values.len()];
    inputs[64] = values[(iteration / values.len()) % values.len()];
    inputs
}

fn checksum_value(value: KleeneValue) -> u64 {
    match value {
        KleeneValue::False => 1,
        KleeneValue::Unknown => 3,
        KleeneValue::True => 7,
    }
}

fn run_generic(iterations: usize) -> Result<(Duration, u64), String> {
    let started = Instant::now();
    let mut checksum = 0_u64;
    for iteration in 0..iterations {
        let inputs = black_box(assignment(iteration));
        let value = evaluate_kleene_program(&PROGRAM, &inputs)
            .map_err(|error| format!("generic evaluator failed: {error:?}"))?;
        checksum = checksum.wrapping_add(checksum_value(black_box(value)));
    }
    Ok((started.elapsed(), checksum))
}

fn run_multiword(
    compiled: &CompiledMultiwordKleeneConjunction,
    iterations: usize,
) -> Result<(Duration, u64), String> {
    let started = Instant::now();
    let mut checksum = 0_u64;
    for iteration in 0..iterations {
        let inputs = black_box(assignment(iteration));
        let value = compiled
            .evaluate(&inputs)
            .map_err(|error| format!("multiword evaluator failed: {error:?}"))?;
        checksum = checksum.wrapping_add(checksum_value(black_box(value)));
    }
    Ok((started.elapsed(), checksum))
}

fn parse_positive_usize(raw: Option<String>, default: usize, name: &str) -> Result<usize, String> {
    let Some(raw) = raw else {
        return Ok(default);
    };
    let value = raw
        .parse::<usize>()
        .map_err(|error| format!("invalid {name} value {raw:?}: {error}"))?;
    if value == 0 {
        return Err(format!("{name} must be greater than zero"));
    }
    Ok(value)
}

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let warmup_iterations = parse_positive_usize(args.next(), 20_000, "warmup_iterations")?;
    let measured_iterations = parse_positive_usize(args.next(), 200_000, "measured_iterations")?;
    let repetitions = parse_positive_usize(args.next(), 10, "repetitions")?;
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra:?}"));
    }

    let compiled = CompiledMultiwordKleeneConjunction::compile(
        65,
        &[KleeneLiteral::positive(0), KleeneLiteral::negative(64)],
    )
    .map_err(|error| format!("failed to compile multiword conjunction: {error:?}"))?;

    let (_, generic_warmup_checksum) = run_generic(warmup_iterations)?;
    let (_, multiword_warmup_checksum) = run_multiword(&compiled, warmup_iterations)?;
    if generic_warmup_checksum != multiword_warmup_checksum {
        return Err(format!(
            "warmup differential mismatch: generic={generic_warmup_checksum} multiword={multiword_warmup_checksum}"
        ));
    }

    println!("schema=booleanlab.bl-be2.multiword-benchmark.v1");
    println!("semantic_scope=strong-kleene-65-input-two-literal-conjunction");
    println!("word_count={}", compiled.word_count());
    println!("warmup_iterations={warmup_iterations}");
    println!("measured_iterations={measured_iterations}");
    println!("repetitions={repetitions}");
    println!("columns=repetition,backend,elapsed_ns,checksum");

    for repetition in 0..repetitions {
        let (generic_elapsed, generic_checksum) = run_generic(measured_iterations)?;
        let (multiword_elapsed, multiword_checksum) =
            run_multiword(&compiled, measured_iterations)?;
        if generic_checksum != multiword_checksum {
            return Err(format!(
                "differential mismatch at repetition {repetition}: generic={generic_checksum} multiword={multiword_checksum}"
            ));
        }
        println!(
            "{repetition},generic_postfix,{},{}",
            generic_elapsed.as_nanos(),
            generic_checksum
        );
        println!(
            "{repetition},compiled_multiword_u64,{},{}",
            multiword_elapsed.as_nanos(),
            multiword_checksum
        );
    }

    Ok(())
}
