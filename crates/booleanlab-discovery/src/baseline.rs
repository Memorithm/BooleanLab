use std::collections::BTreeMap;
use std::fmt;

use booleanlab_core::{BooleanCircuit, CircuitError, Node};
use scirust_modalg::boolean::walsh_hadamard;

use crate::{BooleanFunction, ExactMetrics, FunctionError};

/// Deterministic bounded Boolean-only circuit-search configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaselineConfig {
    pub input_bits: u32,
    pub candidates: u32,
    pub min_gates: usize,
    pub max_gates: usize,
    pub seed: u64,
}

/// One unique Boolean function retained from the bounded search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineRecord {
    pub function: BooleanFunction,
    pub metrics: ExactMetrics,
    pub gate_count: usize,
    pub depth: usize,
    pub imbalance: usize,
}

/// Summary of a bounded Boolean-only search campaign.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaselineSummary {
    pub config: BaselineConfig,
    pub generated: u32,
    pub unique_functions: usize,
    pub balanced_functions: usize,
    pub bent_functions: usize,
    pub best_nonlinearity: u64,
    pub pareto_front: Vec<BaselineRecord>,
}

/// Exhaustive four-input calibration statistics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FourVariableReferenceStats {
    pub total_functions: u32,
    pub balanced_functions: u32,
    pub bent_functions: u32,
    pub resilient_functions: u32,
    pub three_valued_plateaued_functions: u32,
    pub best_nonlinearity: u64,
    pub best_balanced_nonlinearity: u64,
}

/// Errors emitted by the Boolean-only baseline machinery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaselineError {
    ZeroCandidates,
    InvalidGateRange { min_gates: usize, max_gates: usize },
    InputWidth { width: u32, maximum: usize },
    Circuit(CircuitError),
    Function(FunctionError),
}

impl BaselineConfig {
    /// Frozen BL-13.1.2 reference configuration.
    #[must_use]
    pub const fn bl13_reference() -> Self {
        Self {
            input_bits: 8,
            candidates: 4_096,
            min_gates: 4,
            max_gates: 24,
            seed: 0x424c_3133_5f42_4153,
        }
    }
}

/// Exhaustively scans all 65,536 scalar Boolean functions on four inputs.
///
/// This is an oracle calibration for the screening layer, not a synthesis
/// algorithm. `three_valued_plateaued_functions` uses the explicit operational
/// definition that the Walsh spectrum contains zero and every non-zero
/// coefficient has one common absolute magnitude.
///
/// # Panics
///
/// This routine uses only fixed four-input truth tables accepted by
/// [`BooleanFunction::new`]; construction failure would indicate an internal
/// invariant violation.
#[must_use]
pub fn scan_four_variable_reference_space() -> FourVariableReferenceStats {
    let mut balanced_functions = 0_u32;
    let mut bent_functions = 0_u32;
    let mut resilient_functions = 0_u32;
    let mut plateaued_functions = 0_u32;
    let mut best_nonlinearity = 0_u64;
    let mut best_balanced_nonlinearity = 0_u64;

    for encoded in 0_u32..=u16::MAX.into() {
        let table = (0..16)
            .map(|bit| ((encoded >> bit) & 1) as u8)
            .collect::<Vec<_>>();
        let function = BooleanFunction::new(4, table).expect("four-input table is valid");
        let metrics = function.exact_metrics();

        if metrics.balanced {
            balanced_functions += 1;
            best_balanced_nonlinearity = best_balanced_nonlinearity.max(metrics.nonlinearity);
            if metrics.correlation_immunity > 0 {
                resilient_functions += 1;
            }
        }
        if metrics.bent {
            bent_functions += 1;
        }
        if is_three_valued_plateaued(function.truth_table(), 4) {
            plateaued_functions += 1;
        }
        best_nonlinearity = best_nonlinearity.max(metrics.nonlinearity);
    }

    FourVariableReferenceStats {
        total_functions: 1_u32 << 16,
        balanced_functions,
        bent_functions,
        resilient_functions,
        three_valued_plateaued_functions: plateaued_functions,
        best_nonlinearity,
        best_balanced_nonlinearity,
    }
}

/// Runs a deterministic Boolean-only circuit search and returns its exact
/// deduplicated population and Pareto frontier.
///
/// The Pareto objectives are: maximize nonlinearity, minimize imbalance,
/// maximize correlation immunity, minimize gate count and minimize circuit
/// depth. Algebraic degree is reported but intentionally not treated as a
/// monotonic objective.
///
/// # Errors
///
/// Returns [`BaselineError::ZeroCandidates`] for an empty campaign,
/// [`BaselineError::InvalidGateRange`] for an invalid gate budget,
/// [`BaselineError::InputWidth`] when exhaustive circuit evaluation would exceed
/// the core bound, or wraps circuit/function construction failures.
pub fn run_boolean_baseline(config: BaselineConfig) -> Result<BaselineSummary, BaselineError> {
    validate_config(config)?;

    let mut rng = SplitMix64::new(config.seed);
    let mut unique: Vec<BaselineRecord> = Vec::new();
    let mut buckets: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    let mut pareto_front: Vec<BaselineRecord> = Vec::new();

    for _ in 0..config.candidates {
        let gate_count = config.min_gates + rng.bounded(config.max_gates - config.min_gates + 1);
        let (circuit, depth) = random_circuit(config.input_bits, gate_count, &mut rng)?;
        let function = circuit_to_function(&circuit)?;
        let fingerprint = function.stable_fingerprint();

        let duplicate = buckets.get(&fingerprint).is_some_and(|indices| {
            indices
                .iter()
                .any(|&index| unique[index].function == function)
        });
        if duplicate {
            continue;
        }

        let metrics = function.exact_metrics();
        let ones = function
            .truth_table()
            .iter()
            .filter(|&&bit| bit == 1)
            .count();
        let imbalance = ones.abs_diff(function.truth_table().len() / 2);
        let record = BaselineRecord {
            function,
            metrics,
            gate_count,
            depth,
            imbalance,
        };

        let index = unique.len();
        buckets.entry(fingerprint).or_default().push(index);
        update_pareto_front(&mut pareto_front, &record);
        unique.push(record);
    }

    let balanced_functions = unique
        .iter()
        .filter(|record| record.metrics.balanced)
        .count();
    let bent_functions = unique.iter().filter(|record| record.metrics.bent).count();
    let best_nonlinearity = unique
        .iter()
        .map(|record| record.metrics.nonlinearity)
        .max()
        .unwrap_or(0);

    pareto_front.sort_by_key(|record| {
        (
            std::cmp::Reverse(record.metrics.nonlinearity),
            record.imbalance,
            std::cmp::Reverse(record.metrics.correlation_immunity),
            record.gate_count,
            record.depth,
            record.function.stable_fingerprint(),
        )
    });

    Ok(BaselineSummary {
        config,
        generated: config.candidates,
        unique_functions: unique.len(),
        balanced_functions,
        bent_functions,
        best_nonlinearity,
        pareto_front,
    })
}

fn validate_config(config: BaselineConfig) -> Result<(), BaselineError> {
    if config.candidates == 0 {
        return Err(BaselineError::ZeroCandidates);
    }
    if config.min_gates == 0 || config.min_gates > config.max_gates {
        return Err(BaselineError::InvalidGateRange {
            min_gates: config.min_gates,
            max_gates: config.max_gates,
        });
    }
    let maximum = BooleanCircuit::MAX_EXACT_INPUT_BITS;
    if config.input_bits == 0
        || usize::try_from(config.input_bits).map_or(true, |width| width > maximum)
    {
        return Err(BaselineError::InputWidth {
            width: config.input_bits,
            maximum,
        });
    }
    Ok(())
}

fn random_circuit(
    input_bits: u32,
    gate_count: usize,
    rng: &mut SplitMix64,
) -> Result<(BooleanCircuit, usize), BaselineError> {
    let input_width = usize::try_from(input_bits).map_err(|_| BaselineError::InputWidth {
        width: input_bits,
        maximum: BooleanCircuit::MAX_EXACT_INPUT_BITS,
    })?;
    let mut nodes = (0..input_width).map(Node::Input).collect::<Vec<_>>();
    nodes.push(Node::Const(false));
    nodes.push(Node::Const(true));
    let mut depths = vec![0_usize; nodes.len()];

    for _ in 0..gate_count {
        let available = nodes.len();
        let gate = rng.bounded(8);
        let a = rng.bounded(available);
        let b = rng.bounded(available);
        let c = rng.bounded(available);
        let (node, depth) = match gate {
            0 => (Node::Not(a), depths[a] + 1),
            1 => (Node::And(a, b), depths[a].max(depths[b]) + 1),
            2 => (Node::Or(a, b), depths[a].max(depths[b]) + 1),
            3 => (Node::Xor(a, b), depths[a].max(depths[b]) + 1),
            4 => (Node::Xnor(a, b), depths[a].max(depths[b]) + 1),
            5 => (Node::Nand(a, b), depths[a].max(depths[b]) + 1),
            6 => (Node::Nor(a, b), depths[a].max(depths[b]) + 1),
            _ => (
                Node::Mux {
                    select: a,
                    when_false: b,
                    when_true: c,
                },
                depths[a].max(depths[b]).max(depths[c]) + 1,
            ),
        };
        nodes.push(node);
        depths.push(depth);
    }

    let output = nodes.len() - 1;
    let depth = depths[output];
    let circuit =
        BooleanCircuit::new(input_width, nodes, vec![output]).map_err(BaselineError::Circuit)?;
    Ok((circuit, depth))
}

fn circuit_to_function(circuit: &BooleanCircuit) -> Result<BooleanFunction, BaselineError> {
    let rows = circuit
        .exact_truth_table()
        .map_err(BaselineError::Circuit)?;
    let table = rows
        .iter()
        .map(|row| u8::from(row.output.iter().next().unwrap_or(false)))
        .collect();
    BooleanFunction::new(circuit.input_width() as u32, table).map_err(BaselineError::Function)
}

fn update_pareto_front(front: &mut Vec<BaselineRecord>, candidate: &BaselineRecord) {
    if front.iter().any(|known| dominates(known, candidate)) {
        return;
    }
    front.retain(|known| !dominates(candidate, known));
    front.push(candidate.clone());
}

fn dominates(left: &BaselineRecord, right: &BaselineRecord) -> bool {
    let no_worse = left.metrics.nonlinearity >= right.metrics.nonlinearity
        && left.imbalance <= right.imbalance
        && left.metrics.correlation_immunity >= right.metrics.correlation_immunity
        && left.gate_count <= right.gate_count
        && left.depth <= right.depth;
    let strictly_better = left.metrics.nonlinearity > right.metrics.nonlinearity
        || left.imbalance < right.imbalance
        || left.metrics.correlation_immunity > right.metrics.correlation_immunity
        || left.gate_count < right.gate_count
        || left.depth < right.depth;
    no_worse && strictly_better
}

fn is_three_valued_plateaued(table: &[u8], input_bits: u32) -> bool {
    let spectrum = walsh_hadamard(table, input_bits);
    let mut magnitude = None;
    let mut saw_zero = false;
    for value in spectrum {
        let absolute = value.unsigned_abs();
        if absolute == 0 {
            saw_zero = true;
            continue;
        }
        match magnitude {
            Some(expected) if expected != absolute => return false,
            Some(_) => {}
            None => magnitude = Some(absolute),
        }
    }
    saw_zero && magnitude.is_some()
}

#[derive(Clone, Copy, Debug)]
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

    fn bounded(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        (self.next() % upper as u64) as usize
    }
}

impl fmt::Display for BaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCandidates => write!(formatter, "baseline campaign requires candidates"),
            Self::InvalidGateRange {
                min_gates,
                max_gates,
            } => write!(
                formatter,
                "invalid gate range: min={min_gates}, max={max_gates}"
            ),
            Self::InputWidth { width, maximum } => write!(
                formatter,
                "baseline input width {width} is outside exact circuit bound 1..={maximum}"
            ),
            Self::Circuit(error) => error.fmt(formatter),
            Self::Function(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BaselineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_variable_scan_recovers_reference_classes() {
        let stats = scan_four_variable_reference_space();
        assert_eq!(stats.total_functions, 65_536);
        assert!(stats.balanced_functions > 0);
        assert!(stats.bent_functions > 0);
        assert!(stats.resilient_functions > 0);
        assert!(stats.three_valued_plateaued_functions > 0);
        assert_eq!(stats.best_nonlinearity, 6);
    }

    #[test]
    fn bounded_search_is_deterministic() {
        let config = BaselineConfig {
            input_bits: 5,
            candidates: 64,
            min_gates: 2,
            max_gates: 8,
            seed: 7,
        };
        assert_eq!(
            run_boolean_baseline(config).unwrap(),
            run_boolean_baseline(config).unwrap()
        );
    }

    #[test]
    fn pareto_front_is_pairwise_nondominated() {
        let summary = run_boolean_baseline(BaselineConfig {
            input_bits: 5,
            candidates: 128,
            min_gates: 2,
            max_gates: 10,
            seed: 11,
        })
        .unwrap();
        for (index, left) in summary.pareto_front.iter().enumerate() {
            for (other_index, right) in summary.pareto_front.iter().enumerate() {
                if index != other_index {
                    assert!(!dominates(left, right));
                }
            }
        }
    }
}
