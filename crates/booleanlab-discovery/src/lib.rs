#![forbid(unsafe_code)]
//! Exact Boolean-function candidate representation and screening primitives for
//! the BL-13 discovery programme.
//!
//! `BooleanLab` owns candidate provenance, canonicalisation and deduplication.
//! Exact Boolean metrics are delegated to `SciRust` so there is one mathematical
//! implementation of ANF and Walsh analysis across the `Memorithm` ecosystem.

pub mod attention_frontier;
pub mod baseline;
pub mod equivalence_screen;
pub mod gf2;
#[cfg(feature = "sedenion-experiments")]
pub mod sedenion;
pub mod sparsity_freeze;
pub mod sparsity_rule_search;

use std::collections::BTreeMap;
use std::fmt;

use scirust_modalg::boolean::{
    MAX_EXACT_BITS, anf_degree_of_table, correlation_immunity, is_balanced, is_bent, nonlinearity,
};

const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Exact scalar Boolean function represented by its complete truth table.
///
/// Row `x` stores `f(x)` at `truth_table[x]`. Inputs are therefore ordered by
/// their packed integer value from zero to `2^n - 1`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BooleanFunction {
    input_bits: u32,
    truth_table: Vec<u8>,
}

/// Construction errors for exact Boolean functions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FunctionError {
    ZeroInputs,
    InputWidthTooLarge { width: u32, maximum: u32 },
    TruthTableLength { expected: usize, actual: usize },
    NonBooleanValue { index: usize, value: u8 },
}

/// Exact algebraic and spectral metrics supplied by `SciRust`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactMetrics {
    pub algebraic_degree: u32,
    pub nonlinearity: u64,
    pub balanced: bool,
    pub bent: bool,
    pub correlation_immunity: u32,
}

/// Provenance required for a generated BL-13 candidate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateProvenance {
    pub candidate_id: String,
    pub generator_family: String,
    pub domain_x: String,
    pub projection: String,
    pub generator_version: String,
    pub configuration_id: String,
}

/// Result of inserting a function into an exact deduplication index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DedupOutcome {
    New,
    Existing,
}

/// Collision-safe exact deduplication index.
///
/// The stable 64-bit fingerprint is only a bucket key. Equality is always
/// confirmed by comparing the exact input width and truth-table bytes.
#[derive(Clone, Debug, Default)]
pub struct DedupIndex {
    buckets: BTreeMap<u64, Vec<BooleanFunction>>,
    unique_functions: usize,
}

impl BooleanFunction {
    /// Builds a scalar Boolean function from an exact truth table.
    ///
    /// # Errors
    ///
    /// Returns [`FunctionError::ZeroInputs`] for zero input bits,
    /// [`FunctionError::InputWidthTooLarge`] above `SciRust`'s exact-analysis
    /// bound, [`FunctionError::TruthTableLength`] when the table does not have
    /// exactly `2^n` rows, and [`FunctionError::NonBooleanValue`] for entries
    /// other than zero or one.
    pub fn new(input_bits: u32, truth_table: Vec<u8>) -> Result<Self, FunctionError> {
        if input_bits == 0 {
            return Err(FunctionError::ZeroInputs);
        }
        if input_bits > MAX_EXACT_BITS {
            return Err(FunctionError::InputWidthTooLarge {
                width: input_bits,
                maximum: MAX_EXACT_BITS,
            });
        }

        let expected = 1usize << input_bits;
        if truth_table.len() != expected {
            return Err(FunctionError::TruthTableLength {
                expected,
                actual: truth_table.len(),
            });
        }
        if let Some((index, &value)) = truth_table
            .iter()
            .enumerate()
            .find(|(_, value)| **value > 1)
        {
            return Err(FunctionError::NonBooleanValue { index, value });
        }

        Ok(Self {
            input_bits,
            truth_table,
        })
    }

    /// Evaluates a closure exhaustively to construct an exact Boolean function.
    ///
    /// # Errors
    ///
    /// Returns the same width errors as [`Self::new`].
    pub fn from_fn(input_bits: u32, function: impl Fn(u64) -> bool) -> Result<Self, FunctionError> {
        if input_bits == 0 {
            return Err(FunctionError::ZeroInputs);
        }
        if input_bits > MAX_EXACT_BITS {
            return Err(FunctionError::InputWidthTooLarge {
                width: input_bits,
                maximum: MAX_EXACT_BITS,
            });
        }
        let rows = 1_u64 << input_bits;
        let truth_table = (0..rows).map(|x| u8::from(function(x))).collect();
        Self::new(input_bits, truth_table)
    }

    #[must_use]
    pub const fn input_bits(&self) -> u32 {
        self.input_bits
    }

    #[must_use]
    pub fn truth_table(&self) -> &[u8] {
        &self.truth_table
    }

    /// Computes exact ANF/Walsh-derived metrics using `SciRust`.
    #[must_use]
    pub fn exact_metrics(&self) -> ExactMetrics {
        let mut anf_table = self.truth_table.clone();
        ExactMetrics {
            algebraic_degree: anf_degree_of_table(&mut anf_table, self.input_bits),
            nonlinearity: nonlinearity(&self.truth_table, self.input_bits),
            balanced: is_balanced(&self.truth_table, self.input_bits),
            bent: is_bent(&self.truth_table, self.input_bits),
            correlation_immunity: correlation_immunity(&self.truth_table, self.input_bits),
        }
    }

    /// Returns a deterministic 64-bit content fingerprint.
    ///
    /// This fingerprint is not cryptographic and must never be used as proof of
    /// equality. [`DedupIndex`] always confirms exact truth-table equality.
    #[must_use]
    pub fn stable_fingerprint(&self) -> u64 {
        let mut hash = FNV_OFFSET_BASIS;
        for byte in self.input_bits.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        for &byte in &self.truth_table {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    /// Canonicalises only under output complementation: `f ~ f XOR 1`.
    ///
    /// This intentionally does not claim affine, extended-affine, CCZ or any
    /// other stronger equivalence. The lexicographically smaller truth table of
    /// `f` and its complement is returned.
    #[must_use]
    pub fn canonical_under_complement(&self) -> Self {
        let complement: Vec<u8> = self.truth_table.iter().map(|value| *value ^ 1).collect();
        if complement < self.truth_table {
            Self {
                input_bits: self.input_bits,
                truth_table: complement,
            }
        } else {
            self.clone()
        }
    }
}

impl DedupIndex {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            buckets: BTreeMap::new(),
            unique_functions: 0,
        }
    }

    /// Inserts a candidate while remaining collision-safe.
    pub fn insert(&mut self, function: BooleanFunction) -> DedupOutcome {
        let fingerprint = function.stable_fingerprint();
        let bucket = self.buckets.entry(fingerprint).or_default();
        if bucket.iter().any(|known| known == &function) {
            DedupOutcome::Existing
        } else {
            bucket.push(function);
            self.unique_functions += 1;
            DedupOutcome::New
        }
    }

    #[must_use]
    pub const fn unique_functions(&self) -> usize {
        self.unique_functions
    }
}

impl fmt::Display for FunctionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroInputs => write!(
                formatter,
                "Boolean functions require at least one input bit"
            ),
            Self::InputWidthTooLarge { width, maximum } => write!(
                formatter,
                "exact Boolean analysis supports at most {maximum} inputs, got {width}"
            ),
            Self::TruthTableLength { expected, actual } => write!(
                formatter,
                "truth table length mismatch: expected {expected}, got {actual}"
            ),
            Self::NonBooleanValue { index, value } => write!(
                formatter,
                "truth table entry {index} is {value}, expected zero or one"
            ),
        }
    }
}

impl std::error::Error for FunctionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_malformed_truth_tables() {
        assert_eq!(
            BooleanFunction::new(0, vec![0]),
            Err(FunctionError::ZeroInputs)
        );
        assert!(matches!(
            BooleanFunction::new(MAX_EXACT_BITS + 1, vec![]),
            Err(FunctionError::InputWidthTooLarge { .. })
        ));
        assert!(matches!(
            BooleanFunction::new(2, vec![0, 1]),
            Err(FunctionError::TruthTableLength { .. })
        ));
        assert_eq!(
            BooleanFunction::new(1, vec![0, 2]),
            Err(FunctionError::NonBooleanValue { index: 1, value: 2 })
        );
    }

    #[test]
    fn measures_two_bit_and_exactly() {
        let function = BooleanFunction::new(2, vec![0, 0, 0, 1]).unwrap();
        let metrics = function.exact_metrics();
        assert_eq!(metrics.algebraic_degree, 2);
        assert_eq!(metrics.nonlinearity, 1);
        assert!(!metrics.balanced);
        assert!(metrics.bent);
        assert_eq!(metrics.correlation_immunity, 0);
    }

    #[test]
    fn measures_three_bit_parity_exactly() {
        let parity = BooleanFunction::from_fn(3, |x| x.count_ones() % 2 == 1).unwrap();
        let metrics = parity.exact_metrics();
        assert_eq!(metrics.algebraic_degree, 1);
        assert_eq!(metrics.nonlinearity, 0);
        assert!(metrics.balanced);
        assert!(!metrics.bent);
        assert_eq!(metrics.correlation_immunity, 2);
    }

    #[test]
    fn complement_canonicalisation_is_exact() {
        let function = BooleanFunction::new(2, vec![0, 0, 0, 1]).unwrap();
        let complement = BooleanFunction::new(2, vec![1, 1, 1, 0]).unwrap();
        assert_eq!(
            function.canonical_under_complement(),
            complement.canonical_under_complement()
        );
    }

    #[test]
    fn deduplication_confirms_exact_equality() {
        let and = BooleanFunction::new(2, vec![0, 0, 0, 1]).unwrap();
        let or = BooleanFunction::new(2, vec![0, 1, 1, 1]).unwrap();
        let mut index = DedupIndex::new();
        assert_eq!(index.insert(and.clone()), DedupOutcome::New);
        assert_eq!(index.insert(and), DedupOutcome::Existing);
        assert_eq!(index.insert(or), DedupOutcome::New);
        assert_eq!(index.unique_functions(), 2);
    }
}
