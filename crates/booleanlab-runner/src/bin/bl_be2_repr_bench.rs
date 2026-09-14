use std::hint::black_box;
use std::time::{Duration, Instant};

use booleanlab_core::{
    CompiledKleeneConjunction, KleeneInstruction, KleeneLiteral, KleeneValue,
    evaluate_kleene_program,
};

const ASSIGNMENTS: [[KleeneValue; 3]; 9] = [
    [KleeneValue::False, KleeneValue::False, KleeneValue::False],
    [KleeneValue::False, KleeneValue::Unknown, KleeneValue::True],
    [KleeneValue::False, KleeneValue::True, KleeneValue::Unknown],
    [KleeneValue::Unknown, KleeneValue::False, KleeneValue::True],
    [
        KleeneValue::Unknown,
        KleeneValue::Unknown,
        KleeneValue::Unknown,
    ],
    [KleeneValue::Unknown, KleeneValue::True, KleeneValue::False],
    [KleeneValue::True, KleeneValue::False, KleeneValue::Unknown],
    [KleeneValue::True, KleeneValue::Unknown, KleeneValue::False],
    [KleeneValue::True, KleeneValue::True, KleeneValue::True],
];

const PROGRAM: [KleeneInstruction; 6] = [
    KleeneInstruction::Input(0),
    KleeneInstruction::Input(1),
    KleeneInstruction::Not,
    KleeneInstruction::And,
    KleeneInstruction::Input(2),
    KleeneInstruction::And,
];

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
        let inputs = black_box(&ASSIGNMENTS[iteration % ASSIGNMENTS.len()]);
        let value = evaluate_kleene_program(&PROGRAM, inputs)
            .map_err(|error| format!("generic evaluator failed: {error:?}"))?;
        checksum = checksum.wrapping_add(checksum_value(black_box(value)));
    }
    Ok((started.elapsed(), checksum))
}

fn run_compiled(
    compiled: CompiledKleeneConjunction,
    iterations: usize,
) -> Result<(Duration, u64), String> {
    let started = Instant::now();
    let mut checksum = 0_u64;
    for iteration in 0..iterations {
        let inputs = black_box(&ASSIGNMENTS[iteration % ASSIGNMENTS.len()]);
        let value = compiled
            .evaluate(inputs)
            .map_err(|error| format!("compiled evaluator failed: {error:?}"))?;
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
    let warmup_iterations = parse_positive_usize(args.next(), 50_000, "warmup_iterations")?;
    let measured_iterations = parse_positive_usize(args.next(), 500_000, "measured_iterations")?;
    let repetitions = parse_positive_usize(args.next(), 10, "repetitions")?;
    if let Some(extra) = args.next() {
        return Err(format!("unexpected extra argument: {extra:?}"));
    }

    let compiled = CompiledKleeneConjunction::compile(
        3,
        &[
            KleeneLiteral::positive(0),
            KleeneLiteral::negative(1),
            KleeneLiteral::positive(2),
        ],
    )
    .map_err(|error| format!("failed to compile benchmark conjunction: {error:?}"))?;

    let (_, generic_warmup_checksum) = run_generic(warmup_iterations)?;
    let (_, compiled_warmup_checksum) = run_compiled(compiled, warmup_iterations)?;
    if generic_warmup_checksum != compiled_warmup_checksum {
        return Err(format!(
            "warmup differential mismatch: generic={generic_warmup_checksum} compiled={compiled_warmup_checksum}"
        ));
    }

    println!("schema=booleanlab.bl-be2.representation-benchmark.v1");
    println!("semantic_scope=strong-kleene-three-input-conjunction");
    println!("warmup_iterations={warmup_iterations}");
    println!("measured_iterations={measured_iterations}");
    println!("repetitions={repetitions}");
    println!("columns=repetition,backend,elapsed_ns,checksum");

    for repetition in 0..repetitions {
        let (generic_elapsed, generic_checksum) = run_generic(measured_iterations)?;
        let (compiled_elapsed, compiled_checksum) = run_compiled(compiled, measured_iterations)?;
        if generic_checksum != compiled_checksum {
            return Err(format!(
                "differential mismatch at repetition {repetition}: generic={generic_checksum} compiled={compiled_checksum}"
            ));
        }
        println!(
            "{repetition},generic_postfix,{},{}",
            generic_elapsed.as_nanos(),
            generic_checksum
        );
        println!(
            "{repetition},compiled_u64,{},{}",
            compiled_elapsed.as_nanos(),
            compiled_checksum
        );
    }

    Ok(())
}
