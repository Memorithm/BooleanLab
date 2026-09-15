//! BL-BE2 reproducible representation benchmark harness.
//!
//! This binary measures the current Strong-Kleene reference evaluator against
//! the bounded `u64` and multiword conjunction representations. It reports raw
//! wall-clock observations only: no threshold, speedup claim, or production
//! runtime conclusion is encoded here.

use std::env;
use std::hint::black_box;
use std::io;
use std::time::{Duration, Instant};

use booleanlab_core::{
    CompiledKleeneConjunction, CompiledMultiwordKleeneConjunction, KleeneInstruction,
    KleeneLiteral, KleeneValue, evaluate_kleene_program,
};

const SCHEMA: &str = "booleanlab.bl-be2-representation-benchmark.v2";
const DEFAULT_EVALUATIONS: usize = 100_000;
const DEFAULT_SEED: u64 = 0x42b0_01ea_2026_0915;
const CORPUS_ROWS: usize = 257;
const U64_ARITY: usize = 32;
const MULTIWORD_ARITY: usize = 256;
const MAX_EVALUATIONS: usize = 10_000_000;

#[derive(Clone, Copy)]
struct Measurement {
    elapsed: Duration,
    checksum: u64,
}

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn kleene_value(&mut self) -> KleeneValue {
        match self.next() % 3 {
            0 => KleeneValue::False,
            1 => KleeneValue::Unknown,
            _ => KleeneValue::True,
        }
    }
}

fn value_code(value: KleeneValue) -> u64 {
    match value {
        KleeneValue::False => 0,
        KleeneValue::Unknown => 1,
        KleeneValue::True => 2,
    }
}

fn literals(arity: usize) -> Vec<KleeneLiteral> {
    (0..arity)
        .map(|input| {
            if input.is_multiple_of(2) {
                KleeneLiteral::positive(input)
            } else {
                KleeneLiteral::negative(input)
            }
        })
        .collect()
}

fn program(literals: &[KleeneLiteral]) -> Vec<KleeneInstruction> {
    if literals.is_empty() {
        return vec![KleeneInstruction::Constant(KleeneValue::True)];
    }

    let mut instructions = Vec::with_capacity(literals.len().saturating_mul(3));
    for (index, literal) in literals.iter().enumerate() {
        instructions.push(KleeneInstruction::Input(literal.input));
        if !literal.require_true {
            instructions.push(KleeneInstruction::Not);
        }
        if index != 0 {
            instructions.push(KleeneInstruction::And);
        }
    }
    instructions
}

const fn satisfying_value(input: usize) -> KleeneValue {
    if input.is_multiple_of(2) {
        KleeneValue::True
    } else {
        KleeneValue::False
    }
}

const fn violating_value(input: usize) -> KleeneValue {
    if input.is_multiple_of(2) {
        KleeneValue::False
    } else {
        KleeneValue::True
    }
}

fn corpus(arity: usize, seed: u64) -> Vec<Vec<KleeneValue>> {
    let arity_u64 = u64::try_from(arity).expect("bounded benchmark arity fits in u64");
    let mut rng = SplitMix64::new(seed ^ arity_u64.rotate_left(17));
    let mut rows = Vec::with_capacity(CORPUS_ROWS);

    if arity != 0 {
        let satisfied: Vec<_> = (0..arity).map(satisfying_value).collect();
        rows.push(satisfied.clone());

        let mut unknown_at_end = satisfied.clone();
        unknown_at_end[arity - 1] = KleeneValue::Unknown;
        rows.push(unknown_at_end);

        let mut failure_at_end = satisfied.clone();
        failure_at_end[arity - 1] = violating_value(arity - 1);
        rows.push(failure_at_end);

        for input in [63_usize, 64, 127, 128, 191, 192, 255] {
            if input < arity {
                let mut boundary_failure = satisfied.clone();
                boundary_failure[input] = violating_value(input);
                rows.push(boundary_failure);
            }
        }
    }

    while rows.len() < CORPUS_ROWS {
        rows.push((0..arity).map(|_| rng.kleene_value()).collect());
    }
    rows
}

fn outcome_histogram(program: &[KleeneInstruction], inputs: &[Vec<KleeneValue>]) -> [usize; 3] {
    let mut counts = [0_usize; 3];
    for assignment in inputs {
        let value = evaluate_kleene_program(program, assignment)
            .expect("generated generic conjunction must be valid");
        match value {
            KleeneValue::False => counts[0] += 1,
            KleeneValue::Unknown => counts[1] += 1,
            KleeneValue::True => counts[2] += 1,
        }
    }
    counts
}

fn verify_u64_panel(
    generic: &[KleeneInstruction],
    compiled: CompiledKleeneConjunction,
    multiword: &CompiledMultiwordKleeneConjunction,
    inputs: &[Vec<KleeneValue>],
) {
    for assignment in inputs {
        let expected = evaluate_kleene_program(generic, assignment)
            .expect("generated generic conjunction must be valid");
        let u64_value = compiled
            .evaluate(assignment)
            .expect("generated u64 conjunction must accept the corpus arity");
        let multiword_value = multiword
            .evaluate(assignment)
            .expect("generated multiword conjunction must accept the corpus arity");
        assert_eq!(u64_value, expected, "u64 backend diverged before timing");
        assert_eq!(
            multiword_value, expected,
            "multiword backend diverged before timing"
        );
    }
}

fn verify_multiword_panel(
    generic: &[KleeneInstruction],
    compiled: &CompiledMultiwordKleeneConjunction,
    inputs: &[Vec<KleeneValue>],
) {
    for assignment in inputs {
        let expected = evaluate_kleene_program(generic, assignment)
            .expect("generated generic conjunction must be valid");
        let actual = compiled
            .evaluate(assignment)
            .expect("generated multiword conjunction must accept the corpus arity");
        assert_eq!(actual, expected, "multiword backend diverged before timing");
    }
}

fn measure_generic(
    program: &[KleeneInstruction],
    inputs: &[Vec<KleeneValue>],
    evaluations: usize,
) -> Measurement {
    let mut checksum = 0_u64;
    let started = Instant::now();
    for index in 0..evaluations {
        let assignment = black_box(&inputs[index % inputs.len()]);
        let value = evaluate_kleene_program(black_box(program), assignment)
            .expect("preflight established a valid generated program");
        checksum = checksum.wrapping_add(value_code(black_box(value)));
    }
    Measurement {
        elapsed: started.elapsed(),
        checksum,
    }
}

fn measure_u64(
    compiled: CompiledKleeneConjunction,
    inputs: &[Vec<KleeneValue>],
    evaluations: usize,
) -> Measurement {
    let mut checksum = 0_u64;
    let started = Instant::now();
    for index in 0..evaluations {
        let assignment = black_box(&inputs[index % inputs.len()]);
        let value = black_box(compiled)
            .evaluate(assignment)
            .expect("preflight established a compatible u64 corpus");
        checksum = checksum.wrapping_add(value_code(black_box(value)));
    }
    Measurement {
        elapsed: started.elapsed(),
        checksum,
    }
}

fn measure_multiword(
    compiled: &CompiledMultiwordKleeneConjunction,
    inputs: &[Vec<KleeneValue>],
    evaluations: usize,
) -> Measurement {
    let mut checksum = 0_u64;
    let started = Instant::now();
    for index in 0..evaluations {
        let assignment = black_box(&inputs[index % inputs.len()]);
        let value = black_box(compiled)
            .evaluate(assignment)
            .expect("preflight established a compatible multiword corpus");
        checksum = checksum.wrapping_add(value_code(black_box(value)));
    }
    Measurement {
        elapsed: started.elapsed(),
        checksum,
    }
}

fn warm_up(
    program: &[KleeneInstruction],
    compiled: &CompiledMultiwordKleeneConjunction,
    inputs: &[Vec<KleeneValue>],
) {
    for assignment in inputs {
        black_box(
            evaluate_kleene_program(black_box(program), black_box(assignment))
                .expect("generated generic conjunction must remain valid"),
        );
        black_box(
            compiled
                .evaluate(black_box(assignment))
                .expect("generated multiword conjunction must remain valid"),
        );
    }
}

fn print_measurement(
    panel: &str,
    backend: &str,
    arity: usize,
    instruction_count: usize,
    mask_words: usize,
    evaluations: usize,
    measurement: Measurement,
) {
    let elapsed_ns = measurement.elapsed.as_nanos();
    let evaluations_u128 =
        u128::try_from(evaluations).expect("bounded evaluation count fits in u128");
    let ns_per_eval_x1000 = elapsed_ns
        .saturating_mul(1_000)
        .checked_div(evaluations_u128)
        .unwrap_or(0);
    println!(
        "measurement\t{panel}\t{backend}\t{arity}\t{instruction_count}\t{mask_words}\t{evaluations}\t{elapsed_ns}\t{ns_per_eval_x1000}\t{}",
        measurement.checksum
    );
}

fn parse_args() -> Result<(usize, u64), io::Error> {
    let mut args = env::args().skip(1);
    let evaluations = match args.next() {
        Some(raw) => raw.parse::<usize>().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid evaluation count {raw:?}: {error}"),
            )
        })?,
        None => DEFAULT_EVALUATIONS,
    };
    let seed = match args.next() {
        Some(raw) => raw.parse::<u64>().map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid seed {raw:?}: {error}"),
            )
        })?,
        None => DEFAULT_SEED,
    };
    if args.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: bl_be2_repr_bench [evaluations] [seed]",
        ));
    }
    if evaluations == 0 || evaluations > MAX_EVALUATIONS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("evaluations must be in 1..={MAX_EVALUATIONS}"),
        ));
    }
    Ok((evaluations, seed))
}

fn main() -> Result<(), io::Error> {
    let (evaluations, seed) = parse_args()?;

    let u64_literals = literals(U64_ARITY);
    let u64_program = program(&u64_literals);
    let u64_compiled = CompiledKleeneConjunction::compile(U64_ARITY, &u64_literals)
        .expect("fixed u64 benchmark panel is within the compiled bound");
    let u64_multiword = CompiledMultiwordKleeneConjunction::compile(U64_ARITY, &u64_literals)
        .expect("fixed u64 benchmark panel is within the multiword bound");
    let u64_inputs = corpus(U64_ARITY, seed);
    verify_u64_panel(&u64_program, u64_compiled, &u64_multiword, &u64_inputs);
    let u64_outcomes = outcome_histogram(&u64_program, &u64_inputs);
    assert!(u64_outcomes.iter().all(|count| *count != 0));

    let multiword_literals = literals(MULTIWORD_ARITY);
    let multiword_program = program(&multiword_literals);
    let multiword_compiled =
        CompiledMultiwordKleeneConjunction::compile(MULTIWORD_ARITY, &multiword_literals)
            .expect("fixed multiword benchmark panel is within the multiword bound");
    let multiword_inputs = corpus(MULTIWORD_ARITY, seed.rotate_left(23));
    verify_multiword_panel(&multiword_program, &multiword_compiled, &multiword_inputs);
    let multiword_outcomes = outcome_histogram(&multiword_program, &multiword_inputs);
    assert!(multiword_outcomes.iter().all(|count| *count != 0));

    warm_up(&u64_program, &u64_multiword, &u64_inputs);
    warm_up(&multiword_program, &multiword_compiled, &multiword_inputs);

    println!("schema\t{SCHEMA}");
    println!("seed\t{seed}");
    println!("corpus_rows\t{CORPUS_ROWS}");
    println!("os\t{}", env::consts::OS);
    println!("arch\t{}", env::consts::ARCH);
    println!(
        "source_head_sha\t{}",
        env::var("BOOLEANLAB_SOURCE_SHA").unwrap_or_else(|_| "unavailable".to_owned())
    );
    println!("claim_boundary\traw_wall_clock_observation_only");
    println!(
        "corpus_outcomes\tu64-comparable\tfalse={}\tunknown={}\ttrue={}",
        u64_outcomes[0], u64_outcomes[1], u64_outcomes[2]
    );
    println!(
        "corpus_outcomes\tmultiword\tfalse={}\tunknown={}\ttrue={}",
        multiword_outcomes[0], multiword_outcomes[1], multiword_outcomes[2]
    );
    println!(
        "columns\tpanel\tbackend\tinput_arity\tgeneric_instruction_count\tmask_words_per_polarity\tevaluations\telapsed_ns\tns_per_eval_x1000\tchecksum"
    );

    let u64_generic = measure_generic(&u64_program, &u64_inputs, evaluations);
    let u64_mask = measure_u64(u64_compiled, &u64_inputs, evaluations);
    let u64_multiword_measure = measure_multiword(&u64_multiword, &u64_inputs, evaluations);
    assert_eq!(u64_generic.checksum, u64_mask.checksum);
    assert_eq!(u64_generic.checksum, u64_multiword_measure.checksum);

    print_measurement(
        "u64-comparable",
        "generic-postfix",
        U64_ARITY,
        u64_program.len(),
        0,
        evaluations,
        u64_generic,
    );
    print_measurement(
        "u64-comparable",
        "u64-conjunction",
        U64_ARITY,
        u64_program.len(),
        1,
        evaluations,
        u64_mask,
    );
    print_measurement(
        "u64-comparable",
        "multiword-conjunction",
        U64_ARITY,
        u64_program.len(),
        u64_multiword.word_count(),
        evaluations,
        u64_multiword_measure,
    );

    let multiword_generic = measure_generic(&multiword_program, &multiword_inputs, evaluations);
    let multiword_mask = measure_multiword(&multiword_compiled, &multiword_inputs, evaluations);
    assert_eq!(multiword_generic.checksum, multiword_mask.checksum);

    print_measurement(
        "multiword",
        "generic-postfix",
        MULTIWORD_ARITY,
        multiword_program.len(),
        0,
        evaluations,
        multiword_generic,
    );
    print_measurement(
        "multiword",
        "multiword-conjunction",
        MULTIWORD_ARITY,
        multiword_program.len(),
        multiword_compiled.word_count(),
        evaluations,
        multiword_mask,
    );

    Ok(())
}
